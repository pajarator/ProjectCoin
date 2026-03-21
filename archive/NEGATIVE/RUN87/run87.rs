/// RUN87 — Drawdown Recovery Mode
///
/// When portfolio drawdown >= DD_RECOVERY_THRESHOLD, shift market mode bias:
/// - Tighten ISO_SHORT_BREADTH_MAX (lower threshold → more ISO shorts fire)
/// - Widen BREADTH_MAX for LONG (higher threshold → fewer LONG entries)
///
/// Grid: THRESHOLD [0.08, 0.10, 0.15] × EXIT [0.04, 0.05, 0.08]
///       × ISO_TIGHTEN [0.03, 0.05, 0.08] × LONG_WIDEN [0.03, 0.05, 0.08]
/// Total: 3 × 3 × 3 × 3 = 81 + baseline = 82 configs
///
/// Run: cargo run --release --features run87 -- --run87

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

// Standard COINCLAW v16 breadth thresholds
const BREADTH_MAX_LONG: f64 = 0.20;    // LONG: breadth <= this
const BREADTH_MAX_ISO: f64 = 0.20;     // ISO_SHORT: breadth <= this (above LONG threshold)
const BREADTH_MIN_SHORT: f64 = 0.50;   // SHORT: breadth >= this

#[derive(Clone, Debug)]
struct DdCfg {
    threshold: f64,       // activate recovery at this drawdown %
    exit_dd: f64,         // exit recovery when drawdown <= this
    iso_tighten: f64,      // subtract from ISO threshold during recovery
    long_widen: f64,      // add to LONG threshold during recovery
    is_baseline: bool,
}

impl DdCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else {
            format!("TH{:.2}_EX{:.2}_IT{:.2}_LW{:.2}",
                self.threshold, self.exit_dd, self.iso_tighten, self.long_widen)
        }
    }
}

fn build_grid() -> Vec<DdCfg> {
    let mut grid = vec![DdCfg { threshold: 0.0, exit_dd: 0.0, iso_tighten: 0.0, long_widen: 0.0, is_baseline: true }];
    let thresholds = [0.08, 0.10, 0.15];
    let exits = [0.04, 0.05, 0.08];
    let iso_tightens = [0.03, 0.05, 0.08];
    let long_widens = [0.03, 0.05, 0.08];
    for &th in &thresholds {
        for &ex in &exits {
            for &it in &iso_tightens {
                for &lw in &long_widens {
                    grid.push(DdCfg { threshold: th, exit_dd: ex, iso_tighten: it, long_widen: lw, is_baseline: false });
                }
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
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, recovery_bars: usize }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, recovery_activation_rate: f64,
    long_entries: usize, iso_entries: usize, short_entries: usize,
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

fn compute_breadth(data: &[&CoinData15m]) -> Vec<f64> {
    let n = data[0].closes.len();
    let mut breadth = vec![f64::NAN; n];
    for i in 20..n {
        let mut count = 0usize;
        let mut valid = 0usize;
        for d in data {
            if !d.zscore[i].is_nan() {
                valid += 1;
                if d.zscore[i] < -1.5 { count += 1; }
            }
        }
        breadth[i] = if valid > 0 { count as f64 / valid as f64 } else { 0.0 };
    }
    breadth
}

fn simulate(cfg: &DdCfg, coin_data: &[CoinData15m], breadth: &[f64]) -> ConfigResult {
    let n = coin_data[0].closes.len();
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldowns = vec![0usize; N_COINS];
    let mut wins = vec![0usize; N_COINS];
    let mut losses = vec![0usize; N_COINS];
    let mut flats = vec![0usize; N_COINS];
    let mut recovery_bars = vec![0usize; N_COINS];

    // Portfolio equity tracking
    let mut peak_equity: f64 = INITIAL_BAL * N_COINS as f64;
    let mut in_recovery = false;
    let mut total_recovery_bars = 0usize;
    let mut long_entries = 0usize;
    let mut iso_entries = 0usize;
    let mut short_entries = 0usize;

    for i in 1..n {
        // Update portfolio equity
        let portfolio_equity: f64 = balances.iter().sum();
        if portfolio_equity > peak_equity {
            peak_equity = portfolio_equity;
        }

        // Drawdown check
        let drawdown = if peak_equity > 0.0 {
            (peak_equity - portfolio_equity) / peak_equity
        } else {
            0.0
        };

        // Transition into/out of recovery mode
        if !in_recovery && drawdown >= cfg.threshold {
            in_recovery = true;
        } else if in_recovery && drawdown <= cfg.exit_dd {
            in_recovery = false;
        }

        if in_recovery {
            total_recovery_bars += 1;
        }

        // Effective breadth thresholds
        let effective_breadth_max_long = BREADTH_MAX_LONG + (if in_recovery && !cfg.is_baseline { cfg.long_widen } else { 0.0 });
        let effective_breadth_max_iso = BREADTH_MAX_ISO - (if in_recovery && !cfg.is_baseline { cfg.iso_tighten } else { 0.0 });

        // Determine market mode using effective thresholds
        let mode = if breadth[i].is_nan() {
            1 // default to ISO_SHORT if no breadth data
        } else if breadth[i] <= effective_breadth_max_long {
            0 // LONG
        } else if breadth[i] > BREADTH_MIN_SHORT {
            2 // SHORT
        } else {
            1 // ISO_SHORT
        };

        // Count mode entries (only on entry bars, not every bar)
        // Only count in the loop below when we actually enter a position

        for c in 0..N_COINS {
            let d = &coin_data[c];

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
                    if in_recovery { recovery_bars[c] += 1; }
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
            } else {
                // Check if this coin should enter based on market mode alignment
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    let should_enter = match (mode, dir) {
                        (0, 1) => true,   // LONG mode + LONG signal
                        (1, -1) => true,  // ISO_SHORT mode + SHORT signal
                        (2, -1) => true,  // SHORT mode + SHORT signal
                        _ => false,
                    };
                    if should_enter && i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            positions[c] = Some((dir, entry_price));
                            // Count entry type
                            if mode == 0 { long_entries += 1; }
                            else if mode == 1 { iso_entries += 1; }
                            else { short_entries += 1; }
                        }
                    }
                }
            }
        }
    }

    let total_pnl: f64 = balances.iter().map(|&b| b - INITIAL_BAL).sum();
    let total_wins: usize = wins.iter().sum();
    let total_losses: usize = losses.iter().sum();
    let total_flats: usize = flats.iter().sum();
    let total_trades = total_wins + total_losses + total_flats;
    let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let gross = total_wins as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
    let losses_f = total_losses as f64;
    let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };
    let recovery_activation_rate = if n > 0 { total_recovery_bars as f64 / n as f64 * 100.0 } else { 0.0 };

    let coins: Vec<CoinResult> = (0..N_COINS).map(|c| {
        let pnl = balances[c] - INITIAL_BAL;
        let trades = wins[c] + losses[c] + flats[c];
        let wr = if trades > 0 { wins[c] as f64 / trades as f64 * 100.0 } else { 0.0 };
        CoinResult { coin: coin_data[c].name.clone(), pnl, trades, wins: wins[c], losses: losses[c], wr, recovery_bars: recovery_bars[c] }
    }).collect();

    ConfigResult {
        label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf,
        is_baseline: cfg.is_baseline, recovery_activation_rate,
        long_entries, iso_entries, short_entries,
        coins,
    }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN87 — Drawdown Recovery Mode\n");
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
    let coin_refs: Vec<&CoinData15m> = coin_data.iter().collect();

    // Compute breadth across all coins
    let breadth = compute_breadth(&coin_refs);

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins (portfolio-level)", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, recovery_activation_rate: 0.0,
                long_entries: 0, iso_entries: 0, short_entries: 0, coins: vec![]
            };
        }
        let result = simulate(cfg, &coin_data, &breadth);
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  recovery={:>5.1}%",
            d, total_cfgs, result.label, result.total_pnl, result.portfolio_wr,
            result.total_trades, result.recovery_activation_rate);
        result
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN87 Drawdown Recovery Mode Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}  RecoveryRate={:.1}%",
        baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades, baseline.recovery_activation_rate);
    println!("\n{:>3}  {:<40} {:>8} {:>8} {:>6} {:>7} {:>8} {:>10}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF", "Recovery%");
    println!("{}", "-".repeat(100));
    for (i, r) in sorted.iter().enumerate().take(30) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<40} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2} {:>9.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf, r.recovery_activation_rate);
    }
    println!("... ({} total)", sorted.len());
    println!("{}", "=".repeat(100));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN87 DD recovery. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2}, recovery={:.1}%)",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl, best.recovery_activation_rate);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run87_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run87_1_results.json");
}
