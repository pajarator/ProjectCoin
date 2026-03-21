/// RUN66 — Exit Priority Reordering
///
/// Grid: 4 priority orders × 2 direction modes
/// Priority orders:
///   A: SL → Z0 → SMA → Timeout  (current baseline)
///   B: SL → SMA → Z0 → Timeout
///   C: SL → Z0 → Timeout → SMA
///   D: SL → SMA → Timeout → Z0
///
/// Run: cargo run --release --features run66 -- --run66

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const COOLDOWN: usize = 2;
const BASE_POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum ExitPriority {
    A, // SL → Z0 → SMA → Timeout (baseline)
    B, // SL → SMA → Z0 → Timeout
    C, // SL → Z0 → Timeout → SMA
    D, // SL → SMA → Timeout → Z0
}

impl ExitPriority {
    fn label(&self) -> String {
        match self {
            ExitPriority::A => "A_Z0_SMA".to_string(),
            ExitPriority::B => "B_SMA_Z0".to_string(),
            ExitPriority::C => "C_Z0_TMO".to_string(),
            ExitPriority::D => "D_SMA_TMO".to_string(),
        }
    }
}

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    zscore: Vec<f64>,
    sma5: Vec<f64>,
    sma20: Vec<f64>,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    pf: f64,
    is_baseline: bool,
    z0_exits: usize,
    sma_exits: usize,
    sl_exits: usize,
    timeout_exits: usize,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut closes = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() || hh.is_nan() || ll.is_nan() { continue; }
        opens.push(oo); closes.push(cc); highs.push(hh); lows.push(ll);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();

    // Z-score
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>() / 20.0;
        let std = (window.iter().map(|x| (x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }

    // SMA5
    let mut sma5 = vec![f64::NAN; n];
    for i in 4..n {
        sma5[i] = closes[i+1-5..=i].iter().sum::<f64>() / 5.0;
    }

    // SMA20
    let mut sma20 = vec![f64::NAN; n];
    for i in 19..n {
        sma20[i] = closes[i+1-20..=i].iter().sum::<f64>() / 20.0;
    }

    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, zscore, sma5, sma20 })
}

fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Check if price crosses SMA20 in the direction's exit direction
// LONG exit: price crosses below SMA20 (from above)
// SHORT exit: price crosses above SMA20 (from below)
fn sma_exit_signal(d: &CoinData15m, i: usize, dir: i8) -> bool {
    if i < 1 { return false; }
    let sma = d.sma20[i];
    let prev_sma = d.sma20[i-1];
    let price = d.closes[i];
    let prev_price = d.closes[i-1];
    if sma.is_nan() || prev_sma.is_nan() { return false; }

    if dir == 1 {
        // LONG exit: price crosses below SMA20
        prev_price > prev_sma && price < sma
    } else if dir == -1 {
        // SHORT exit: price crosses above SMA20
        prev_price < prev_sma && price > sma
    } else {
        false
    }
}

// Z0 exit: z-score crosses back toward 0.5 threshold
fn z0_exit_signal(d: &CoinData15m, i: usize, dir: i8) -> bool {
    let z = d.zscore[i];
    if z.is_nan() { return false; }
    if dir == 1 && z > 0.5 { return true; }
    if dir == -1 && z < -0.5 { return true; }
    false
}

fn simulate(d: &CoinData15m, priority: ExitPriority) -> (f64, usize, usize, usize, usize, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize)> = None; // dir, entry_price, entry_bar
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut z0_exits = 0usize;
    let mut sma_exits = 0usize;
    let mut sl_exits = 0usize;
    let mut timeout_exits = 0usize;

    for i in 1..n {
        if let Some((dir, entry, entry_bar)) = pos {
            let held = i - entry_bar;
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;
            let mut exit_reason = "";

            // SL check (always first)
            if pct <= -SL_PCT {
                exit_pct = -SL_PCT;
                exit_reason = "SL";
                closed = true;
            }

            // After MIN_HOLD=2, check other exits based on priority
            if !closed && held >= 2 {
                let z0 = z0_exit_signal(d, i, dir);
                let sma = sma_exit_signal(d, i, dir);

                match priority {
                    ExitPriority::A => {
                        // SL → Z0 → SMA → Timeout
                        if z0 { exit_pct = pct; exit_reason = "Z0"; closed = true; }
                        else if sma { exit_pct = pct; exit_reason = "SMA"; closed = true; }
                    }
                    ExitPriority::B => {
                        // SL → SMA → Z0 → Timeout
                        if sma { exit_pct = pct; exit_reason = "SMA"; closed = true; }
                        else if z0 { exit_pct = pct; exit_reason = "Z0"; closed = true; }
                    }
                    ExitPriority::C => {
                        // SL → Z0 → Timeout → SMA
                        if z0 { exit_pct = pct; exit_reason = "Z0"; closed = true; }
                    }
                    ExitPriority::D => {
                        // SL → SMA → Timeout → Z0
                        if sma { exit_pct = pct; exit_reason = "SMA"; closed = true; }
                    }
                }

                // Timeout exit if not closed yet
                if !closed {
                    exit_pct = pct;
                    exit_reason = "TMO";
                    closed = true;
                }
            }

            if closed {
                let net = (bal * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }

                match exit_reason {
                    "Z0" => z0_exits += 1,
                    "SMA" => sma_exits += 1,
                    "SL" => sl_exits += 1,
                    "TMO" => timeout_exits += 1,
                    _ => {}
                }

                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d, i) {
                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price, i+1)); }
                }
            }
        }
    }

    let _total = wins + losses + flats;
    (bal - INITIAL_BAL, wins, losses, flats, z0_exits, sma_exits, sl_exits, timeout_exits)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN66 — Exit Priority Reordering\n");
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref data) = loaded { eprintln!("  {} — {} bars", name, data.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let grid = vec![
        ExitPriority::A, // baseline (current)
        ExitPriority::B,
        ExitPriority::C,
        ExitPriority::D,
    ];

    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|&priority| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: priority.label(),
                total_pnl: 0.0,
                portfolio_wr: 0.0,
                total_trades: 0,
                pf: 0.0,
                is_baseline: priority == ExitPriority::A,
                z0_exits: 0,
                sma_exits: 0,
                sl_exits: 0,
                timeout_exits: 0,
            };
        }
        let mut total_pnl = 0.0;
        let mut wins_sum = 0usize;
        let mut losses_sum = 0usize;
        let mut flats_sum = 0usize;
        let mut z0_sum = 0usize;
        let mut sma_sum = 0usize;
        let mut sl_sum = 0usize;
        let mut tmo_sum = 0usize;

        for d in &coin_data {
            let (pnl, wins, losses, flats, z0, sma, sl, tmo) = simulate(d, priority);
            total_pnl += pnl;
            wins_sum += wins;
            losses_sum += losses;
            flats_sum += flats;
            z0_sum += z0;
            sma_sum += sma;
            sl_sum += sl;
            tmo_sum += tmo;
        }

        let total_trades = wins_sum + losses_sum + flats_sum;
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins_sum as f64 * 0.003 * BASE_POSITION_SIZE * LEVERAGE;
        let pf = if losses_sum > 0 { gross / (losses_sum as f64 * 0.003 * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}  Z0={} SMA={} SL={} TMO={}",
            d, total_cfgs, priority.label(), total_pnl, portfolio_wr, total_trades, z0_sum, sma_sum, sl_sum, tmo_sum);

        ConfigResult {
            label: priority.label(),
            total_pnl,
            portfolio_wr,
            total_trades,
            pf,
            is_baseline: priority == ExitPriority::A,
            z0_exits: z0_sum,
            sma_exits: sma_sum,
            sl_exits: sl_sum,
            timeout_exits: tmo_sum,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN66 Exit Priority Reordering Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<12} {:>8} {:>8} {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "Z0", "SMA", "SL", "TMO");
    println!("{}", "-".repeat(82));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<12} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.z0_exits, r.sma_exits, r.sl_exits, r.timeout_exits);
    }
    println!("{}", "=".repeat(82));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN66 exit priority reordering. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run66_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run66_1_results.json");
}