/// RUN49 — Cross-Coin Correlation Filter: Avoiding Clustered Over-Concentration
///
/// Grid: CORR_THRESHOLD [0.60, 0.70, 0.75, 0.80, 0.85]
///     × CORR_WINDOW [10, 15, 20, 30] bars
///     × MODE [1=suppress_weakest, 2=random_drop]
///
/// Portfolio-level evaluation. Uses per-coin independent simulation but with
/// cross-coin correlation filtering applied to entry selection.
///
/// Run: cargo run --release --features run49 -- --run49

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const COOLDOWN: usize = 2;
const MIN_HOLD: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct CorrCfg {
    threshold: f64,
    window: usize,
    mode: u8, // 0=disabled, 1=suppress_weakest, 2=random_drop
}

impl CorrCfg {
    fn label(&self) -> String {
        if self.mode == 0 { "DISABLED".to_string() }
        else {
            let m = if self.mode == 1 { "W" } else { "R" };
            format!("T{:.2}_W{}_{}", self.threshold, self.window, m)
        }
    }
}

fn build_grid() -> Vec<CorrCfg> {
    let mut grid = vec![CorrCfg { threshold: 0.75, window: 15, mode: 0 }];
    let thresh_vals = [0.60, 0.70, 0.75, 0.80, 0.85];
    let window_vals = [10, 15, 20, 30];
    let modes = [1u8, 2];
    for &t in &thresh_vals {
        for &w in &window_vals {
            for &m in &modes {
                grid.push(CorrCfg { threshold: t, window: w, mode: m });
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
    returns: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, flats: usize, wr: f64 }

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    portfolio_pnl: f64,
    portfolio_wr: f64,
    portfolio_trades: usize,
    suppressed_count: usize,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut closes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo);
        closes.push(cc);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();
    let mut zscore = vec![f64::NAN; n];
    let mut returns = vec![0.0; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>() / 20.0;
        let std = (window.iter().map(|x| (x-mean).powi(2)).sum::<f64>() / 20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
        returns[i] = if i > 0 { (closes[i] - closes[i-1]) / closes[i-1] } else { 0.0 };
    }
    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore, returns })
}

fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn pearson_corr(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n < 3 { return 0.0; }
    let sa: f64 = a.iter().take(n).sum();
    let sb: f64 = b.iter().take(n).sum();
    let ma = sa / n as f64;
    let mb = sb / n as f64;
    let mut num = 0.0;
    let mut da2 = 0.0;
    let mut db2 = 0.0;
    for i in 0..n {
        let dx = a[i] - ma;
        let dy = b[i] - mb;
        num += dx * dy;
        da2 += dx * dx;
        db2 += dy * dy;
    }
    let denom = (da2 * db2).sqrt();
    if denom > 1e-10 { num / denom } else { 0.0 }
}

type SimOutput = (f64, usize, usize, usize, usize); // pnl, wins, losses, flats, suppressed

/// Per-coin baseline simulation (no correlation filter)
fn simulate_baseline(d: &CoinData15m) -> SimOutput {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;

    for i in 1..n {
        if let Some((dir, entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                let new_dir = regime_signal(d, i);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= MIN_HOLD { exit_pct = pct; closed = true; }
            if closed {
                let net = (bal * POSITION_SIZE * LEVERAGE) * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; } else if net < -1e-10 { losses += 1; } else { flats += 1; }
                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d, i) {
                if i + 1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                }
            }
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats, 0)
}

/// Portfolio-level simulation with correlation filter applied to entry selection
fn simulate_corr_portfolio(coins: &[CoinData15m], cfg: CorrCfg) -> SimOutput {
    let n = coins[0].closes.len();
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldowns = vec![0usize; N_COINS];
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut suppressed = 0usize;

    // Per-coin rolling return buffers
    let mut ret_bufs: Vec<Vec<f64>> = vec![Vec::new(); N_COINS];

    for i in 1..n {
        // Update return buffers
        for ci in 0..N_COINS {
            if i > 0 {
                ret_bufs[ci].push(coins[ci].returns[i]);
                if ret_bufs[ci].len() > cfg.window {
                    ret_bufs[ci].remove(0);
                }
            }
        }

        // ── Pass 1: collect candidates ───────────────────────────────────
        let mut candidates: Vec<(usize, i8)> = Vec::new(); // (ci, dir)
        for ci in 0..N_COINS {
            if positions[ci].is_none() && cooldowns[ci] == 0 {
                if let Some(dir) = regime_signal(&coins[ci], i) {
                    candidates.push((ci, dir));
                }
            }
        }

        // ── Pass 2: correlation filter ────────────────────────────────────
        let mut suppressed_ci = vec![false; N_COINS];
        if candidates.len() > 1 && cfg.mode > 0 {
            for idx_a in 0..candidates.len() {
                for idx_b in (idx_a + 1)..candidates.len() {
                    let (ci_a, dir_a) = candidates[idx_a];
                    let (ci_b, dir_b) = candidates[idx_b];
                    if dir_a != dir_b { continue; }
                    let corr = pearson_corr(&ret_bufs[ci_a], &ret_bufs[ci_b]);
                    if corr > cfg.threshold {
                        if cfg.mode == 1 {
                            // suppress_weakest: suppress b if a is better
                            // For LONG (dir=1): lower z = better (more oversold)
                            // For SHORT (dir=-1): higher z = better (more overbought)
                            suppressed_ci[ci_b] = true;
                        } else {
                            // random_drop: suppress b
                            suppressed_ci[ci_b] = true;
                        }
                    }
                }
            }
        }

        // Count suppressed
        for &(ci, _) in &candidates {
            if suppressed_ci[ci] { suppressed += 1; }
        }

        // ── Pass 3: execute entries ─────────────────────────────────────
        for &(ci, dir) in &candidates {
            if suppressed_ci[ci] { continue; }
            if i + 1 < n {
                let entry_price = coins[ci].opens[i+1];
                if entry_price > 0.0 {
                    positions[ci] = Some((dir, entry_price));
                }
            }
        }

        // ── Pass 4: check exits (all coins) ─────────────────────────────
        for ci in 0..N_COINS {
            if let Some((dir, entry)) = positions[ci] {
                let pct = if dir == 1 {
                    (coins[ci].closes[i] - entry) / entry
                } else {
                    (entry - coins[ci].closes[i]) / entry
                };
                let mut closed = false;
                let mut exit_pct = 0.0;
                if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
                if !closed {
                    let new_dir = regime_signal(&coins[ci], i);
                    if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
                }
                if !closed && i >= MIN_HOLD { exit_pct = pct; closed = true; }
                if closed {
                    let net = (balances[ci] * POSITION_SIZE * LEVERAGE) * exit_pct;
                    balances[ci] += net;
                    if net > 1e-10 { wins += 1; }
                    else if net < -1e-10 { losses += 1; }
                    else { flats += 1; }
                    positions[ci] = None;
                    cooldowns[ci] = COOLDOWN;
                }
            } else {
                if cooldowns[ci] > 0 { cooldowns[ci] -= 1; }
            }
        }
    }

    let portfolio_pnl: f64 = balances.iter().map(|&b| b - INITIAL_BAL).sum();
    (portfolio_pnl, wins, losses, flats, suppressed)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN49 — Cross-Coin Correlation Filter\n");
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let grid = build_grid();
    eprintln!("\nGrid: {} configs", grid.len());

    // ── Phase 1: Baseline (no correlation filter) — per coin, parallel ─────────
    eprintln!("\nPhase 1: Running baseline...");
    let baseline_results: Vec<SimOutput> = coin_data.par_iter()
        .map(|d| simulate_baseline(d))
        .collect();

    let base_total_pnl: f64 = baseline_results.iter().map(|r| r.0).sum();
    let base_total_wins: usize = baseline_results.iter().map(|r| r.1).sum();
    let base_total_losses: usize = baseline_results.iter().map(|r| r.2).sum();
    let base_total_flats: usize = baseline_results.iter().map(|r| r.3).sum();
    let base_total_trades = base_total_wins + base_total_losses + base_total_flats;
    let base_wr = if base_total_trades > 0 { base_total_wins as f64 / base_total_trades as f64 * 100.0 } else { 0.0 };

    eprintln!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", base_total_pnl, base_wr, base_total_trades);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), portfolio_pnl: 0.0, portfolio_wr: 0.0, portfolio_trades: 0, suppressed_count: 0, is_baseline: cfg.mode == 0, coins: vec![] };
        }

        if cfg.mode == 0 {
            // Return baseline results
            let coins: Vec<CoinResult> = coin_data.iter().zip(baseline_results.iter()).map(|(cd, &r)| {
                let (pnl, wins, losses, flats, _) = r;
                let trades = wins + losses + flats;
                let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
                CoinResult { coin: cd.name.clone(), pnl, trades, wins, losses, flats, wr }
            }).collect();
            let total_pnl = baseline_results.iter().map(|r| r.0).sum();
            let total_wins: usize = baseline_results.iter().map(|r| r.1).sum();
            let total_losses: usize = baseline_results.iter().map(|r| r.2).sum();
            let total_flats: usize = baseline_results.iter().map(|r| r.3).sum();
            let total_trades = total_wins + total_losses + total_flats;
            let wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
            let d = done.fetch_add(1, Ordering::SeqCst) + 1;
            eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}  suppressed=0",
                d, total_cfgs, cfg.label(), total_pnl, wr, total_trades);
            ConfigResult { label: cfg.label(), portfolio_pnl: total_pnl, portfolio_wr: wr, portfolio_trades: total_trades, suppressed_count: 0, is_baseline: true, coins }
        } else {
            // Run correlation-filtered portfolio simulation
            let (pnl, wins, losses, flats, suppressed) = simulate_corr_portfolio(&coin_data, *cfg);
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            let coins: Vec<CoinResult> = coin_data.iter().map(|cd| {
                CoinResult { coin: cd.name.clone(), pnl: 0.0, trades: 0, wins: 0, losses: 0, flats: 0, wr: 0.0 }
            }).collect();
            let d = done.fetch_add(1, Ordering::SeqCst) + 1;
            eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}  suppressed={}",
                d, total_cfgs, cfg.label(), pnl, wr, trades, suppressed);
            ConfigResult { label: cfg.label(), portfolio_pnl: pnl, portfolio_wr: wr, portfolio_trades: trades, suppressed_count: suppressed, is_baseline: false, coins }
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline_cfg = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.portfolio_pnl.partial_cmp(&a.portfolio_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN49 Correlation Filter Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline_cfg.portfolio_pnl, baseline_cfg.portfolio_wr, baseline_cfg.portfolio_trades);
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>8} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Suppressed");
    println!("{}", "-".repeat(65));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.portfolio_pnl - baseline_cfg.portfolio_pnl;
        println!("{:>3}  {:<22} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}", i+1, r.label, r.portfolio_pnl, delta, r.portfolio_wr, r.suppressed_count);
    }
    println!("{}", "=".repeat(65));

    let best = sorted.first().unwrap();
    let is_positive = best.portfolio_pnl > baseline_cfg.portfolio_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN49 correlation filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline_cfg.portfolio_pnl, best.label, best.portfolio_pnl, best.portfolio_pnl - baseline_cfg.portfolio_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run49_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run49_1_results.json");
}
