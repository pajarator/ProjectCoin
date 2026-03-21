/// RUN75.2 — Sharpe-Weighted Capital Allocation: Walk-Forward Validation
///
/// 3 windows: (0-5760 train/5760-8640 test), (2880-8640 train/8640-11520 test), (5760-11520 train/11520-14400 test)
/// Compare: best grid config vs baseline
///
/// Run: cargo run --release --features run75_2 -- --run75-2

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const BASE_POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;
const SL_PCT: f64 = 0.003;
const COOLDOWN: usize = 2;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct SharpeCfg {
    freq: usize,
    window: usize,
    min_cap_mult: f64,
    max_cap_mult: f64,
}

impl SharpeCfg {
    fn label(&self) -> String {
        format!("F{}_W{}_MIN{:.2}_MAX{:.1}", self.freq, self.window, self.min_cap_mult, self.max_cap_mult)
    }
}

const BEST_CFG: SharpeCfg = SharpeCfg {
    freq: 336,
    window: 10,
    min_cap_mult: 0.50,
    max_cap_mult: 3.0,
};

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct WindowResult {
    window: usize,
    train_pnl: f64,
    test_pnl: f64,
    test_delta: f64,
    test_wr: f64,
    test_trades: usize,
}
#[derive(Serialize)]
struct Output { notes: String, windows: Vec<WindowResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut closes = Vec::new();
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

fn trailing_sharpe(pnl_pcts: &[f64], window: usize) -> f64 {
    if pnl_pcts.len() < 2 { return 0.0; }
    let start = pnl_pcts.len().saturating_sub(window);
    let trades = &pnl_pcts[start..];
    if trades.len() < 2 { return 0.0; }
    let mean: f64 = trades.iter().sum::<f64>() / trades.len() as f64;
    let variance: f64 = trades.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / trades.len() as f64;
    let std = variance.sqrt();
    if std == 0.0 { return 0.0; }
    let annualized_mean = mean * 35040.0;
    let annualized_std = std * (35040.0_f64.sqrt());
    annualized_mean / annualized_std
}

fn simulate_portfolio(data: &[CoinData15m], cfg: &SharpeCfg, start: usize, end: usize) -> (f64, usize, usize, usize) {
    let n = end.min(data[0].closes.len());
    if start >= n { return (0.0, 0, 0, 0); }

    let mut bals: Vec<f64> = vec![INITIAL_BAL; N_COINS];
    let mut pos: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldown: Vec<usize> = vec![0; N_COINS];
    let mut trade_pnls: Vec<Vec<f64>> = vec![Vec::new(); N_COINS];
    let mut last_rebalance_bar: Vec<usize> = vec![0; N_COINS];
    let mut total_wins = 0usize;
    let mut total_losses = 0usize;
    let mut total_flats = 0usize;

    for i in start..n {
        for ci in 0..N_COINS {
            let d = &data[ci];
            if let Some((dir, entry)) = pos[ci] {
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
                    let bal = bals[ci];
                    let net = bal * BASE_POSITION_SIZE * LEVERAGE * exit_pct;
                    bals[ci] += net;
                    if net > 1e-10 { total_wins += 1; }
                    else if net < -1e-10 { total_losses += 1; }
                    else { total_flats += 1; }
                    trade_pnls[ci].push(exit_pct);
                    pos[ci] = None;
                    cooldown[ci] = COOLDOWN;
                }
            } else if cooldown[ci] > 0 {
                cooldown[ci] -= 1;
            }
        }

        let is_baseline = cfg.freq == 0;
        if !is_baseline {
            let bars_since_first = i.saturating_sub(last_rebalance_bar[0]);
            if bars_since_first >= cfg.freq {
                let mut sharpes: Vec<f64> = Vec::new();
                for ci in 0..N_COINS {
                    sharpes.push(trailing_sharpe(&trade_pnls[ci], cfg.window));
                }
                let sum_sharpe: f64 = sharpes.iter().sum();
                if sum_sharpe > 0.0 && sum_sharpe.is_finite() {
                    let total_portfolio: f64 = bals.iter().sum();
                    let base_cap = INITIAL_BAL;
                    let mut targets: Vec<f64> = Vec::new();
                    for &sh in &sharpes {
                        let weight = sh / sum_sharpe;
                        let target = (total_portfolio * weight).max(base_cap * cfg.min_cap_mult).min(base_cap * cfg.max_cap_mult);
                        targets.push(target);
                    }
                    for ci in 0..N_COINS {
                        let diff = targets[ci] - bals[ci];
                        bals[ci] += diff;
                    }
                }
                for ci in 0..N_COINS { last_rebalance_bar[ci] = i; }
            }
        }

        for ci in 0..N_COINS {
            if cooldown[ci] > 0 || pos[ci].is_some() { continue; }
            let d = &data[ci];
            if let Some(dir) = regime_signal(d.zscore[i]) {
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 { pos[ci] = Some((dir, entry_price)); }
                }
            }
        }
    }

    let total_pnl: f64 = bals.iter().map(|b| b - INITIAL_BAL).sum();
    (total_pnl, total_wins, total_losses, total_flats)
}

impl SharpeCfg {
    fn is_baseline(&self) -> bool { self.freq == 0 }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN75.2 — Sharpe Allocation Walk-Forward\n");
    eprintln!("Loading 15m data...");
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        raw_data.push(load_15m(name));
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    // Windows: (train_start, train_end, test_start, test_end)
    let windows = vec![
        (0, 5760, 5760, 8640),
        (2880, 8640, 8640, 11520),
        (5760, 11520, 11520, 14400),
    ];

    let baseline_cfg = SharpeCfg { freq: 0, window: 0, min_cap_mult: 0.0, max_cap_mult: 0.0 };
    let cfg = BEST_CFG;

    let mut results: Vec<WindowResult> = Vec::new();

    for (wi, (ts, te, tst_s, tst_e)) in windows.iter().enumerate() {
        eprintln!("\nWindow {}: train={}-{} test={}-{}", wi+1, ts, te, tst_s, tst_e);

        let (train_pnl, _, _, _) = simulate_portfolio(&coin_data, &cfg, *ts, *te);
        let (test_pnl, wins, losses, flats) = simulate_portfolio(&coin_data, &cfg, *tst_s, *tst_e);
        let (bl_test_pnl, _, _, _) = simulate_portfolio(&coin_data, &baseline_cfg, *tst_s, *tst_e);

        let test_delta = test_pnl - bl_test_pnl;
        let test_trades = wins + losses + flats;
        let test_wr = if test_trades > 0 { wins as f64 / test_trades as f64 * 100.0 } else { 0.0 };

        eprintln!("  Train PnL: {:+.2}", train_pnl);
        eprintln!("  Test PnL (config): {:+.2}", test_pnl);
        eprintln!("  Test PnL (baseline): {:+.2}", bl_test_pnl);
        eprintln!("  Delta: {:+.2}", test_delta);
        eprintln!("  Test WR: {:.1}%  Trades: {}", test_wr, test_trades);

        results.push(WindowResult {
            window: wi + 1,
            train_pnl,
            test_pnl,
            test_delta,
            test_wr,
            test_trades,
        });
    }

    let avg_delta: f64 = results.iter().map(|r| r.test_delta).sum::<f64>() / results.len() as f64;
    let positive_windows = results.iter().filter(|r| r.test_delta > 0.0).count();
    let verdict = if avg_delta > 0.0 && positive_windows >= 2 { "POSITIVE" } else { "NEGATIVE" };

    println!("\n=== RUN75.2 Walk-Forward Summary ===");
    println!("Best config: {}", cfg.label());
    println!("\n{:>3}  {:>8}  {:>8}  {:>8}  {:>6}  {:>7}",
        "Win", "TrainPnL", "TestPnL", "ΔvsBL", "WR%", "Trades");
    println!("{}", "-".repeat(50));
    for r in &results {
        println!("  {}  {:>+8.2}  {:>+8.2}  {:>+8.2}  {:>5.1}%  {:>6}",
            r.window, r.train_pnl, r.test_pnl, r.test_delta, r.test_wr, r.test_trades);
    }
    println!("{}", "-".repeat(50));
    println!("Avg Δ: {:+.2}  Positive windows: {}/{}", avg_delta, positive_windows, results.len());
    println!("VERDICT: {}", verdict);

    let notes = format!("RUN75.2 walk-forward. Best: {} avg_delta={:.2} pos={}/{}",
        cfg.label(), avg_delta, positive_windows, results.len());
    let output = Output { notes, windows: results };
    std::fs::write("/home/scamarena/ProjectCoin/run75_2_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run75_2_results.json");
}
