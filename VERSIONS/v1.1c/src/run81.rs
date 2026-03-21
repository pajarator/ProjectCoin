/// RUN81 — Equity Curve Circuit Breaker
///
/// Halt regime entries when portfolio drawdown exceeds threshold for sustained bars:
/// - Track peak_equity and current_equity
/// - Circuit breaker activates after CONSECUTIVE bars below THRESHOLD
/// - During breaker: regime entries blocked (scalp/momentum/ISO exempt)
/// - Deactivates when equity recovers to within RECOVERY of peak
///
/// Grid: THRESHOLD [0.10, 0.15, 0.20] × BARS [5, 10, 20] × RECOVERY [0.05, 0.10, 0.15]
/// Total: 3 × 3 × 3 = 27 + baseline = 28 configs
///
/// Run: cargo run --release --features run81 -- --run81

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

#[derive(Clone, Debug)]
struct BreakerCfg {
    threshold: f64,   // drawdown threshold to start counting
    bars: usize,       // consecutive bars below threshold to activate
    recovery: f64,     // deactivate when drawdown <= this
    is_baseline: bool,
}

impl BreakerCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("T{:+.0}_B{}_R{:+.0}", self.threshold * 100.0, self.bars, self.recovery * 100.0) }
    }
}

fn build_grid() -> Vec<BreakerCfg> {
    let mut grid = vec![BreakerCfg { threshold: 0.0, bars: 0, recovery: 0.0, is_baseline: true }];
    let thresholds = [0.10, 0.15, 0.20];
    let bars_vals = [5usize, 10, 20];
    let recoveries = [0.05, 0.10, 0.15];
    for &t in &thresholds {
        for &b in &bars_vals {
            for &r in &recoveries {
                grid.push(BreakerCfg { threshold: t, bars: b, recovery: r, is_baseline: false });
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
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, breaker_activations: usize, blocked: usize }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, breaker_active_pct: f64,
    regime_entries_blocked: usize, breaker_activations: usize, coins: Vec<CoinResult>,
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

/// Portfolio-level simulation with circuit breaker
fn simulate_portfolio(coin_data: &[CoinData15m], cfg: &BreakerCfg) -> ConfigResult {
    if cfg.is_baseline {
        // Baseline: no circuit breaker, just sum all coin results
        let mut total_pnl = 0.0;
        let mut total_wins = 0usize;
        let mut total_losses = 0usize;
        let mut total_flats = 0usize;
        let mut total_trades = 0usize;
        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
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
                            if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                        }
                    }
                }
            }
            let pnl = bal - INITIAL_BAL;
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            total_pnl += pnl;
            total_wins += wins;
            total_losses += losses;
            total_flats += flats;
            total_trades += trades;
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, wr, breaker_activations: 0, blocked: 0 }
        }).collect();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = total_wins as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
        let losses_f = total_losses as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };
        return ConfigResult {
            label: "BASELINE".to_string(), total_pnl, portfolio_wr, total_trades,
            pf, is_baseline: true, breaker_active_pct: 0.0,
            regime_entries_blocked: 0, breaker_activations: 0, coins: coin_results,
        };
    }

    // With circuit breaker: need bar-by-bar portfolio equity tracking
    let n = coin_data[0].closes.len();

    // Per-coin balances
    let mut balances: Vec<f64> = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldowns: Vec<usize> = vec![0usize; N_COINS];

    let mut peak_equity = INITIAL_BAL * N_COINS as f64;
    let mut consec_below = 0usize;
    let mut breaker_active = false;
    let mut breaker_activations = 0usize;
    let mut total_blocked = 0usize;
    let mut breaker_active_bars = 0usize;

    // Per-coin stats
    let mut wins = vec![0usize; N_COINS];
    let mut losses = vec![0usize; N_COINS];
    let mut flats = vec![0usize; N_COINS];

    for i in 1..n {
        // Update portfolio equity and check circuit breaker at each bar
        let current_equity: f64 = balances.iter().sum();
        if current_equity > peak_equity {
            peak_equity = current_equity;
        }

        let drawdown = if peak_equity > 0.0 { (peak_equity - current_equity) / peak_equity } else { 0.0 };

        // Circuit breaker logic
        if drawdown >= cfg.threshold {
            consec_below += 1;
            if consec_below >= cfg.bars && !breaker_active {
                breaker_active = true;
                breaker_activations += 1;
            }
        } else {
            consec_below = 0;
            // Check recovery: if drawdown <= recovery threshold, deactivate
            if drawdown <= cfg.recovery {
                breaker_active = false;
            }
        }

        if breaker_active { breaker_active_bars += 1; }

        // Process each coin
        for c in 0..N_COINS {
            let d = &coin_data[c];

            // Handle existing position
            if let Some((dir, entry)) = positions[c] {
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
                    let net = balances[c] * POSITION_SIZE * LEVERAGE * exit_pct;
                    balances[c] += net;
                    if net > 1e-10 { wins[c] += 1; }
                    else if net < -1e-10 { losses[c] += 1; }
                    else { flats[c] += 1; }
                    positions[c] = None;
                    cooldowns[c] = COOLDOWN;
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
            } else {
                // Check for new entry
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    // Block regime entries if breaker is active
                    if breaker_active {
                        total_blocked += 1;
                    } else {
                        if i + 1 < n {
                            let entry_price = d.opens[i + 1];
                            if entry_price > 0.0 {
                                positions[c] = Some((dir, entry_price));
                            }
                        }
                    }
                }
            }
        }
    }

    // Aggregate results
    let total_pnl: f64 = balances.iter().map(|&b| b - INITIAL_BAL).sum();
    let total_wins: usize = wins.iter().sum();
    let total_losses: usize = losses.iter().sum();
    let total_flats: usize = flats.iter().sum();
    let total_trades = total_wins + total_losses + total_flats;
    let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let gross = total_wins as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
    let losses_f = total_losses as f64;
    let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };
    let breaker_active_pct = breaker_active_bars as f64 / n as f64 * 100.0;

    let coin_results: Vec<CoinResult> = (0..N_COINS).map(|c| {
        let pnl = balances[c] - INITIAL_BAL;
        let trades = wins[c] + losses[c] + flats[c];
        let wr = if trades > 0 { wins[c] as f64 / trades as f64 * 100.0 } else { 0.0 };
        CoinResult { coin: coin_data[c].name.clone(), pnl, trades, wins: wins[c], losses: losses[c], wr, breaker_activations: 0, blocked: 0 }
    }).collect();

    ConfigResult {
        label: cfg.label(), total_pnl, portfolio_wr, total_trades,
        pf, is_baseline: false, breaker_active_pct,
        regime_entries_blocked: total_blocked, breaker_activations, coins: coin_results,
    }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN81 — Equity Curve Circuit Breaker\n");
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
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, breaker_active_pct: 0.0,
                regime_entries_blocked: 0, breaker_activations: 0, coins: vec![],
            };
        }
        let result = simulate_portfolio(&coin_data, cfg);
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  breaker={:.1}%  blocked={}",
            d, total_cfgs, result.label, result.total_pnl, result.portfolio_wr,
            result.total_trades, result.breaker_active_pct, result.regime_entries_blocked);
        result
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN81 Equity Curve Circuit Breaker Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<18} {:>8} {:>8} {:>6} {:>7} {:>8} {:>9} {:>10}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF", "Breaker%", "Blocked");
    println!("{}", "-".repeat(85));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<18} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2} {:>8.1}%  {:>9}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf, r.breaker_active_pct, r.regime_entries_blocked);
    }
    println!("{}", "=".repeat(85));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN81 equity circuit breaker. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run81_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run81_1_results.json");
}
