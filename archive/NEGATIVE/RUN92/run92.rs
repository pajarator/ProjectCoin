/// RUN92 — Exit Reason Weighted Learning
///
/// Track per-coin historical exit quality, suppress entries when expected_exit_score < threshold.
///
/// Grid: WEIGHT_WINDOW [30, 50, 100] × MIN_SAMPLES [15, 20] × MIN_SCORE [0.50, 0.60, 0.70]
/// Total: 3 × 2 × 3 = 18 + baseline = 19 configs
///
/// Run: cargo run --release --features run92 -- --run92

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

#[derive(Clone, Debug)]
struct WtCfg {
    window: usize,
    min_samples: usize,
    min_score: f64,
    is_baseline: bool,
}

impl WtCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("W{}_MS{}_SC{:.2}", self.window, self.min_samples, self.min_score) }
    }
}

fn build_grid() -> Vec<WtCfg> {
    let mut grid = vec![WtCfg { window: 0, min_samples: 0, min_score: 0.0, is_baseline: true }];
    let ws = [30usize, 50, 100];
    let mss = [15usize, 20];
    let scs = [0.50, 0.60, 0.70];
    for &w in &ws {
        for &ms in &mss {
            for &sc in &scs {
                grid.push(WtCfg { window: w, min_samples: ms, min_score: sc, is_baseline: false });
            }
        }
    }
    grid
}

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, suppressed: usize }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, suppressed_entries: usize,
    coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); closes.push(cc);
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
    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData15m, cfg: &WtCfg) -> (f64, usize, usize, usize, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut suppressed = 0usize;

    // Rolling exit PnL history for this coin
    let mut exit_history: Vec<f64> = Vec::with_capacity(cfg.window.max(1));
    let mut suppressed_total = 0usize;

    for i in 1..n {
        // Compute expected exit score from historical exits
        let exit_score = if cfg.is_baseline || cfg.window == 0 || exit_history.len() < cfg.min_samples {
            1.0 // baseline or insufficient data: allow all
        } else {
            // Use trailing window of exits
            let start = exit_history.len().saturating_sub(cfg.window);
            let window_exits = &exit_history[start..];
            if window_exits.is_empty() {
                1.0
            } else {
                // Score = fraction of non-negative exits (wins/flats are "good")
                let good: usize = window_exits.iter().map(|&e| if e >= 0.0 { 1 } else { 0 }).sum();
                good as f64 / window_exits.len() as f64
            }
        };

        if let Some((dir, entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
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
                exit_history.push(exit_pct);
                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            // Check exit score gating
            if exit_score < cfg.min_score {
                suppressed_total += 1;
            } else {
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                    }
                }
            }
        }
    }

    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    (pnl, total_trades, wins, losses, suppressed_total, exit_history.len(), flats)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN92 — Exit Reason Weighted Learning\n");
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

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins = {} simulations", grid.len(), N_COINS, grid.len() * N_COINS);

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, suppressed_entries: 0, coins: vec![]
            };
        }
        let coin_results: Vec<CoinResult> = (0..N_COINS).map(|c| {
            let (pnl, trades, wins, losses, suppressed, _, _) = simulate(&coin_data[c], cfg);
            CoinResult {
                coin: coin_data[c].name.clone(),
                pnl,
                trades,
                wins,
                losses,
                wr: if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 },
                suppressed,
            }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = total_wins as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
        let losses_f = total_losses as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };
        let suppressed_entries: usize = coin_results.iter().map(|c| c.suppressed).sum();

        eprintln!("  {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  suppressed={}",
            cfg.label(), total_pnl, portfolio_wr, total_trades, suppressed_entries);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf,
            is_baseline: cfg.is_baseline, suppressed_entries, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN92 Exit Reason Weighted Learning Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}",
        baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<25} {:>8} {:>8} {:>6} {:>7} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(70));
    for (i, r) in sorted.iter().enumerate().take(20) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<25} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("{}", "=".repeat(70));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN92 exit-weighted. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run92_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run92_1_results.json");
}
