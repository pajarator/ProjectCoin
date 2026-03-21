/// RUN88.2 — Walk-Forward Validation for Trailing Z-Score Exit
///
/// Best config from grid: RP0.75_MH8 (recovery_pct=0.75, min_hold=8)
/// 3-window walk-forward: train 2mo / test 1mo
///
/// Run: cargo run --release --features run88_2 -- --run88_2

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

const BEST_CFG: (&str, f64, usize) = ("RP0.75_MH8", 0.75, 8);

const WINDOWS: [(usize, usize, usize); 3] = [
    (0, 5760, 8640),       // train 0-5760, test 5760-8640
    (2880, 8640, 11520),   // train 2880-8640, test 8640-11520
    (5760, 11520, 14400),  // train 5760-11520, test 11520-14400
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
struct Output { best_config: String, recovery_pct: f64, min_hold: usize, windows: Vec<WfWin> }

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

fn simulate_segment(recovery_pct: f64, min_hold: usize, coin_data: &[&CoinData15m], start: usize, end: usize) -> f64 {
    let n = end.min(coin_data[0].closes.len());
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64, usize, f64)>> = vec![None; N_COINS]; // dir, entry, entry_bar, z_entry
    let mut cooldowns = vec![0usize; N_COINS];

    for i in start..end {
        for c in 0..N_COINS {
            let d = coin_data[c];
            if let Some((dir, entry, entry_bar, z_entry)) = positions[c] {
                let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
                let mut closed = false;
                let mut exit_pct = 0.0;
                let bars_held = i - entry_bar;

                // Z-recovery exit
                if !closed && recovery_pct > 0.0 && bars_held >= min_hold {
                    let z_target = z_entry * (1.0 - recovery_pct);
                    let z_curr = d.zscore[i];
                    if !z_curr.is_nan() {
                        let recovered = if dir == 1 { z_curr >= z_target } else { z_curr <= z_target };
                        if recovered { exit_pct = pct; closed = true; }
                    }
                }

                // SL
                if !closed && pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }

                // Z0 exit
                if !closed {
                    let new_dir = regime_signal(d.zscore[i]);
                    if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
                }

                // End
                if !closed && i >= n - 1 { exit_pct = pct; closed = true; }

                if closed {
                    let net = balances[c] * POSITION_SIZE * LEVERAGE * exit_pct;
                    balances[c] += net;
                    positions[c] = None;
                    cooldowns[c] = COOLDOWN;
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
            } else {
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            let z_entry = d.zscore[i];
                            positions[c] = Some((dir, entry_price, i, z_entry));
                        }
                    }
                }
            }
        }
    }
    balances.iter().map(|&b| b - INITIAL_BAL).sum()
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN88.2 — Walk-Forward Validation: Trailing Z-Score Exit\n");
    let (cfg_label, recovery_pct, min_hold) = BEST_CFG;
    eprintln!("Best config: {} (recovery_pct={}, min_hold={})\n", cfg_label, recovery_pct, min_hold);

    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        raw_data.push(load_15m(name));
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let coin_refs: Vec<&CoinData15m> = coin_data.iter().collect();

    let mut results: Vec<WfWin> = Vec::new();
    for (wi, &(train_s, train_e, test_e)) in WINDOWS.iter().enumerate() {
        let test_s = train_e;
        eprintln!("Window {}: train={}-{}, test={}-{}", wi+1, train_s, train_e, test_s, test_e);

        // Baseline: simulate with recovery_pct=0.0 (no recovery exit)
        let train_baseline = simulate_segment(0.0, 0, &coin_refs, train_s, train_e);
        let train_best = simulate_segment(recovery_pct, min_hold, &coin_refs, train_s, train_e);
        let test_baseline = simulate_segment(0.0, 0, &coin_refs, test_s, test_e);
        let test_best = simulate_segment(recovery_pct, min_hold, &coin_refs, test_s, test_e);

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

    println!("\n=== RUN88.2 Walk-Forward Summary ===");
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

    let output = Output { best_config: cfg_label.to_string(), recovery_pct, min_hold, windows: results };
    std::fs::write("/home/scamarena/ProjectCoin/run88_2_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run88_2_results.json");
}
