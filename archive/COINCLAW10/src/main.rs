#![allow(dead_code)]
mod config;
mod coordinator;
mod engine;
mod fetcher;
mod indicators;
mod state;
mod strategies;
mod tui;

use config::COINS;
use futures::future::join_all;
use state::SharedState;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let reset = std::env::args().any(|a| a == "--reset");
    if reset {
        let _ = std::fs::remove_file(config::STATE_FILE);
        let _ = std::fs::remove_file(config::LOG_FILE);
        eprintln!("State reset!");
    }

    // --close-all: close all open positions at entry price and exit
    if std::env::args().any(|a| a == "--close-all") {
        let mut state = SharedState::new();
        let mut closed = 0usize;
        for ci in 0..state.coins.len() {
            if let Some(ref pos) = state.coins[ci].pos.clone() {
                let tt = pos.trade_type.unwrap_or(state::TradeType::Regime);
                let tt_label = match tt {
                    state::TradeType::Scalp => " [SCALP]",
                    state::TradeType::Regime => "",
                };
                state.coins[ci].trades.push(state::TradeRecord {
                    pnl: 0.0,
                    reason: "CLOSE_ALL".to_string(),
                    dir: pos.dir.clone(),
                });
                let name = state.coins[ci].name;
                state.log(format!("CLOSE_ALL {}{} @ entry {} | flat exit", name, tt_label, state::fmt_price(pos.e)));
                state.coins[ci].pos = None;
                state.coins[ci].candles_held = 0;
                state.coins[ci].cooldown = 0;
                closed += 1;
            }
        }
        state.save_state();
        eprintln!("Closed {} positions. State saved.", closed);
        return;
    }

    let shared = Arc::new(RwLock::new(SharedState::new()));
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(20)
        .timeout(Duration::from_secs(15))
        .build()
        .expect("Failed to create HTTP client");

    // SIGINT handler
    let running = Arc::clone(&shared);
    ctrlc::set_handler(move || {
        eprintln!("\nSaving state and exiting...");
        // We can't async here, so just set flag
        // The main loop will save on next iteration
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            let r = Arc::clone(&running);
            handle.spawn(async move {
                let mut s = r.write().await;
                s.save_state();
                s.running = false;
            });
        }
        std::process::exit(0);
    })
    .expect("Failed to set SIGINT handler");

    // Spawn TUI task
    let tui_state = Arc::clone(&shared);
    let tui_handle = tokio::spawn(async move {
        run_tui(tui_state).await;
    });

    // Spawn trading loop
    let trade_state = Arc::clone(&shared);
    let trade_handle = tokio::spawn(async move {
        run_trading_loop(trade_state, client).await;
    });

    let _ = tokio::select! {
        r = tui_handle => r,
        r = trade_handle => r,
    };

    // Save on exit
    let s = shared.read().await;
    s.save_state();
}

async fn run_tui(shared: Arc<RwLock<SharedState>>) {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use ratatui::prelude::*;

    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen).expect("Failed to enter alt screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    loop {
        {
            let state = shared.read().await;
            if !state.running { break; }
            terminal
                .draw(|frame| {
                    tui::render(frame, &state);
                })
                .ok();
        }

        // Poll for key events with 200ms timeout
        if event::poll(Duration::from_millis(200)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    let mut state = shared.write().await;
                    state.save_state();
                    state.running = false;
                    break;
                }
            }
        }
    }

    disable_raw_mode().ok();
    crossterm::execute!(std::io::stdout(), LeaveAlternateScreen).ok();
}

async fn run_trading_loop(shared: Arc<RwLock<SharedState>>, client: reqwest::Client) {
    let mut last_15m_fetch = Instant::now() - Duration::from_secs(999);
    let mut last_1m_fetch = Instant::now() - Duration::from_secs(999);

    loop {
        {
            let state = shared.read().await;
            if !state.running { break; }
        }

        let now = Instant::now();

        // Phase 1: Fetch 15m candles (every 60s)
        if now.duration_since(last_15m_fetch).as_secs() >= config::FETCH_15M_INTERVAL_SECS {
            let futures: Vec<_> = COINS
                .iter()
                .map(|cfg| {
                    let client = client.clone();
                    let symbol = cfg.binance_symbol;
                    async move {
                        fetcher::fetch_klines(&client, symbol, "15m", config::FETCH_15M_LIMIT).await
                    }
                })
                .collect();

            let results = join_all(futures).await;

            let mut state = shared.write().await;
            for (idx, result) in results.into_iter().enumerate() {
                match result {
                    Ok(candles) => {
                        let ind = indicators::compute_15m_indicators(&candles);
                        state.coins[idx].candles_15m = candles;
                        state.coins[idx].ind_15m = ind;
                    }
                    Err(e) => {
                        state.log(format!("ERROR {}: {}", COINS[idx].name, e));
                    }
                }
            }

            // Phase 4: Compute market breadth
            let (breadth, bearish, total, mode, ctx) =
                coordinator::compute_breadth_and_context(&state);
            state.breadth = breadth;
            state.bearish_count = bearish;
            state.total_count = total;
            state.market_mode = mode;

            // Phase 5: Check exits then entries
            let n = state.coins.len();
            for ci in 0..n {
                engine::check_exit(&mut state, ci);
            }
            for ci in 0..n {
                engine::check_entry(&mut state, ci, mode, &ctx);
            }

            // Phase 6: Save state
            state.save_state();

            last_15m_fetch = Instant::now();
        }

        // Phase 2: Fetch 1m candles (every 15s)
        if now.duration_since(last_1m_fetch).as_secs() >= config::FETCH_1M_INTERVAL_SECS {
            let futures: Vec<_> = COINS
                .iter()
                .map(|cfg| {
                    let client = client.clone();
                    let symbol = cfg.binance_symbol;
                    async move {
                        fetcher::fetch_klines(&client, symbol, "1m", 50).await
                    }
                })
                .collect();

            let results = join_all(futures).await;

            let mut state = shared.write().await;
            for (idx, result) in results.into_iter().enumerate() {
                match result {
                    Ok(candles) => {
                        let ind = indicators::compute_1m_indicators(&candles);
                        state.coins[idx].candles_1m = candles;
                        state.coins[idx].ind_1m = ind;
                    }
                    Err(e) => {
                        state.log(format!("ERROR 1m {}: {}", COINS[idx].name, e));
                    }
                }
            }

            // Check scalp exits and entries
            let n = state.coins.len();
            for ci in 0..n {
                let cs = &state.coins[ci];
                if let Some(ref pos) = cs.pos {
                    if pos.trade_type == Some(crate::state::TradeType::Scalp) {
                        engine::check_exit(&mut state, ci);
                    }
                }
            }
            for ci in 0..n {
                let cs = &state.coins[ci];
                // Only enter scalps if no position at all
                if cs.pos.is_none() {
                    engine::check_scalp_entry(&mut state, ci);
                }
            }

            last_1m_fetch = Instant::now();
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
