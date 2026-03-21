/// RUN83 — Cooldown by Market Mode
///
/// Different cooldown periods based on market mode (LONG/ISO_SHORT/SHORT) and exit quality.
/// Market mode determined by breadth (% of coins with z < -1.5):
///   LONG: breadth <= 0.20
///   ISO_SHORT: 0.20 < breadth <= 0.50
///   SHORT: breadth > 0.50
///
/// Grid: LONG_GOOD [1,2,3] × LONG_BAD [4,6,8] × ISO_GOOD [4,6,8] × ISO_BAD [15,20,30] × ESCALATE [2,3,5]
/// Total: 3 × 3 × 3 × 3 × 3 = 243 + baseline = 244 configs
///
/// Run: cargo run --release --features run83 -- --run83

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const BASE_COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct CooldownCfg {
    long_good: usize,
    long_bad: usize,
    iso_good: usize,
    iso_bad: usize,
    escalate: usize,
    is_baseline: bool,
}

impl CooldownCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("LG{}_LB{}_IG{}_IB{}_E{}", self.long_good, self.long_bad, self.iso_good, self.iso_bad, self.escalate) }
    }
}

fn build_grid() -> Vec<CooldownCfg> {
    let mut grid = vec![CooldownCfg { long_good: 2, long_bad: 2, iso_good: 2, iso_bad: 2, escalate: 1, is_baseline: true }];
    let lg_vals = [1usize, 2, 3];
    let lb_vals = [4usize, 6, 8];
    let ig_vals = [4usize, 6, 8];
    let ib_vals = [15usize, 20, 30];
    let esc_vals = [2usize, 3, 5];
    for &lg in &lg_vals {
        for &lb in &lb_vals {
            for &ig in &ig_vals {
                for &ib in &ib_vals {
                    for &e in &esc_vals {
                        grid.push(CooldownCfg { long_good: lg, long_bad: lb, iso_good: ig, iso_bad: ib, escalate: e, is_baseline: false });
                    }
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
    breadth: Vec<f64>, // market mode at each bar
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
    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore, breadth: vec![0.0; n] })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn compute_breadth(data: &[&CoinData15m]) -> Vec<f64> {
    let n = data[0].closes.len();
    let mut breadth = vec![0.0; n];
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

fn simulate(cfg: &CooldownCfg, coin_data: &[CoinData15m], breadth: &[f64]) -> ConfigResult {
    let n = coin_data[0].closes.len();
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldowns = vec![0usize; N_COINS];
    let mut consec_sl = vec![0usize; N_COINS];
    let mut wins = vec![0usize; N_COINS];
    let mut losses = vec![0usize; N_COINS];
    let mut flats = vec![0usize; N_COINS];

    for i in 1..n {
        let mode = if breadth[i] <= 0.20 { 0 } // LONG
                   else if breadth[i] <= 0.50 { 1 } // ISO_SHORT
                   else { 2 }; // SHORT

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
                    if net > 1e-10 { wins[c] += 1; consec_sl[c] = 0; }
                    else if net < -1e-10 { losses[c] += 1; consec_sl[c] += 1; }
                    else { flats[c] += 1; consec_sl[c] = 0; }

                    // Compute cooldown
                    if cfg.is_baseline {
                        // Baseline: fixed 2-bar cooldown
                        cooldowns[c] = BASE_COOLDOWN;
                    } else {
                        let bad_exit = exit_pct <= -SL_PCT;
                        let new_mode = mode; // Use current mode

                        let base = match (new_mode as usize, bad_exit) {
                            (0, false) => cfg.long_good,  // LONG, good exit
                            (0, true) => cfg.long_bad,    // LONG, bad exit
                            (1, false) => cfg.iso_good,   // ISO_SHORT, good exit
                            (1, true) => cfg.iso_bad,     // ISO_SHORT, bad exit
                            (2, _) => cfg.long_good,      // SHORT: use LONG good (not specified)
                            _ => cfg.long_good,
                        };

                        // Escalation for consecutive SLs
                        if bad_exit && consec_sl[c] >= 2 {
                            cooldowns[c] = base * cfg.escalate;
                        } else {
                            cooldowns[c] = base;
                        }
                    }
                    positions[c] = None;
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
            } else {
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 { positions[c] = Some((dir, entry_price)); }
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

    let coins: Vec<CoinResult> = (0..N_COINS).map(|c| {
        let pnl = balances[c] - INITIAL_BAL;
        let trades = wins[c] + losses[c] + flats[c];
        let wr = if trades > 0 { wins[c] as f64 / trades as f64 * 100.0 } else { 0.0 };
        CoinResult { coin: coin_data[c].name.clone(), pnl, trades, wins: wins[c], losses: losses[c], wr }
    }).collect();

    ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf, is_baseline: cfg.is_baseline, coins }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN83 — Cooldown by Market Mode\n");
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

    // Compute breadth from all coin z-scores
    let coin_refs: Vec<&CoinData15m> = coin_data.iter().collect();
    let breadth = compute_breadth(&coin_refs);
    eprintln!("Breadth range: {:.1}% - {:.1}%",
        breadth.iter().filter(|&&x| x > 0.0).fold(f64::INFINITY, |a, &x| a.min(x)) * 100.0,
        breadth.iter().fold(0.0f64, |a, &x| a.max(x)) * 100.0);

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![] };
        }
        let result = simulate(cfg, &coin_data, &breadth);
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        if d % 20 == 0 || d == total_cfgs {
            eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
                d, total_cfgs, result.label, result.total_pnl, result.portfolio_wr, result.total_trades);
        }
        result
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN83 Cooldown by Market Mode Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>6} {:>7} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(70));
    for (i, r) in sorted.iter().enumerate().take(30) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<22} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("... ({} total configs)", sorted.len());
    println!("{}", "=".repeat(70));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    let top5_positive = sorted.iter().take(5).filter(|r| !r.is_baseline && r.total_pnl > baseline.total_pnl).count();
    println!("\nTop 5: {}/5 positive  Best: {} (PnL={:+.2}, Δ={:+.2})", top5_positive, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    println!("VERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN83 cooldown by mode. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run83_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run83_1_results.json");
}
