/// RUN86.2 — Walk-Forward Validation for Coin Correlation Clustering
///
/// Best config from grid: T0.70_MS3_MC1_CD4 (threshold=0.70, min_size=3, max_coins=1, cooldown=4)
/// 3-window walk-forward: train 2mo / test 1mo
///
/// Run: cargo run --release --features run86_2 -- --run86_2

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

const BEST_CFG: (&str, f64, usize, usize, usize) = ("T0.70_MS3_MC1_CD4", 0.70, 3, 1, 4);

const WINDOWS: [(usize, usize, usize); 3] = [
    (0, 5760, 8640),      // train 0-5760, test 5760-8640
    (2880, 8640, 11520),   // train 2880-8640, test 8640-11520
    (5760, 11520, 14400), // train 5760-11520, test 11520-14400
];

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    zscore: Vec<f64>,
    returns: Vec<f64>,
}

#[derive(Serialize)]
struct WfWin { window: usize, train_pnl_baseline: f64, train_pnl_best: f64, train_delta: f64,
    test_pnl_baseline: f64, test_pnl_best: f64, test_delta: f64, pass: bool }
#[derive(Serialize)]
struct Output { best_config: String, threshold: f64, min_size: usize, max_coins: usize, cooldown: usize, windows: Vec<WfWin> }

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
    let mut returns = vec![f64::NAN; n];
    for i in 1..n {
        returns[i] = if closes[i-1] > 0.0 { (closes[i] - closes[i-1]) / closes[i-1] } else { 0.0 };
    }
    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore, returns })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn pearson_corr(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.len() < 5 { return 0.0; }
    let n = a.len() as f64;
    let (sum_a, sum_b, sum_ab, sum_a2, sum_b2) = a.iter().zip(b.iter())
        .fold((0.0, 0.0, 0.0, 0.0, 0.0), |(sa,sb,sab,sa2,sb2), (x,y)| {
            (sa+x, sb+y, sab+x*y, sa2+x*x, sb2+y*y)
        });
    let num = n*sum_ab - sum_a*sum_b;
    let den = ((n*sum_a2-sum_a*sum_a)*(n*sum_b2-sum_b*sum_b)).sqrt();
    if den == 0.0 { 0.0 } else { num/den }
}

fn compute_corr_matrix(data: &[&CoinData15m], bar: usize, lookback: usize) -> Vec<Vec<f64>> {
    let n = data.len();
    let mut corr = vec![vec![0.0; n]; n];
    for i in 0..n {
        corr[i][i] = 1.0;
        for j in (i+1)..n {
            let start = bar.saturating_sub(lookback);
            let len = (bar - start).min(data[i].returns.len());
            if len < 5 { continue; }
            let a = &data[i].returns[start..start+len];
            let b = &data[j].returns[start..start+len];
            let r = pearson_corr(a, b);
            corr[i][j] = r;
            corr[j][i] = r;
        }
    }
    corr
}

fn find_clusters(corr: &[Vec<f64>], threshold: f64, min_size: usize) -> Vec<Vec<usize>> {
    let n = corr.len();
    let mut visited = vec![false; n];
    let mut clusters = Vec::new();
    for i in 0..n {
        if visited[i] { continue; }
        let mut cluster = vec![i];
        visited[i] = true;
        for j in (i+1)..n {
            if !visited[j] && corr[i][j] > threshold {
                cluster.push(j);
                visited[j] = true;
            }
        }
        if cluster.len() >= min_size {
            clusters.push(cluster);
        }
    }
    clusters
}

fn simulate_segment(cfg: &CorrCfgSim, coin_data: &[&CoinData15m], start: usize, end: usize) -> f64 {
    let n = end.min(coin_data[0].closes.len());
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldowns = vec![0usize; N_COINS];
    let mut suppress_until = vec![0usize; N_COINS];
    let corr_lookback = 20usize;

    for i in start..end {
        let corr = if i % 5 == 0 && i >= corr_lookback {
            Some(compute_corr_matrix(coin_data, i, corr_lookback))
        } else { None };

        if !cfg.is_baseline {
            if let Some(ref cm) = corr {
                let clusters = find_clusters(cm, cfg.threshold, cfg.min_size);
                for cluster in clusters {
                    let mut signaling = Vec::new();
                    for &ci in &cluster {
                        if positions[ci].is_none() && cooldowns[ci] == 0 && suppress_until[ci] <= i {
                            if regime_signal(coin_data[ci].zscore[i]).is_some() {
                                signaling.push(ci);
                            }
                        }
                    }
                    if signaling.len() >= cfg.min_size {
                        for &ci in signaling.iter().skip(cfg.max_coins) {
                            suppress_until[ci] = i + cfg.suppress_cd;
                        }
                    }
                }
            }
        }

        for c in 0..N_COINS {
            let d = coin_data[c];
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
                    positions[c] = None;
                    cooldowns[c] = COOLDOWN;
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
            } else if suppress_until[c] > i {
                // suppressed
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
    balances.iter().map(|&b| b - INITIAL_BAL).sum()
}

#[derive(Clone)]
struct CorrCfgSim {
    threshold: f64,
    min_size: usize,
    max_coins: usize,
    suppress_cd: usize,
    is_baseline: bool,
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN86.2 — Walk-Forward Validation: Coin Correlation Clustering\n");
    let (cfg_label, threshold, min_size, max_coins, suppress_cd) = BEST_CFG;
    eprintln!("Best config: {} (threshold={}, min_size={}, max_coins={}, cooldown={})\n", cfg_label, threshold, min_size, max_coins, suppress_cd);

    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        raw_data.push(load_15m(name));
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let coin_refs: Vec<&CoinData15m> = coin_data.iter().collect();

    let baseline_cfg = CorrCfgSim { threshold: 0.0, min_size: 0, max_coins: 99, suppress_cd: 0, is_baseline: true };
    let best_cfg = CorrCfgSim { threshold, min_size, max_coins, suppress_cd, is_baseline: false };

    let mut results: Vec<WfWin> = Vec::new();
    for (wi, &(train_s, train_e, test_e)) in WINDOWS.iter().enumerate() {
        let test_s = train_e;
        eprintln!("Window {}: train={}-{}, test={}-{}", wi+1, train_s, train_e, test_s, test_e);

        let train_baseline: f64 = simulate_segment(&baseline_cfg, &coin_refs, train_s, train_e);
        let train_best: f64 = simulate_segment(&best_cfg, &coin_refs, train_s, train_e);
        let test_baseline: f64 = simulate_segment(&baseline_cfg, &coin_refs, test_s, test_e);
        let test_best: f64 = simulate_segment(&best_cfg, &coin_refs, test_s, test_e);

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

    println!("\n=== RUN86.2 Walk-Forward Summary ===");
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

    let output = Output { best_config: cfg_label.to_string(), threshold, min_size, max_coins, cooldown: suppress_cd, windows: results };
    std::fs::write("/home/scamarena/ProjectCoin/run86_2_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run86_2_results.json");
}
