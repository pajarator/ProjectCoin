/// RUN130 — Negative Day Revenue Filter: Rolling N-Day Loss Frequency as Market Stress Signal
///
/// Grid: NEGDAY_WINDOW [3, 5, 7] × NEGDAY_SUPPRESS [0.80, 1.0]
/// 6 configs × 18 coins = portfolio-level simulation (sequential time)
/// NOTE: This is a portfolio-level backtest — bars processed in time order with shared state
///
/// Run: cargo run --release --features run130 -- --run130

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use chrono::NaiveDateTime;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct NegdayCfg {
    window: usize,
    suppress: f64,
    is_baseline: bool,
}

impl NegdayCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("NW{}_NS{}", self.window, self.suppress) }
    }
}

fn build_grid() -> Vec<NegdayCfg> {
    let mut grid = vec![NegdayCfg { window: 0, suppress: 0.0, is_baseline: true }];
    for &w in &[3usize, 5, 7] {
        for &s in &[0.80f64, 1.0f64] {
            grid.push(NegdayCfg { window: w, suppress: s, is_baseline: false });
        }
    }
    grid
}

struct CoinData {
    name: String,
    opens: Vec<f64>,
    closes: Vec<f64>,
    zscore: Vec<f64>,
    timestamps: Vec<i64>, // Unix timestamp
}

#[derive(Clone)]
struct PortfolioState {
    daily_pnl: Vec<f64>,       // Portfolio P&L per UTC day (rolling window)
    current_day_pnl: f64,      // Current UTC day's accumulated P&L
    current_day_idx: isize,    // Index of current day in daily_pnl
    suppress_count: usize,
    total_signals: usize,
}

impl PortfolioState {
    fn new(window: usize) -> Self {
        PortfolioState {
            daily_pnl: vec![0.0; window],
            current_day_pnl: 0.0,
            current_day_idx: -1,
            suppress_count: 0,
            total_signals: 0,
        }
    }

    fn loss_frequency(&self) -> f64 {
        if self.daily_pnl.is_empty() { return 0.0; }
        let negatives = self.daily_pnl.iter().filter(|&&p| p < 0.0).count();
        negatives as f64 / self.daily_pnl.len() as f64
    }

    fn end_day(&mut self) {
        if self.current_day_idx >= 0 {
            let idx = self.current_day_idx as usize % self.daily_pnl.len();
            self.daily_pnl[idx] = self.current_day_pnl;
        }
        self.current_day_pnl = 0.0;
        self.current_day_idx += 1;
    }
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    pnl: f64,
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    wr: f64,
    pf: f64,
}
#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    pf: f64,
    is_baseline: bool,
    suppress_rate: f64,
    coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn get_utc_day(ts: i64) -> i64 {
    // Convert Unix timestamp (seconds) to UTC day number (days since epoch)
    ts / 86400
}

fn load_15m(coin: &str) -> Option<CoinData> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut closes = Vec::new();
    let mut timestamps = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let ts_str = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        // Parse datetime like "2025-10-15 17:30:00" to Unix timestamp
        let ts = match NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S") {
            Ok(dt) => dt.and_utc().timestamp(),
            Err(_) => continue,
        };
        timestamps.push(ts);
        opens.push(oo);
        closes.push(cc);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();

    let mut zscore_arr = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>() / 20.0;
        let std = (window.iter().map(|x| (x-mean).powi(2)).sum::<f64>() / 20.0).sqrt();
        zscore_arr[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }

    Some(CoinData { name: coin.to_string(), opens, closes, zscore: zscore_arr, timestamps })
}

/// Simulate one config across all 18 coins as a PORTFOLIO.
/// Time flows sequentially; portfolio state is shared across coins.
/// Returns (total_pnl, wins, losses, flats, suppress_rate)
fn simulate_portfolio(coins_data: &[CoinData], cfg: &NegdayCfg) -> (f64, usize, usize, usize, f64, Vec<CoinResult>) {
    let n_bars = coins_data[0].closes.len();
    let n_coins = coins_data.len();

    // Per-coin position state: (dir, entry_price, entry_bar)
    let mut positions: Vec<Option<(i8, f64, usize)>> = vec![None; n_coins];
    let mut cooldowns: Vec<usize> = vec![0usize; n_coins];
    let mut bal = INITIAL_BAL;

    // Per-coin realized P&L tracking for daily aggregation
    let mut coin_daily_pnl: Vec<f64> = vec![0.0; n_coins];
    let mut last_day: Vec<i64> = vec![-1; n_coins];

    // Portfolio state (shared)
    let mut port_state = PortfolioState::new(cfg.window.max(1));

    let mut total_wins = 0usize;
    let mut total_losses = 0usize;
    let mut total_flats = 0usize;

    for i in 100..n_bars {
        let current_day = get_utc_day(coins_data[0].timestamps[i]);

        // Process each coin
        for c in 0..n_coins {
            let d = &coins_data[c];

            // Update daily P&L tracking
            let day = get_utc_day(d.timestamps[i]);
            if last_day[c] != day && last_day[c] != -1 {
                // New day: close out any open position's daily P&L contribution
                coin_daily_pnl[c] = 0.0;
            }
            last_day[c] = day;

            // Countdown cooldown
            cooldowns[c] = cooldowns[c].saturating_sub(1);

            // Handle open position
            if let Some((dir, entry, entry_bar)) = positions[c].as_mut() {
                let pct = if *dir == 1 { (d.closes[i] - *entry) / *entry } else { (*entry - d.closes[i]) / *entry };
                let mut closed = false;
                let mut exit_pct = 0.0;

                if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
                if !closed {
                    let new_dir = regime_signal(d.zscore[i]);
                    if new_dir.is_some() && new_dir != Some(*dir) { exit_pct = pct; closed = true; }
                }
                if !closed && i >= n_bars - 1 { exit_pct = pct; closed = true; }
                if !closed && i >= *entry_bar + MIN_HOLD_BARS {
                    if (*dir == 1 && d.zscore[i] >= 0.0) || (*dir == -1 && d.zscore[i] <= 0.0) {
                        exit_pct = pct; closed = true;
                    }
                }

                if closed {
                    let net = bal * POSITION_SIZE * LEVERAGE * exit_pct;
                    bal += net;
                    coin_daily_pnl[c] += net;
                    if net > 1e-10 { total_wins += 1; }
                    else if net < -1e-10 { total_losses += 1; }
                    else { total_flats += 1; }
                    positions[c] = None;
                    cooldowns[c] = COOLDOWN;
                }
            } else if cooldowns[c] == 0 {
                // Entry decision
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    port_state.total_signals += 1;

                    let suppressed = if cfg.is_baseline {
                        false
                    } else {
                        port_state.loss_frequency() >= cfg.suppress
                    };

                    if suppressed {
                        port_state.suppress_count += 1;
                    } else {
                        // Enter position on next bar's open
                        if i + 1 < n_bars {
                            let entry_price = d.opens[i + 1];
                            if entry_price > 0.0 {
                                positions[c] = Some((dir, entry_price, i));
                            }
                        }
                    }
                }
            }
        }

        // End-of-bar: update portfolio current day P&L
        let day_port_pnl: f64 = coin_daily_pnl.iter().sum();
        port_state.current_day_pnl += day_port_pnl;

        // Check for day boundary (compare across coins using first coin's timestamps)
        if i + 1 < n_bars {
            let next_day = get_utc_day(coins_data[0].timestamps[i + 1]);
            if next_day != current_day {
                // Day boundary: commit current day P&L to rolling history
                port_state.end_day();
                // Reset coin daily P&L for new day
                for cp in 0..n_coins { coin_daily_pnl[cp] = 0.0; }
            }
        } else {
            // Last bar: commit final day
            port_state.end_day();
        }
    }

    let _total_trades = total_wins + total_losses + total_flats;
    let suppress_rate = if port_state.total_signals > 0 {
        port_state.suppress_count as f64 / port_state.total_signals as f64 * 100.0
    } else { 0.0 };

    // Per-coin results (approximate — shares portfolio state so not perfectly accurate)
    let coin_results: Vec<CoinResult> = (0..n_coins).map(|c| {
        CoinResult {
            coin: coins_data[c].name.clone(),
            pnl: 0.0, // Portfolio-level; individual PnL not easily attributable
            trades: 0, wins: 0, losses: 0, flats: 0, wr: 0.0, pf: 0.0,
        }
    }).collect();

    (bal - INITIAL_BAL, total_wins, total_losses, total_flats, suppress_rate, coin_results)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN130 — Negative Day Revenue Filter Grid Search\n");
    eprintln!("  Loading {} coins...", N_COINS);

    let mut raw_data: Vec<Option<CoinData>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        else { eprintln!("  {} — FAILED TO LOAD", name); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) {
        eprintln!("Missing data for some coins!");
        return;
    }
    if shutdown.load(Ordering::SeqCst) { return; }
    let data: Vec<CoinData> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let grid = build_grid();
    eprintln!("\nGrid: {} configs (portfolio-level simulation)", grid.len());
    eprintln!("NOTE: Each config processes all 18 coins in time-order with shared portfolio state");

    let done = AtomicUsize::new(0);
    let total_sims = grid.len();

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0,
                total_trades: 0, wins: 0, losses: 0, flats: 0, pf: 0.0,
                is_baseline: cfg.is_baseline, suppress_rate: 0.0, coins: vec![]
            };
        }

        let (total_pnl, wins, losses, flats, suppress_rate, coins) = simulate_portfolio(&data, cfg);
        let total_trades = wins + losses + flats;
        let portfolio_wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_win_f = POSITION_SIZE * LEVERAGE * SL_PCT * (wins as f64);
        let avg_loss_f = POSITION_SIZE * LEVERAGE * SL_PCT * (losses as f64);
        let pf = if losses > 0 { avg_win_f / avg_loss_f } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  suppress={:.1}%",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, suppress_rate);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades,
            wins, losses, flats, pf, is_baseline: cfg.is_baseline,
            suppress_rate, coins
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN130 Negative Day Revenue Filter Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<12} {:>8} {:>8} {:>6} {:>7} {:>10}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "SuppressRate");
    println!("{}", "-".repeat(60));
    for (i, r) in sorted.iter().enumerate().take(10) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<12} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>9.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.suppress_rate);
    }
    println!("{}", "=".repeat(60));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN130 Negative Day Revenue filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run130_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run130_1_results.json");
}
