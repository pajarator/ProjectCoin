/// RUN102 — TWAP Entry Execution: Accumulate Positions Over Time to Reduce Entry Timing Risk
///
/// Grid: TWAP_BARS [2, 3, 4] × TWAP_SL_TRIGGER [0.003, 0.005, 0.007]
/// 9 configs × 18 coins = 162 simulations (parallel per coin)
///
/// Run: cargo run --release --features run102 -- --run102

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

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

const Z_BREADTH_LONG: usize = 20;
const Z_BREADTH_SHORT: usize = 50;

#[derive(Clone, Copy, PartialEq, Debug)]
struct TwapCfg {
    bars: usize,
    sl_trigger: f64,
    is_baseline: bool,
}

impl TwapCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("TB{}_ST{:.3}", self.bars, self.sl_trigger) }
    }
}

fn build_grid() -> Vec<TwapCfg> {
    let mut grid = vec![TwapCfg { bars: 0, sl_trigger: 0.0, is_baseline: true }];
    for &b in &[2usize, 3, 4] {
        for &t in &[0.003, 0.005, 0.007] {
            grid.push(TwapCfg { bars: b, sl_trigger: t, is_baseline: false });
        }
    }
    grid
}

struct CoinData {
    name: String,
    opens: Vec<f64>,
    closes: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    zscore: Vec<f64>,
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
    pf: f64,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
    let mut highs = Vec::new(); let mut lows = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); highs.push(hh); lows.push(ll); closes.push(cc);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>()/20.0;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }
    Some(CoinData { name: coin.to_string(), opens, closes, highs, lows, zscore })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData, cfg: &TwapCfg) -> CoinResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None; // dir, avg_entry, entry_bar, z_at_entry
    let mut cooldown = 0usize;

    // TWAP state
    let mut twap_active = false;
    let mut twap_bars_remaining = 0usize;
    let mut twap_dir = 0i8;
    let mut twap_start_price = 0.0f64;
    let mut twap_accumulated = 0.0f64;
    let mut twap_target_size = 0.0f64;

    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;

    for i in 20..n {
        // TWAP accumulation: add partial fills
        if twap_active {
            if cfg.is_baseline || twap_bars_remaining == 0 {
                // Baseline: enter immediately; or bars=0 means immediate entry
                pos = Some((twap_dir, d.opens[i + 1], i, d.zscore[i]));
                twap_active = false;
                twap_bars_remaining = 0;
            } else {
                // Typical price for this bar
                let vwap = (d.highs[i] + d.lows[i] + d.closes[i]) / 3.0;
                // Check SL trigger: if price moved significantly against us
                let move_pct = if twap_dir == 1 {
                    (vwap - twap_start_price) / twap_start_price
                } else {
                    (twap_start_price - vwap) / twap_start_price
                };
                if move_pct <= -cfg.sl_trigger {
                    // Cancel TWAP — skip entry
                    twap_active = false;
                    twap_bars_remaining = 0;
                } else {
                    twap_bars_remaining -= 1;
                    if twap_bars_remaining == 0 {
                        // TWAP complete — open full position
                        pos = Some((twap_dir, twap_start_price, i, d.zscore[i]));
                        twap_active = false;
                    }
                }
            }
        }

        if let Some((dir, entry, entry_bar, _z_entry)) = pos.as_mut() {
            let pct = if *dir == 1 { (d.closes[i]-*entry)/ *entry } else { (*entry-d.closes[i])/ *entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(*dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }
            if !closed && i >= *entry_bar + MIN_HOLD_BARS {
                if (*dir == 1 && d.zscore[i] >= 0.0) || (*dir == -1 && d.zscore[i] <= 0.0) {
                    exit_pct = pct; closed = true;
                }
            }

            if closed {
                let net = bal * POSITION_SIZE * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else if !twap_active {
            if let Some(dir) = regime_signal(d.zscore[i]) {
                // Start TWAP accumulation
                twap_active = true;
                twap_dir = dir;
                twap_start_price = d.opens[i + 1]; // entry at next bar's open
                twap_bars_remaining = cfg.bars;
                twap_accumulated = 0.0;
                twap_target_size = POSITION_SIZE;
                twap_dir = dir;
            }
        }
    }

    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let avg_win_f = POSITION_SIZE * LEVERAGE * SL_PCT * wins as f64;
    let avg_loss_f = POSITION_SIZE * LEVERAGE * SL_PCT * losses as f64;
    let pf = if losses > 0 { avg_win_f / avg_loss_f } else { 0.0 };

    CoinResult { coin: d.name.clone(), pnl, trades: total_trades, wins, losses, flats, wr, pf }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN102 — TWAP Entry Execution Grid Search\n");
    let mut raw_data: Vec<Option<CoinData>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let data: Vec<CoinData> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins = {} simulations", grid.len(), N_COINS, grid.len() * N_COINS);

    let done = AtomicUsize::new(0);
    let total_sims = grid.len() * N_COINS;

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![] };
        }
        let coin_results: Vec<CoinResult> = data.iter().map(|d| simulate(d, cfg)).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_win_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_wins as f64);
        let avg_loss_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_losses as f64);
        let pf = if total_losses > 0 { avg_win_f / avg_loss_f } else { 0.0 };

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, is_baseline: cfg.is_baseline, coins: coin_results }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN102 TWAP Entry Execution Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>6} {:>7}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(55));
    for (i, r) in sorted.iter().enumerate().take(15) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<20} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(55));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN102 TWAP entry. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run102_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run102_1_results.json");
}
