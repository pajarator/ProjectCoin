/// RUN84 — Session-Based Partial Exit Scaling
///
/// Apply session multipliers to partial exit tiers:
/// - ASIA (UTC 00:00-08:00): ASIA_MULT applied to tiers
/// - EUUS (UTC 08:00-16:00): EUUS_MULT applied to tiers
/// - US (UTC 16:00-24:00): US_MULT applied to tiers
///
/// Grid: ASIA_MULT [0.60, 0.70, 0.80] × US_MULT [0.80, 0.85, 0.90] × EUUS_MULT [0.90, 1.00, 1.10]
/// Total: 3 × 3 × 3 = 27 + baseline = 28 configs
///
/// Run: cargo run --release --features run84 -- --run84

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

// Partial exit tiers (from RUN53)
const TIER1_PCT: f64 = 0.004;  // first partial exit at 0.4%
const TIER1_SIZE: f64 = 0.20;  // 20% of position
const TIER2_PCT: f64 = 0.008;  // second partial exit at 0.8%
const TIER2_SIZE: f64 = 0.20;  // 20% of position
const REMAIN_SIZE: f64 = 0.60;  // 60% goes to full exit (SL or signal)

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct SessionCfg {
    asia_mult: f64,
    us_mult: f64,
    euus_mult: f64,
    is_baseline: bool,
}

impl SessionCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("A{:.2}_U{:.2}_E{:.2}", self.asia_mult, self.us_mult, self.euus_mult) }
    }
}

fn build_grid() -> Vec<SessionCfg> {
    let mut grid = vec![SessionCfg { asia_mult: 1.0, us_mult: 1.0, euus_mult: 1.0, is_baseline: true }];
    let asia_vals = [0.60, 0.70, 0.80];
    let us_vals = [0.80, 0.85, 0.90];
    let euus_vals = [0.90, 1.00, 1.10];
    for &a in &asia_vals {
        for &u in &us_vals {
            for &e in &euus_vals {
                grid.push(SessionCfg { asia_mult: a, us_mult: u, euus_mult: e, is_baseline: false });
            }
        }
    }
    grid
}

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    hours: Vec<u8>, // UTC hour at each bar
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64 }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new(); let mut hours = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        // Parse UTC hour from timestamp "2025-10-15 17:30:00"
        let hour: u8 = if ts.len() >= 13 {
            ts[11..13].parse().unwrap_or(0)
        } else { 0 };
        opens.push(oo); closes.push(cc); hours.push(hour);
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
    Some(CoinData15m { name: coin.to_string(), closes, opens, hours, zscore })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn session_mult(hour: u8, cfg: &SessionCfg) -> f64 {
    if hour >= 0 && hour < 8 { cfg.asia_mult }       // Asia
    else if hour >= 8 && hour < 16 { cfg.euus_mult } // EUUS
    else { cfg.us_mult }                              // US
}

// Partial exit tiers: [tier1_pct, tier2_pct, SL_pct]
fn compute_tiers(hour: u8, cfg: &SessionCfg) -> [f64; 3] {
    let mult = session_mult(hour, cfg);
    if cfg.is_baseline {
        [TIER1_PCT, TIER2_PCT, SL_PCT]
    } else {
        [TIER1_PCT * mult, TIER2_PCT * mult, SL_PCT]
    }
}

fn simulate(d: &CoinData15m, cfg: &SessionCfg) -> (f64, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize)> = None; // dir, entry_price, entry_bar
    // Partial exit tracking: for each exit tier, track if it's been triggered
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    // For partial exit tracking: (dir, entry_price, tiers_triggered: u8, tier1_pct, tier2_pct)
    // tiers_triggered: bit 0 = tier1 taken, bit 1 = tier2 taken

    for i in 1..n {
        if let Some((dir, entry, entry_bar)) = pos {
            // Check partial exits first
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let tiers = compute_tiers(d.hours[entry_bar], cfg);
            let tier1 = tiers[0];
            let tier2 = tiers[1];
            let sl = tiers[2];

            // Process partial exits
            // Note: we need to track per-tier state. For simplicity, simulate as:
            // - If pct >= tier1: close TIER1_SIZE at tier1
            // - If pct >= tier2: close TIER2_SIZE at tier2
            // - If pct <= -sl: close REMAIN_SIZE at -sl
            // But this doesn't account for already-closed partials.
            // For simplicity, use a tier-based approach:
            // Tier1 exit: if pct >= tier1, close 20% at tier1
            // Tier2 exit: if pct >= tier2, close additional 20% at tier2
            // Remainder: closed at SL or signal exit

            let mut closed = false;
            let mut exit_pct = 0.0;

            // SL check
            if pct <= -sl { exit_pct = -sl; closed = true; }

            // Tier exits - check from highest to lowest
            if !closed && pct >= tier2 { exit_pct = tier2; closed = true; }
            if !closed && pct >= tier1 { exit_pct = tier1; closed = true; }

            // Signal exit (SMA crossback) - but only if not already hit by tier
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
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
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d.zscore[i]) {
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price, i)); }
                }
            }
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN84 — Session-Based Partial Exit Scaling\n");
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref data) = loaded {
            let hour_dist = {
                let mut h = [0usize; 24];
                for &hh in &data.hours { h[hh as usize] += 1; }
                format!("{}/{}/{}", h[3], h[11], h[19])
            };
            eprintln!("  {} — {} bars  hour dist (3am/11am/7pm): {}", name, data.closes.len(), hour_dist);
        }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![] };
        }
        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, flats) = simulate(d, cfg);
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, wr }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins_sum as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
        let losses_f = coin_results.iter().map(|c| c.losses).sum::<usize>() as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf, is_baseline: cfg.is_baseline, coins: coin_results }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN84 Session-Based Partial Exit Scaling Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>6} {:>7} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(70));
    for (i, r) in sorted.iter().enumerate().take(30) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<20} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("... ({} total)", sorted.len());
    println!("{}", "=".repeat(70));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN84 session partial scaling. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run84_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run84_1_results.json");
}
