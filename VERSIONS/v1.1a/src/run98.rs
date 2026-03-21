/// RUN98 — Intraday Max Drawdown Clip: Exit Positions When Unrealized Loss Exceeds Daily Threshold
///
/// Grid: DD_CAP [0.005, 0.0075, 0.010, 0.015] + baseline = 5 configs × 18 coins
///
/// Run: cargo run --release --features run98 -- --run98

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
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
struct DdClipCfg {
    dd_cap: f64,
    is_baseline: bool,
}

impl DdClipCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("DC{:.4}", self.dd_cap) }
    }
}

fn build_grid() -> Vec<DdClipCfg> {
    let mut grid = vec![DdClipCfg { dd_cap: 999.0, is_baseline: true }];
    for &dc in &[0.005, 0.0075, 0.010, 0.015] {
        grid.push(DdClipCfg { dd_cap: dc, is_baseline: false });
    }
    grid
}

struct CoinData {
    name: String,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    zscore: Vec<f64>,
    timestamps: Vec<i64>, // Unix timestamp
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
    dd_clip_exits: usize,
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
    dd_clip_exits: usize,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut highs = Vec::new(); let mut lows = Vec::new();
    let mut closes = Vec::new(); let mut timestamps = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let ts_str = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        let ts: i64 = ts_str.parse().unwrap_or(0);
        opens.push(oo); highs.push(hh); lows.push(ll); closes.push(cc); timestamps.push(ts);
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
    Some(CoinData { name: coin.to_string(), opens, highs, lows, closes, zscore, timestamps })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn is_new_day(ts: i64, prev_ts: i64) -> bool {
    // Check if ts and prev_ts are on different UTC days
    // 86400 seconds per day
    ts / 86400 != prev_ts / 86400
}

fn simulate(d: &CoinData, cfg: &DdClipCfg) -> CoinResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None; // dir, entry, entry_bar, z_at_entry
    let mut cooldown = 0usize;

    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut dd_clip_exits = 0usize;

    // For intraday DD tracking
    let mut daily_high = f64::NAN;
    let mut daily_low = f64::NAN;
    let mut prev_ts: i64 = 0;

    for i in 20..n {
        // Check for UTC midnight reset
        if i > 0 && is_new_day(d.timestamps[i], prev_ts) {
            daily_high = d.opens[i]; // Reset at new day's open
            daily_low = d.opens[i];
        }
        prev_ts = d.timestamps[i];

        // Update daily high/low
        if daily_high.is_nan() || d.highs[i] > daily_high { daily_high = d.highs[i]; }
        if daily_low.is_nan() || d.lows[i] < daily_low { daily_low = d.lows[i]; }

        if let Some((dir, entry, _entry_bar, _z_entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }
            // Intraday DD clip (only for longs since we track daily_high)
            if !closed && !cfg.is_baseline && dir == 1 {
                // Daily drawdown from high: (daily_high - current) / daily_high
                let intraday_dd = if daily_high > 0.0 { (daily_high - d.closes[i]) / daily_high } else { 0.0 };
                if intraday_dd >= cfg.dd_cap {
                    exit_pct = pct; closed = true; dd_clip_exits += 1;
                }
            }
            if !closed && !cfg.is_baseline && dir == -1 {
                // For shorts: drawdown from daily low
                let intraday_dd = if daily_low > 0.0 { (d.closes[i] - daily_low) / daily_low } else { 0.0 };
                if intraday_dd >= cfg.dd_cap {
                    exit_pct = pct; closed = true; dd_clip_exits += 1;
                }
            }
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }

            if closed {
                let net = bal * POSITION_SIZE * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                pos = None;
                cooldown = COOLDOWN;
                daily_high = f64::NAN;
                daily_low = f64::NAN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d.zscore[i]) {
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 {
                        pos = Some((dir, entry_price, i, d.zscore[i]));
                        // Reset daily tracking at entry
                        daily_high = entry_price;
                        daily_low = entry_price;
                    }
                }
            }
        }
    }

    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let avg_win = POSITION_SIZE * LEVERAGE * SL_PCT * wins as f64;
    let avg_loss = POSITION_SIZE * LEVERAGE * SL_PCT * losses as f64;
    let pf = if losses > 0 { avg_win / avg_loss } else { 0.0 };

    CoinResult { coin: d.name.clone(), pnl, trades: total_trades, wins, losses, flats, wr, pf, dd_clip_exits }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN98 — Intraday Max Drawdown Clip Grid Search\n");
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

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, dd_clip_exits: 0, is_baseline: cfg.is_baseline, coins: vec![] };
        }
        let coin_results: Vec<CoinResult> = data.iter().map(|d| simulate(d, cfg)).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let total_dd_clips: usize = coin_results.iter().map(|c| c.dd_clip_exits).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_win_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_wins as f64);
        let avg_loss_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_losses as f64);
        let pf = if total_losses > 0 { avg_win_f / avg_loss_f } else { 0.0 };

        eprintln!("  {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  dd_clips={}",
            cfg.label(), total_pnl, portfolio_wr, total_trades, total_dd_clips);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, dd_clip_exits: total_dd_clips, is_baseline: cfg.is_baseline, coins: coin_results }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN98 Intraday Max Drawdown Clip Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}  DDClips=0", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<15} {:>8} {:>8} {:>6} {:>7} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "DDClips");
    println!("{}", "-".repeat(60));
    for (i, r) in sorted.iter().enumerate().take(10) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<15} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.dd_clip_exits);
    }
    println!("{}", "=".repeat(60));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN98 intraday dd clip. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run98_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run98_1_results.json");
}
