/// RUN101.2 — Partial Position Split: Walk-Forward Validation
///
/// Best config from grid: SL_MULT=1.0, SAT_Z_EXIT=-0.5
/// 3-window walk-forward: train 2mo / test 1mo
///
/// Run: cargo run --release --features run101_2 -- --run101-2

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

const Z_BREADTH_LONG: usize = 20;
const Z_BREADTH_SHORT: usize = 50;

// Best config from grid search
const SL_MULT: f64 = 1.0;
const SAT_Z_EXIT: f64 = -0.5;

const WINDOWS: [(usize, usize, usize, usize); 3] = [
    (0, 5760, 5760, 8640),        // train 0-5760, test 5760-8640
    (2880, 8640, 8640, 11520),     // train 2880-8640, test 8640-11520
    (5760, 11520, 11520, 14400),    // train 5760-11520, test 11520-14400
];

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct WfWin {
    window: usize,
    train_pnl_baseline: f64,
    train_pnl_best: f64,
    train_delta: f64,
    test_pnl_baseline: f64,
    test_pnl_best: f64,
    test_delta: f64,
    test_wr_baseline: f64,
    test_wr_best: f64,
    test_trades: usize,
    pass: bool,
}

#[derive(Serialize)]
struct Output {
    best_config: String,
    sl_mult: f64,
    sat_z_exit: f64,
    windows: Vec<WfWin>,
}

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut closes = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); closes.push(cc); highs.push(hh); lows.push(ll);
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
    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, zscore })
}

/// Returns (total_pnl, wins, losses, trades_count) for a simulation
fn simulate(data: &[CoinData15m], use_split: bool, start: usize, end: usize) -> (f64, usize, usize, usize) {
    let n = end.min(data[0].closes.len());
    if start >= n { return (0.0, 0, 0, 0); }

    let mut bals: Vec<f64> = vec![INITIAL_BAL; N_COINS];
    let mut pos: Vec<Option<Pos>> = vec![None; N_COINS];
    let mut cooldown: Vec<usize> = vec![0; N_COINS];
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut hold_counts: Vec<usize> = vec![0usize; N_COINS];

    for i in start..n {
        // Count hold bars for held positions
        for ci in 0..N_COINS {
            if let Some(ref p) = pos[ci] {
                if p.last_i < i {
                    hold_counts[ci] += 1;
                    pos[ci].as_mut().unwrap().last_i = i;
                }
            }
        }

        for ci in 0..N_COINS {
            let d = &data[ci];

            // Exit check
            if let Some(ref mut p) = pos[ci] {
                let pct = if p.dir == 1 { (d.closes[i]-p.e)/p.e } else { (p.e-d.closes[i])/p.e };
                let eff_sl = SL_PCT * if use_split && p.is_satellite { SL_MULT } else { 1.0 };
                let mut closed = false;
                let mut exit_pct = 0.0;

                // SL
                if pct <= -eff_sl { exit_pct = -eff_sl; closed = true; }

                // Z0 exit (after min hold) - for satellite use SAT_Z_EXIT
                if !closed && hold_counts[ci] >= MIN_HOLD_BARS {
                    let z = d.zscore[i];
                    if !z.is_nan() {
                        let z_thresh = if use_split && p.is_satellite { SAT_Z_EXIT } else { 0.0 };
                        if p.dir == 1 && z > z_thresh { exit_pct = pct; closed = true; }
                        if p.dir == -1 && z < -z_thresh { exit_pct = pct; closed = true; }
                    }
                }

                // SMA exit
                if !closed && hold_counts[ci] >= MIN_HOLD_BARS {
                    if p.dir == 1 && d.closes[i] > d.opens[i.saturating_sub(1)] {
                        exit_pct = pct; closed = true;
                    }
                    if p.dir == -1 && d.closes[i] < d.opens[i.saturating_sub(1)] {
                        exit_pct = pct; closed = true;
                    }
                }

                // End of test window
                if !closed && i >= n - 1 { exit_pct = pct; closed = true; }

                if closed {
                    let net = bals[ci] * POSITION_SIZE * LEVERAGE * exit_pct;
                    bals[ci] += net;
                    if net > 1e-10 { wins += 1; } else if net < -1e-10 { losses += 1; }
                    pos[ci] = None;
                    cooldown[ci] = COOLDOWN;
                    hold_counts[ci] = 0;
                }
            } else {
                // Entry check
                if cooldown[ci] > 0 { cooldown[ci] -= 1; continue; }
                let z = d.zscore[i];
                if z.is_nan() { continue; }

                let breadth_long = Z_BREADTH_LONG;
                let breadth_short = Z_BREADTH_SHORT;
                let regime = {
                    let z_vals: Vec<f64> = data.iter().map(|cd| cd.zscore[i]).collect();
                    let negatives: usize = z_vals.iter().filter(|&&z| z < -1.5).count();
                    let pos_fraction = negatives as f64 / N_COINS as f64;
                    if pos_fraction <= 0.20 { 1 } else if pos_fraction <= 0.50 { -1 } else { 0 }
                };

                let dir = if regime == 1 && z < -2.0 {
                    if use_split { Some(1) } else { Some(1) }
                } else if regime == -1 && z > 2.0 {
                    Some(-1)
                } else { None };

                if let Some(dir_val) = dir {
                    if i + 1 < n {
                        let entry = d.opens[i + 1];
                        if entry > 0.0 {
                            pos[ci] = Some(Pos {
                                e: entry,
                                dir: dir_val,
                                is_satellite: false,
                                last_i: i,
                            });
                            hold_counts[ci] = 0;
                        }
                    }
                }
            }
        }
    }

    let total_pnl: f64 = bals.iter().map(|b| b - INITIAL_BAL).sum();
    (total_pnl, wins, losses, wins + losses)
}

#[derive(Clone)]
struct Pos {
    e: f64,
    dir: i8,
    is_satellite: bool,
    last_i: usize,
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN101.2 — Partial Position Split Walk-Forward\n");
    eprintln!("Loading 15m data...");
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        raw_data.push(load_15m(name));
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let mut results: Vec<WfWin> = Vec::new();

    for (wi, (ts, te, tst_s, tst_e)) in WINDOWS.iter().enumerate() {
        eprintln!("\nWindow {}: train={}-{} test={}-{}", wi+1, ts, te, tst_s, tst_e);

        // Baseline simulation (no split)
        let (train_bl_pnl, _, _, _) = simulate(&coin_data, false, *ts, *te);
        let (test_bl_pnl, bl_wins, bl_losses, _) = simulate(&coin_data, false, *tst_s, *tst_e);

        // Config simulation (with partial split)
        let (train_cfg_pnl, _, _, _) = simulate(&coin_data, true, *ts, *te);
        let (test_cfg_pnl, cfg_wins, cfg_losses, test_trades) = simulate(&coin_data, true, *tst_s, *tst_e);

        let train_delta = train_cfg_pnl - train_bl_pnl;
        let test_delta = test_cfg_pnl - test_bl_pnl;
        let bl_wr = if bl_wins + bl_losses > 0 { bl_wins as f64 / (bl_wins + bl_losses) as f64 * 100.0 } else { 0.0 };
        let cfg_wr = if cfg_wins + cfg_losses > 0 { cfg_wins as f64 / (cfg_wins + cfg_losses) as f64 * 100.0 } else { 0.0 };
        let pass = test_delta > 0.0;

        eprintln!("  Train Δ: {:+.2}  Test Δ: {:+.2}  (baseline test: {:+.2}, cfg test: {:+.2})",
            train_delta, test_delta, test_bl_pnl, test_cfg_pnl);
        eprintln!("  Test WR: baseline {:.1}%  cfg {:.1}%  Trades: {}",
            bl_wr, cfg_wr, test_trades);

        results.push(WfWin {
            window: wi + 1,
            train_pnl_baseline: train_bl_pnl,
            train_pnl_best: train_cfg_pnl,
            train_delta,
            test_pnl_baseline: test_bl_pnl,
            test_pnl_best: test_cfg_pnl,
            test_delta,
            test_wr_baseline: bl_wr,
            test_wr_best: cfg_wr,
            test_trades,
            pass,
        });
    }

    let avg_delta: f64 = results.iter().map(|r| r.test_delta).sum::<f64>() / results.len() as f64;
    let positive_windows = results.iter().filter(|r| r.test_delta > 0.0).count();
    let verdict = if avg_delta > 0.0 && positive_windows >= 2 { "POSITIVE" } else { "NEGATIVE" };

    println!("\n=== RUN101.2 Walk-Forward Summary ===");
    println!("Best config: SL_MULT={:.1} SAT_Z_EXIT={:.1}", SL_MULT, SAT_Z_EXIT);
    println!("\n{:>3}  {:>10}  {:>10}  {:>8}  {:>10}  {:>10}  {:>8}  {:>6}",
        "Win", "TrainBL", "TrainCfg", "TrainΔ", "TestBL", "TestCfg", "TestΔ", "Trades");
    println!("{}", "-".repeat(80));
    for r in &results {
        println!("  {}  {:>+10.2}  {:>+10.2}  {:>+8.2}  {:>+10.2}  {:>+10.2}  {:>+8.2}  {:>6}",
            r.window, r.train_pnl_baseline, r.train_pnl_best, r.train_delta,
            r.test_pnl_baseline, r.test_pnl_best, r.test_delta, r.test_trades);
    }
    println!("{}", "-".repeat(80));
    println!("Avg Δ: {:+.2}  Positive windows: {}/{}", avg_delta, positive_windows, results.len());
    println!("VERDICT: {}", verdict);

    let output = Output {
        best_config: format!("SL{:.1}_SZ{:.1}", SL_MULT, SAT_Z_EXIT),
        sl_mult: SL_MULT,
        sat_z_exit: SAT_Z_EXIT,
        windows: results,
    };
    std::fs::write("/home/scamarena/ProjectCoin/run101_2_results.json",
        &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run101_2_results.json");
}
