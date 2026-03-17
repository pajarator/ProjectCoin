/// RUN15b (proper) — Scalp NN Filter on 1m Data
///
/// Tests whether a Logistic Regression trained on 9 indicator features
/// can gate COINCLAW scalp signals (vol_spike_rev + stoch_cross) to
/// improve win rate on out-of-sample data.
///
/// Fixes vs original Python RUN15b:
///   ✓ Full 1-year 1m dataset (~526k bars, not 30k = 21 days)
///   ✓ Proper trade simulation: TP=0.80% / SL=0.10% via OHLCV bars
///   ✓ Fees (0.1%) + slippage (0.05%) included in win/loss classification
///   ✓ Pre-specified thresholds (0.55, 0.60) — not chosen on test data
///   ✓ StandardScaler fit on train only, applied to test
///   ✓ Separate long/short LR models (matching Python approach)
///   ✓ 10 coins in parallel with Rayon
///
/// Output: run15b_results.json

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rayon::prelude::*;
use serde::Serialize;

use crate::indicators::{bollinger, rolling_max, rolling_mean, rolling_min, rsi};

// ── Scalp constants ───────────────────────────────────────────────────────────
const TP_PCT:    f64 = 0.008;   // 0.80%
const SL_PCT:    f64 = 0.001;   // 0.10%
const MAX_HOLD:  usize = 60;    // 60 bars = 60 minutes
const FEE:       f64 = 0.001;   // 0.1% per side
const SLIP:      f64 = 0.0005;  // 0.05% per side

const THRESH_55: f64 = 0.55;
const THRESH_60: f64 = 0.60;

const SCALP_COINS: [&str; 10] = [
    "BTC", "ETH", "BNB", "SOL", "ADA", "XRP", "DOGE", "LTC", "LINK", "DOT",
];

// ── 1m bar ────────────────────────────────────────────────────────────────────
#[derive(Clone)]
struct Bar1m {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    v: f64,
    hour: u8,
}

fn load_1m(coin: &str) -> Vec<Bar1m> {
    let path = format!(
        "/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_1year.csv",
        coin
    );
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let mut bars = Vec::new();
    for line in data.lines().skip(1) {
        let p: Vec<&str> = line.splitn(7, ',').collect();
        if p.len() < 6 { continue; }
        // Timestamp format: "2025-03-14 12:34:00"
        let hour: u8 = p[0].get(11..13)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let o: f64 = p[1].parse().unwrap_or(f64::NAN);
        let h: f64 = p[2].parse().unwrap_or(f64::NAN);
        let l: f64 = p[3].parse().unwrap_or(f64::NAN);
        let c: f64 = p[4].parse().unwrap_or(f64::NAN);
        let v: f64 = p[5].parse().unwrap_or(f64::NAN);
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() || v.is_nan() {
            continue;
        }
        bars.push(Bar1m { o, h, l, c, v, hour });
    }
    bars
}

// ── Trade simulation ──────────────────────────────────────────────────────────
/// Returns net pnl% including fees and slippage. Positive = win.
fn simulate_trade(bars: &[Bar1m], entry_bar: usize, direction: i32) -> f64 {
    let n = bars.len();
    if entry_bar + 1 >= n { return -(FEE * 2.0 + SLIP * 2.0) * 100.0; }

    let entry_close = bars[entry_bar].c;
    let (entry_price, tp_price, sl_price) = if direction == 1 {
        let ep = entry_close * (1.0 + SLIP);
        (ep, ep * (1.0 + TP_PCT), ep * (1.0 - SL_PCT))
    } else {
        let ep = entry_close * (1.0 - SLIP);
        (ep, ep * (1.0 - TP_PCT), ep * (1.0 + SL_PCT))
    };

    let end = (entry_bar + 1 + MAX_HOLD).min(n);
    for i in (entry_bar + 1)..end {
        let b = &bars[i];
        if direction == 1 {
            if b.l <= sl_price {
                let exit = sl_price * (1.0 - SLIP);
                return (exit - entry_price) / entry_price * 100.0 - FEE * 200.0;
            }
            if b.h >= tp_price {
                let exit = tp_price * (1.0 - SLIP);
                return (exit - entry_price) / entry_price * 100.0 - FEE * 200.0;
            }
        } else {
            if b.h >= sl_price {
                let exit = sl_price * (1.0 + SLIP);
                return (entry_price - exit) / entry_price * 100.0 - FEE * 200.0;
            }
            if b.l <= tp_price {
                let exit = tp_price * (1.0 + SLIP);
                return (entry_price - exit) / entry_price * 100.0 - FEE * 200.0;
            }
        }
    }
    // Max hold expired
    let exit_close = bars[end - 1].c;
    if direction == 1 {
        let exit = exit_close * (1.0 - SLIP);
        (exit - entry_price) / entry_price * 100.0 - FEE * 200.0
    } else {
        let exit = exit_close * (1.0 + SLIP);
        (entry_price - exit) / entry_price * 100.0 - FEE * 200.0
    }
}

// ── Feature computation + signal generation ───────────────────────────────────
/// Signal: (entry_bar_idx, direction:  1=long / -1=short)
/// Features (9): rsi, vol_ratio, stoch_k, stoch_d, stoch_cross,
///               bb_position, roc_3, avg_body_3, hour_of_day
fn compute(bars: &[Bar1m]) -> (Vec<[f64; 9]>, Vec<(usize, i32)>) {
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
    let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();
    let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();
    let open:  Vec<f64> = bars.iter().map(|b| b.o).collect();

    let rsi14    = rsi(&close, 14);
    let vol_ma20 = rolling_mean(&vol, 20);
    let low14    = rolling_min(&low, 14);
    let high14   = rolling_max(&high, 14);
    let (bb_upper, _bb_mid, bb_lower) = bollinger(&close, 20, 2.0);

    // Stochastic K
    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n {
        let range = high14[i] - low14[i];
        if !high14[i].is_nan() && range > 0.0 {
            stoch_k[i] = 100.0 * (close[i] - low14[i]) / range;
        }
    }
    // Stochastic D = SMA(K, 3) — computed manually to avoid NaN-propagation
    // in rolling_sum when the first 13 bars of stoch_k are NaN.
    let mut stoch_d = vec![f64::NAN; n];
    for i in 2..n {
        if !stoch_k[i].is_nan() && !stoch_k[i-1].is_nan() && !stoch_k[i-2].is_nan() {
            stoch_d[i] = (stoch_k[i] + stoch_k[i-1] + stoch_k[i-2]) / 3.0;
        }
    }

    // Body size (absolute, normalised by close)
    let body: Vec<f64> = (0..n).map(|i| (close[i] - open[i]).abs() / close[i]).collect();
    let body_ma3 = rolling_mean(&body, 3);

    // Pre-compute vol_ratio
    let vol_ratio: Vec<f64> = (0..n).map(|i| {
        if vol_ma20[i].is_nan() || vol_ma20[i] == 0.0 { f64::NAN }
        else { vol[i] / vol_ma20[i] }
    }).collect();

    // Warmup: need at least 20 bars for all indicators, plus 3 for roc
    let warmup = 23usize;
    let mut features: Vec<[f64; 9]> = vec![[0.0; 9]; n];
    let mut signals: Vec<(usize, i32)> = Vec::new();

    for i in warmup..n.saturating_sub(1) {
        if rsi14[i].is_nan() || vol_ratio[i].is_nan()
            || stoch_k[i].is_nan() || stoch_d[i].is_nan()
            || bb_upper[i].is_nan() || bb_lower[i].is_nan()
            || body_ma3[i].is_nan()
        { continue; }

        let roc3 = if close[i - 3] > 0.0 {
            (close[i] - close[i - 3]) / close[i - 3]
        } else { continue };

        let bb_range = bb_upper[i] - bb_lower[i];
        let bb_pos = if bb_range > 0.0 {
            (close[i] - bb_lower[i]) / bb_range
        } else { 0.5 };

        let sk = stoch_k[i];
        let sd = stoch_d[i];
        let stoch_cross_val = if sk > sd { 1.0 } else if sk < sd { -1.0 } else { 0.0 };

        features[i] = [
            rsi14[i],
            vol_ratio[i],
            sk,
            sd,
            stoch_cross_val,
            bb_pos,
            roc3,
            body_ma3[i],
            bars[i].hour as f64,
        ];

        // Signal detection
        let vr = vol_ratio[i];
        let prev_sk = stoch_k[i - 1];
        let prev_sd = stoch_d[i - 1];

        // vol_spike_long: vol > 3.5x AND rsi < 20
        if vr > 3.5 && rsi14[i] < 20.0 {
            signals.push((i, 1));
        }
        // vol_spike_short: vol > 3.5x AND rsi > 80
        else if vr > 3.5 && rsi14[i] > 80.0 {
            signals.push((i, -1));
        }
        // stoch_cross_long: k crosses above d, both < 20
        else if !prev_sk.is_nan() && !prev_sd.is_nan()
            && prev_sk <= prev_sd && sk > sd && sk < 20.0
        {
            signals.push((i, 1));
        }
        // stoch_cross_short: k crosses below d, both > 80
        else if !prev_sk.is_nan() && !prev_sd.is_nan()
            && prev_sk >= prev_sd && sk < sd && sk > 80.0
        {
            signals.push((i, -1));
        }
    }

    (features, signals)
}

// ── Logistic Regression ───────────────────────────────────────────────────────
const N_FEAT: usize = 9;
const LR: f64 = 0.05;
const MAX_ITER: usize = 500;
const LAMBDA: f64 = 10.0; // C=0.1 → lambda=1/C=10

#[derive(Clone)]
struct LR {
    w: [f64; N_FEAT],
    b: f64,
}

impl LR {
    fn new() -> Self { Self { w: [0.0; N_FEAT], b: 0.0 } }

    fn sigmoid(x: f64) -> f64 { 1.0 / (1.0 + (-x).exp()) }

    fn predict(&self, x: &[f64; N_FEAT]) -> f64 {
        let z: f64 = self.b + self.w.iter().zip(x).map(|(w, xi)| w * xi).sum::<f64>();
        Self::sigmoid(z)
    }

    fn fit(&mut self, xs: &[[f64; N_FEAT]], ys: &[f64]) {
        let n = xs.len();
        if n == 0 { return; }
        let nf = n as f64;

        for _ in 0..MAX_ITER {
            let mut grad_w = [0.0f64; N_FEAT];
            let mut grad_b = 0.0f64;

            for i in 0..n {
                let p = self.predict(&xs[i]);
                let err = p - ys[i];
                grad_b += err;
                for j in 0..N_FEAT {
                    grad_w[j] += err * xs[i][j];
                }
            }

            self.b -= LR * grad_b / nf;
            for j in 0..N_FEAT {
                self.w[j] -= LR * (grad_w[j] / nf + LAMBDA * self.w[j]);
            }
        }
    }
}

// ── Standardiser ─────────────────────────────────────────────────────────────
struct Scaler {
    mean: [f64; N_FEAT],
    std:  [f64; N_FEAT],
}

impl Scaler {
    fn fit(xs: &[[f64; N_FEAT]]) -> Self {
        let n = xs.len() as f64;
        let mut mean = [0.0f64; N_FEAT];
        let mut std  = [1.0f64; N_FEAT];
        if xs.is_empty() { return Self { mean, std }; }
        for j in 0..N_FEAT {
            mean[j] = xs.iter().map(|x| x[j]).sum::<f64>() / n;
        }
        for j in 0..N_FEAT {
            let var = xs.iter().map(|x| (x[j] - mean[j]).powi(2)).sum::<f64>() / n;
            std[j] = var.sqrt().max(1e-8);
        }
        Self { mean, std }
    }

    fn transform(&self, x: &[f64; N_FEAT]) -> [f64; N_FEAT] {
        let mut out = [0.0f64; N_FEAT];
        for j in 0..N_FEAT {
            out[j] = (x[j] - self.mean[j]) / self.std[j];
        }
        out
    }
}

// ── Per-threshold stats ───────────────────────────────────────────────────────
fn eval(pnls: &[f64]) -> (f64, f64, f64) {
    // returns (win_rate, profit_factor, total_pnl)
    if pnls.is_empty() { return (0.0, 0.0, 0.0); }
    let wins: Vec<f64>   = pnls.iter().cloned().filter(|&p| p > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().cloned().filter(|&p| p <= 0.0).collect();
    let wr = wins.len() as f64 / pnls.len() as f64 * 100.0;
    let tw: f64 = wins.iter().sum();
    let tl: f64 = losses.iter().map(|&p| -p).sum();
    let pf = if tl > 0.0 { tw / tl } else { tw };
    let total_pnl: f64 = pnls.iter().sum();
    (wr, pf, total_pnl)
}

// ── Result structs ────────────────────────────────────────────────────────────
#[derive(Debug, Serialize)]
struct ThreshResult {
    n_trades: usize,
    win_rate: f64,
    profit_factor: f64,
    total_pnl: f64,
    pct_of_binary: f64,
}

#[derive(Debug, Serialize)]
struct CoinResult {
    coin: String,
    total_signals: usize,
    train_signals: usize,
    test_signals: usize,
    binary: ThreshResult,
    bay55: ThreshResult,
    bay60: ThreshResult,
}

#[derive(Debug, Serialize)]
struct Summary {
    n_coins: usize,
    binary_avg_wr: f64,
    bay55_avg_wr: f64,
    bay60_avg_wr: f64,
    binary_avg_pf: f64,
    bay55_avg_pf: f64,
    bay60_avg_pf: f64,
    binary_total_pnl: f64,
    bay55_total_pnl: f64,
    bay60_total_pnl: f64,
    binary_total_trades: usize,
    bay55_total_trades: usize,
    bay60_total_trades: usize,
    coins_bay55_beats_wr: usize,
    coins_bay55_beats_pf: usize,
    coins_bay60_beats_wr: usize,
    coins_bay60_beats_pf: usize,
}

#[derive(Debug, Serialize)]
struct Output {
    experiment: String,
    tp_pct: f64,
    sl_pct: f64,
    max_hold_bars: usize,
    fee_pct: f64,
    slip_pct: f64,
    per_coin: Vec<CoinResult>,
    summary: Summary,
}

// ── Main ─────────────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN15b — Scalp NN Filter (proper Rust implementation)");
    println!("================================================================");
    println!("Trade: TP={:.2}% SL={:.2}% MaxHold={}bars Fee={:.2}% Slip={:.3}%",
             TP_PCT * 100.0, SL_PCT * 100.0, MAX_HOLD, FEE * 100.0, SLIP * 100.0);
    println!("Signals: vol_spike_rev + stoch_cross | Split: 50/50 OOS");
    println!("Thresholds: {:.0}% and {:.0}% (pre-specified, not tuned on test)",
             THRESH_55 * 100.0, THRESH_60 * 100.0);
    println!();

    let out_path = "run15b_results.json";
    if std::path::Path::new(out_path).exists() {
        println!("Results already exist at {}. Delete to re-run.", out_path);
        return;
    }

    let per_coin: Vec<CoinResult> = SCALP_COINS
        .par_iter()
        .filter_map(|&coin| {
            if shutdown.load(Ordering::SeqCst) { return None; }

            let bars = load_1m(coin);
            if bars.is_empty() {
                eprintln!("  SKIP {}: no 1m data", coin);
                return None;
            }

            let (features, signals) = compute(&bars);
            if signals.len() < 100 {
                eprintln!("  SKIP {}: only {} signals", coin, signals.len());
                return None;
            }

            // 50/50 split on signal index (chronological)
            let split = signals.len() / 2;
            let train_sigs = &signals[..split];
            let test_sigs  = &signals[split..];

            // ── Train: collect (feature, direction, outcome) ───────────────
            let (train_long_x, train_long_y,
                 train_short_x, train_short_y) = {
                let mut lx: Vec<[f64; N_FEAT]> = Vec::new();
                let mut ly: Vec<f64> = Vec::new();
                let mut sx: Vec<[f64; N_FEAT]> = Vec::new();
                let mut sy: Vec<f64> = Vec::new();

                for &(bar, dir) in train_sigs {
                    let pnl = simulate_trade(&bars, bar, dir);
                    let win = if pnl > 0.0 { 1.0 } else { 0.0 };
                    if dir == 1 {
                        lx.push(features[bar]);
                        ly.push(win);
                    } else {
                        sx.push(features[bar]);
                        sy.push(win);
                    }
                }
                (lx, ly, sx, sy)
            };

            // StandardScaler fit on train
            let scaler_long  = Scaler::fit(&train_long_x);
            let scaler_short = Scaler::fit(&train_short_x);

            let train_long_xs:  Vec<[f64; N_FEAT]> = train_long_x.iter().map(|x| scaler_long.transform(x)).collect();
            let train_short_xs: Vec<[f64; N_FEAT]> = train_short_x.iter().map(|x| scaler_short.transform(x)).collect();

            // Train models
            let mut model_long  = LR::new();
            let mut model_short = LR::new();
            model_long.fit(&train_long_xs, &train_long_y);
            model_short.fit(&train_short_xs, &train_short_y);

            // ── Test: backtest all three modes ─────────────────────────────
            let mut bin_pnl:  Vec<f64> = Vec::new();
            let mut b55_pnl:  Vec<f64> = Vec::new();
            let mut b60_pnl:  Vec<f64> = Vec::new();

            for &(bar, dir) in test_sigs {
                let pnl = simulate_trade(&bars, bar, dir);
                bin_pnl.push(pnl);

                let feat_scaled = if dir == 1 {
                    scaler_long.transform(&features[bar])
                } else {
                    scaler_short.transform(&features[bar])
                };
                let prob = if dir == 1 {
                    model_long.predict(&feat_scaled)
                } else {
                    model_short.predict(&feat_scaled)
                };

                if prob > THRESH_55 { b55_pnl.push(pnl); }
                if prob > THRESH_60 { b60_pnl.push(pnl); }
            }

            let (bin_wr, bin_pf, bin_total) = eval(&bin_pnl);
            let (b55_wr, b55_pf, b55_total) = eval(&b55_pnl);
            let (b60_wr, b60_pf, b60_total) = eval(&b60_pnl);
            let n_test = bin_pnl.len();

            println!(
                "  {:6}  sigs={:6}  \
                 | bin  {:5.1}%WR PF={:.2} ({:5}t) \
                 | b55  {:5.1}%WR PF={:.2} ({:4}t,{:3.0}%) \
                 | b60  {:5.1}%WR PF={:.2} ({:4}t,{:3.0}%)",
                coin, signals.len(),
                bin_wr, bin_pf, n_test,
                b55_wr, b55_pf, b55_pnl.len(),
                if n_test > 0 { b55_pnl.len() as f64 / n_test as f64 * 100.0 } else { 0.0 },
                b60_wr, b60_pf, b60_pnl.len(),
                if n_test > 0 { b60_pnl.len() as f64 / n_test as f64 * 100.0 } else { 0.0 },
            );

            let pct55 = if n_test > 0 { b55_pnl.len() as f64 / n_test as f64 * 100.0 } else { 0.0 };
            let pct60 = if n_test > 0 { b60_pnl.len() as f64 / n_test as f64 * 100.0 } else { 0.0 };

            Some(CoinResult {
                coin: coin.to_string(),
                total_signals: signals.len(),
                train_signals: split,
                test_signals: test_sigs.len(),
                binary: ThreshResult { n_trades: n_test, win_rate: bin_wr, profit_factor: bin_pf, total_pnl: bin_total, pct_of_binary: 100.0 },
                bay55: ThreshResult { n_trades: b55_pnl.len(), win_rate: b55_wr, profit_factor: b55_pf, total_pnl: b55_total, pct_of_binary: pct55 },
                bay60: ThreshResult { n_trades: b60_pnl.len(), win_rate: b60_wr, profit_factor: b60_pf, total_pnl: b60_total, pct_of_binary: pct60 },
            })
        })
        .collect();

    if per_coin.is_empty() {
        eprintln!("No coins processed.");
        return;
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    let n = per_coin.len() as f64;
    let avg = |v: Vec<f64>| v.iter().sum::<f64>() / n;

    let summary = Summary {
        n_coins: per_coin.len(),
        binary_avg_wr:  avg(per_coin.iter().map(|r| r.binary.win_rate).collect()),
        bay55_avg_wr:   avg(per_coin.iter().map(|r| r.bay55.win_rate).collect()),
        bay60_avg_wr:   avg(per_coin.iter().map(|r| r.bay60.win_rate).collect()),
        binary_avg_pf:  avg(per_coin.iter().map(|r| r.binary.profit_factor).collect()),
        bay55_avg_pf:   avg(per_coin.iter().map(|r| r.bay55.profit_factor).collect()),
        bay60_avg_pf:   avg(per_coin.iter().map(|r| r.bay60.profit_factor).collect()),
        binary_total_pnl:  per_coin.iter().map(|r| r.binary.total_pnl).sum(),
        bay55_total_pnl:   per_coin.iter().map(|r| r.bay55.total_pnl).sum(),
        bay60_total_pnl:   per_coin.iter().map(|r| r.bay60.total_pnl).sum(),
        binary_total_trades: per_coin.iter().map(|r| r.binary.n_trades).sum(),
        bay55_total_trades:  per_coin.iter().map(|r| r.bay55.n_trades).sum(),
        bay60_total_trades:  per_coin.iter().map(|r| r.bay60.n_trades).sum(),
        coins_bay55_beats_wr: per_coin.iter().filter(|r| r.bay55.win_rate > r.binary.win_rate).count(),
        coins_bay55_beats_pf: per_coin.iter().filter(|r| r.bay55.profit_factor > r.binary.profit_factor).count(),
        coins_bay60_beats_wr: per_coin.iter().filter(|r| r.bay60.win_rate > r.binary.win_rate).count(),
        coins_bay60_beats_pf: per_coin.iter().filter(|r| r.bay60.profit_factor > r.binary.profit_factor).count(),
    };

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("SUMMARY (OOS test — second 50% of signals per coin)");
    let bin_t = summary.binary_total_trades;
    println!("  {:8}  Avg WR: {:4.1}%  Avg PF: {:.3}  PnL: {:+.1}%  Trades: {}",
             "Binary", summary.binary_avg_wr, summary.binary_avg_pf,
             summary.binary_total_pnl, bin_t);
    println!("  {:8}  Avg WR: {:4.1}%  Avg PF: {:.3}  PnL: {:+.1}%  Trades: {} ({:.0}% of bin)",
             "Bay>55%", summary.bay55_avg_wr, summary.bay55_avg_pf,
             summary.bay55_total_pnl, summary.bay55_total_trades,
             summary.bay55_total_trades as f64 / bin_t as f64 * 100.0);
    println!("  {:8}  Avg WR: {:4.1}%  Avg PF: {:.3}  PnL: {:+.1}%  Trades: {} ({:.0}% of bin)",
             "Bay>60%", summary.bay60_avg_wr, summary.bay60_avg_pf,
             summary.bay60_total_pnl, summary.bay60_total_trades,
             summary.bay60_total_trades as f64 / bin_t as f64 * 100.0);
    println!();
    println!("  Coins Bay>55% beats Binary (WR): {}/{}", summary.coins_bay55_beats_wr, per_coin.len());
    println!("  Coins Bay>55% beats Binary (PF): {}/{}", summary.coins_bay55_beats_pf, per_coin.len());
    println!("  Coins Bay>60% beats Binary (WR): {}/{}", summary.coins_bay60_beats_wr, per_coin.len());
    println!("  Coins Bay>60% beats Binary (PF): {}/{}", summary.coins_bay60_beats_pf, per_coin.len());

    let output = Output {
        experiment: "RUN15b Scalp NN Filter — proper Rust implementation".to_string(),
        tp_pct: TP_PCT * 100.0,
        sl_pct: SL_PCT * 100.0,
        max_hold_bars: MAX_HOLD,
        fee_pct: FEE * 100.0,
        slip_pct: SLIP * 100.0,
        per_coin,
        summary,
    };

    let json = serde_json::to_string_pretty(&output).expect("JSON");
    std::fs::write(out_path, json).expect("write results");
    println!();
    println!("Results saved to {}", out_path);
}
