/// RUN94.2 — Walk-Forward Validation for Partial Reentry After Cooldown
///
/// Best config from grid: ZM1.1_SP0.7_MC1 (z_mult=1.1, size_pct=0.7, max_count=1)
/// 3-window walk-forward: train 2mo / test 1mo
///
/// Run: cargo run --release --features run94_2 -- --run94_2

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

const BEST_CFG: (&str, f64, f64, usize) = ("ZM1.1_SP0.7_MC1", 1.1, 0.7, 1);

const WINDOWS: [(usize, usize, usize); 3] = [
    (0, 5760, 8640),
    (2880, 8640, 11520),
    (5760, 11520, 14400),
];

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct WfWin { window: usize, train_pnl_baseline: f64, train_pnl_best: f64, train_delta: f64,
    test_pnl_baseline: f64, test_pnl_best: f64, test_delta: f64, pass: bool }
#[derive(Serialize)]
struct Output { best_config: String, z_mult: f64, size_pct: f64, max_count: usize, windows: Vec<WfWin> }

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

fn simulate_segment(z_mult: f64, size_pct: f64, max_count: usize, is_baseline: bool, coin_data: &[&CoinData15m], start: usize, end: usize) -> f64 {
    let n = end.min(coin_data[0].closes.len());
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64, f64)>> = vec![None; N_COINS]; // dir, entry, z_entry
    let mut cooldowns = vec![0usize; N_COINS];
    let mut reentry_count = vec![0usize; N_COINS];
    let mut original_z: Vec<Option<f64>> = vec![None; N_COINS];
    let mut pos_size_mult = vec![1.0f64; N_COINS];

    for i in start..end {
        for c in 0..N_COINS {
            let d = coin_data[c];
            if let Some((dir, entry, z_entry)) = positions[c] {
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
                    let effective_size = POSITION_SIZE * pos_size_mult[c];
                    let net = balances[c] * effective_size * LEVERAGE * exit_pct;
                    balances[c] += net;
                    original_z[c] = Some(z_entry);
                    positions[c] = None;
                    cooldowns[c] = COOLDOWN;
                    reentry_count[c] = 0;
                    pos_size_mult[c] = 1.0;
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
                // Re-entry check during cooldown
                if !is_baseline && z_mult > 0.0 {
                    if let Some(dir) = regime_signal(d.zscore[i]) {
                        if let Some(orig_z) = original_z[c] {
                            if reentry_count[c] < max_count {
                                let threshold = orig_z * z_mult;
                                let can_reenter = match dir {
                                    1 => d.zscore[i] < threshold,
                                    -1 => d.zscore[i] > threshold,
                                    _ => false,
                                };
                                if can_reenter && i + 1 < n {
                                    let entry_price = d.opens[i + 1];
                                    if entry_price > 0.0 {
                                        positions[c] = Some((dir, entry_price, d.zscore[i]));
                                        reentry_count[c] += 1;
                                        cooldowns[c] = 0;
                                        pos_size_mult[c] = size_pct;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            positions[c] = Some((dir, entry_price, d.zscore[i]));
                            original_z[c] = None;
                            reentry_count[c] = 0;
                            pos_size_mult[c] = 1.0;
                        }
                    }
                }
            }
        }
    }
    balances.iter().map(|&b| b - INITIAL_BAL).sum()
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN94.2 — Walk-Forward Validation: Partial Reentry After Cooldown\n");
    let (cfg_label, z_mult, size_pct, max_count) = BEST_CFG;
    eprintln!("Best config: {} (z_mult={}, size_pct={}, max_count={})\n", cfg_label, z_mult, size_pct, max_count);

    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES { raw_data.push(load_15m(name)); }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let coin_refs: Vec<&CoinData15m> = coin_data.iter().collect();

    let mut results: Vec<WfWin> = Vec::new();
    for (wi, &(train_s, train_e, test_e)) in WINDOWS.iter().enumerate() {
        let test_s = train_e;
        eprintln!("Window {}: train={}-{}, test={}-{}", wi+1, train_s, train_e, test_s, test_e);

        let train_baseline = simulate_segment(0.0, 1.0, 0, true, &coin_refs, train_s, train_e);
        let train_best = simulate_segment(z_mult, size_pct, max_count, false, &coin_refs, train_s, train_e);
        let test_baseline = simulate_segment(0.0, 1.0, 0, true, &coin_refs, test_s, test_e);
        let test_best = simulate_segment(z_mult, size_pct, max_count, false, &coin_refs, test_s, test_e);

        let train_delta = train_best - train_baseline;
        let test_delta = test_best - test_baseline;
        let pass = test_delta > 0.0;
        eprintln!("  Train: baseline={:+.2}, best={:+.2}, delta={:+.2}", train_baseline, train_best, train_delta);
        eprintln!("  Test:  baseline={:+.2}, best={:+.2}, delta={:+.2}  --> {}", test_baseline, test_best, test_delta, if pass { "PASS" } else { "FAIL" });

        results.push(WfWin { window: wi + 1, train_pnl_baseline: train_baseline, train_pnl_best: train_best, train_delta, test_pnl_baseline: test_baseline, test_pnl_best: test_best, test_delta, pass });
    }

    let passes = results.iter().filter(|r| r.pass).count();
    let avg_test_delta: f64 = results.iter().map(|r| r.test_delta).sum::<f64>() / 3.0;
    let is_positive = passes >= 2 && avg_test_delta > 0.0;

    println!("\n=== RUN94.2 Walk-Forward Summary ===");
    println!("Config: {}", cfg_label);
    println!("{:>3}  {:>12}  {:>12}  {:>10}  {:>12}  {:>12}  {:>10}  {:>5}",
        "Win", "Train(Base)", "Train(Best)", "Δ(train)", "Test(Base)", "Test(Best)", "Δ(test)", "Pass?");
    println!("{}", "-".repeat(80));
    for r in &results {
        println!("  {}  {:>+12.2}  {:>+12.2}  {:>+10.2}  {:>+12.2}  {:>+12.2}  {:>+10.2}  {:>5}",
            r.window, r.train_pnl_baseline, r.train_pnl_best, r.train_delta,
            r.test_pnl_baseline, r.test_pnl_best, r.test_delta, r.pass);
    }
    println!("{}", "=".repeat(80));
    println!("Passes: {}/3  Avg test Δ: {:+.2}", passes, avg_test_delta);
    println!("VERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let output = Output { best_config: cfg_label.to_string(), z_mult, size_pct, max_count, windows: results };
    std::fs::write("/home/scamarena/ProjectCoin/run94_2_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run94_2_results.json");
}
