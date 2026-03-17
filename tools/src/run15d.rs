/// RUN15d — Pre-signal filter comparison for COINCLAW scalp signals
///
/// Tests whether filtering signals BEFORE the NN (Logistic Regression) gate
/// improves actual trade quality:
///   · Baseline        — all vol_spike_rev + stoch_cross signals
///   · Time-of-Day     — signals only during UTC hours 14–21 (NYSE session)
///   · Liquidity       — signals only when vol_ratio ≥ 2.0
///
/// Each filter variant tested WITHOUT and WITH LR(9-feature) gate at threshold 0.55.
/// = 6 result columns per coin.
///
/// Proper methodology (same as run15b/c):
///   ✓ Full 1-year 1m data per coin
///   ✓ TP=0.80% / SL=0.10% via OHLCV bars
///   ✓ Fees (0.1%) + slippage (0.05%)
///   ✓ 50/50 chronological split
///   ✓ Pre-specified threshold 0.55

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rayon::prelude::*;
use serde::Serialize;

use crate::indicators::{bollinger, rsi, rolling_mean, rolling_min, rolling_max};

// ── Constants ─────────────────────────────────────────────────────────────────
const TP_PCT:   f64   = 0.008;
const SL_PCT:   f64   = 0.001;
const MAX_HOLD: usize = 60;
const FEE:      f64   = 0.001;
const SLIP:     f64   = 0.0005;
const THRESH:   f64   = 0.55;

const LEARN_RATE: f64   = 0.05;
const LR_ITER:    usize = 500;
const LR_LAM:     f64   = 10.0;
const N_FEAT:     usize = 9;

// Time-of-day filter: UTC 14–21 covers NYSE session (9:30 AM – 4 PM ET ≈ 14:30–21:00 UTC)
const TOD_MIN: u8 = 14;
const TOD_MAX: u8 = 21;

// Liquidity filter: signal bar must have vol_ratio ≥ 2.0
const LIQ_MIN: f64 = 2.0;

const SCALP_COINS: [&str; 10] = [
    "BTC", "ETH", "BNB", "SOL", "ADA", "XRP", "DOGE", "LTC", "LINK", "DOT",
];
const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const OUT_PATH: &str =
    "/home/scamarena/ProjectCoin/archive/RUN15d/run15d_corrected_results.json";

// ── Bar1m ─────────────────────────────────────────────────────────────────────
#[derive(Clone)]
struct Bar1m { o: f64, h: f64, l: f64, c: f64, v: f64, hour: u8 }

fn load_1m(coin: &str) -> Vec<Bar1m> {
    let path = format!("{}/{}_USDT_1m_1year.csv", DATA_DIR, coin);
    let data = match std::fs::read_to_string(&path) { Ok(d) => d, Err(_) => return vec![] };
    let mut bars = Vec::new();
    for line in data.lines().skip(1) {
        let p: Vec<&str> = line.splitn(7, ',').collect();
        if p.len() < 6 { continue; }
        let hour: u8 = p[0].get(11..13).and_then(|s| s.parse().ok()).unwrap_or(0);
        let o: f64 = p[1].parse().unwrap_or(f64::NAN);
        let h: f64 = p[2].parse().unwrap_or(f64::NAN);
        let l: f64 = p[3].parse().unwrap_or(f64::NAN);
        let c: f64 = p[4].parse().unwrap_or(f64::NAN);
        let v: f64 = p[5].parse().unwrap_or(f64::NAN);
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() || v.is_nan() { continue; }
        bars.push(Bar1m { o, h, l, c, v, hour });
    }
    bars
}

// ── Trade simulation ──────────────────────────────────────────────────────────
fn simulate_trade(bars: &[Bar1m], entry_bar: usize, direction: i32) -> f64 {
    let n = bars.len();
    if entry_bar + 1 >= n { return -(FEE * 2.0 + SLIP * 2.0) * 100.0; }
    let ec = bars[entry_bar].c;
    let (ep, tp, sl) = if direction == 1 {
        let e = ec * (1.0 + SLIP);
        (e, e * (1.0 + TP_PCT), e * (1.0 - SL_PCT))
    } else {
        let e = ec * (1.0 - SLIP);
        (e, e * (1.0 - TP_PCT), e * (1.0 + SL_PCT))
    };
    let end = (entry_bar + 1 + MAX_HOLD).min(n);
    for i in (entry_bar + 1)..end {
        let b = &bars[i];
        if direction == 1 {
            if b.l <= sl { return (sl * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0; }
            if b.h >= tp { return (tp * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0; }
        } else {
            if b.h >= sl { return (ep - sl * (1.0 + SLIP)) / ep * 100.0 - FEE * 200.0; }
            if b.l <= tp { return (ep - tp * (1.0 + SLIP)) / ep * 100.0 - FEE * 200.0; }
        }
    }
    let ec2 = bars[end - 1].c;
    if direction == 1 { (ec2 * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0 }
    else              { (ep - ec2 * (1.0 + SLIP)) / ep * 100.0 - FEE * 200.0 }
}

// ── Feature computation + signal detection ────────────────────────────────────
/// Returns (features [N_FEAT], all_signals) for each bar.
/// Features [9]: [rsi14, vol_ratio, stoch_k, stoch_d, stoch_cross, bb_pos, roc3, body3, hour]
/// Signal: (bar_idx, direction: 1=long / -1=short) — no filter applied yet.
fn compute(bars: &[Bar1m]) -> (Vec<[f64; N_FEAT]>, Vec<(usize, i32)>) {
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

    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n {
        let r = high14[i] - low14[i];
        if !high14[i].is_nan() && r > 0.0 {
            stoch_k[i] = 100.0 * (close[i] - low14[i]) / r;
        }
    }
    // NaN-aware stoch_d (run15b fix)
    let mut stoch_d = vec![f64::NAN; n];
    for i in 2..n {
        if !stoch_k[i].is_nan() && !stoch_k[i-1].is_nan() && !stoch_k[i-2].is_nan() {
            stoch_d[i] = (stoch_k[i] + stoch_k[i-1] + stoch_k[i-2]) / 3.0;
        }
    }

    let body: Vec<f64> = (0..n).map(|i| (close[i] - open[i]).abs() / close[i].max(1e-12)).collect();
    let body_ma3 = rolling_mean(&body, 3);

    let vol_ratio: Vec<f64> = (0..n).map(|i| {
        if vol_ma20[i].is_nan() || vol_ma20[i] == 0.0 { f64::NAN }
        else { vol[i] / vol_ma20[i] }
    }).collect();

    let warmup = 23usize;
    let mut features: Vec<[f64; N_FEAT]> = vec![[0.0; N_FEAT]; n];
    let mut signals:  Vec<(usize, i32)>  = Vec::new();

    for i in warmup..n.saturating_sub(1) {
        if rsi14[i].is_nan() || vol_ratio[i].is_nan()
            || stoch_k[i].is_nan() || stoch_d[i].is_nan()
            || bb_upper[i].is_nan() || bb_lower[i].is_nan()
            || body_ma3[i].is_nan()
        { continue; }

        let roc3 = if close[i-3] > 0.0 { (close[i] - close[i-3]) / close[i-3] } else { continue };
        let bb_range = bb_upper[i] - bb_lower[i];
        let bb_pos   = if bb_range > 0.0 { (close[i] - bb_lower[i]) / bb_range } else { 0.5 };
        let sk = stoch_k[i]; let sd = stoch_d[i];
        let cross = if sk > sd { 1.0 } else if sk < sd { -1.0 } else { 0.0 };

        features[i] = [rsi14[i], vol_ratio[i], sk, sd, cross, bb_pos, roc3, body_ma3[i], bars[i].hour as f64];

        let vr = vol_ratio[i];
        let prev_sk = stoch_k[i-1]; let prev_sd = stoch_d[i-1];
        if vr > 3.5 && rsi14[i] < 20.0 {
            signals.push((i, 1));
        } else if vr > 3.5 && rsi14[i] > 80.0 {
            signals.push((i, -1));
        } else if !prev_sk.is_nan() && !prev_sd.is_nan()
            && prev_sk <= prev_sd && sk > sd && sk < 20.0
        {
            signals.push((i, 1));
        } else if !prev_sk.is_nan() && !prev_sd.is_nan()
            && prev_sk >= prev_sd && sk < sd && sk > 80.0
        {
            signals.push((i, -1));
        }
    }

    (features, signals)
}

// ── Pre-signal filters ─────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
enum Filter { Baseline, TimeOfDay, Liquidity }

/// Apply a pre-signal filter: return subset of signals passing the filter.
fn apply_filter(signals: &[(usize, i32)], bars: &[Bar1m], feats: &[[f64; N_FEAT]], f: Filter)
    -> Vec<(usize, i32)>
{
    signals.iter().cloned().filter(|&(bar, _)| {
        match f {
            Filter::Baseline  => true,
            Filter::TimeOfDay => {
                let h = bars[bar].hour;
                h >= TOD_MIN && h <= TOD_MAX
            }
            Filter::Liquidity => {
                let vr = feats[bar][1]; // vol_ratio is index 1
                !vr.is_nan() && vr >= LIQ_MIN
            }
        }
    }).collect()
}

// ── Standardiser ─────────────────────────────────────────────────────────────
struct Scaler { mean: [f64; N_FEAT], std: [f64; N_FEAT] }

impl Scaler {
    fn fit(xs: &[[f64; N_FEAT]]) -> Self {
        let n = xs.len() as f64;
        let mut mean = [0.0f64; N_FEAT];
        let mut std  = [1.0f64; N_FEAT];
        if xs.is_empty() { return Self { mean, std }; }
        for j in 0..N_FEAT { mean[j] = xs.iter().map(|x| x[j]).sum::<f64>() / n; }
        for j in 0..N_FEAT {
            let var = xs.iter().map(|x| (x[j] - mean[j]).powi(2)).sum::<f64>() / n;
            std[j] = var.sqrt().max(1e-8);
        }
        Self { mean, std }
    }
    fn transform(&self, x: &[f64; N_FEAT]) -> [f64; N_FEAT] {
        let mut out = [0.0f64; N_FEAT];
        for j in 0..N_FEAT { out[j] = (x[j] - self.mean[j]) / self.std[j]; }
        out
    }
}

// ── Logistic Regression ───────────────────────────────────────────────────────
#[derive(Clone)]
struct LogReg { w: [f64; N_FEAT], b: f64 }

impl LogReg {
    fn new() -> Self { Self { w: [0.0; N_FEAT], b: 0.0 } }
    fn sigmoid(x: f64) -> f64 { 1.0 / (1.0 + (-x.clamp(-500.0, 500.0)).exp()) }
    fn predict(&self, x: &[f64; N_FEAT]) -> f64 {
        let z: f64 = self.b + self.w.iter().zip(x.iter()).map(|(&w, &xi)| w * xi).sum::<f64>();
        Self::sigmoid(z)
    }
    fn fit(&mut self, xs: &[[f64; N_FEAT]], ys: &[f64]) {
        let n = xs.len(); if n == 0 { return; }
        let nf = n as f64;
        for _ in 0..LR_ITER {
            let mut dw = [0.0f64; N_FEAT];
            let mut db = 0.0f64;
            for (xi, &yi) in xs.iter().zip(ys.iter()) {
                let err = self.predict(xi) - yi;
                db += err;
                for j in 0..N_FEAT { dw[j] += err * xi[j]; }
            }
            self.b -= LEARN_RATE * db / nf;
            for j in 0..N_FEAT {
                self.w[j] -= LEARN_RATE * (dw[j] / nf + LR_LAM * self.w[j]);
            }
        }
    }
}

// ── Result structs ────────────────────────────────────────────────────────────
#[derive(Debug, Serialize, Clone)]
struct FilterResult {
    filter: String,
    binary_trades: usize,
    binary_wr: f64,
    binary_pf: f64,
    binary_pnl: f64,
    lr_trades: usize,
    lr_wr: f64,
    lr_pf: f64,
    lr_pnl: f64,
    lr_pct_of_binary: f64,
}

#[derive(Debug, Serialize)]
struct CoinResult {
    coin: String,
    total_signals: usize,
    baseline: FilterResult,
    tod: FilterResult,
    liquidity: FilterResult,
}

#[derive(Debug, Serialize, Clone)]
struct FilterSummary {
    filter: String,
    binary_avg_wr: f64,
    binary_avg_pf: f64,
    binary_total_trades: usize,
    lr_avg_wr: f64,
    lr_avg_pf: f64,
    lr_total_trades: usize,
    lr_pct_of_binary: f64,
}

#[derive(Debug, Serialize)]
struct Output {
    experiment: String,
    tp_pct: f64, sl_pct: f64, fee_pct: f64, slip_pct: f64,
    threshold: f64, max_hold_bars: usize,
    tod_hours: String,
    liq_vol_ratio_min: f64,
    per_coin: Vec<CoinResult>,
    summary: Vec<FilterSummary>,
}

// ── Evaluate one filter configuration ────────────────────────────────────────
fn eval_filter(
    filter: Filter,
    filter_name: &str,
    all_signals: &[(usize, i32)],
    bars: &[Bar1m],
    feats: &[[f64; N_FEAT]],
) -> FilterResult {
    let sigs = apply_filter(all_signals, bars, feats, filter);

    if sigs.len() < 50 {
        return FilterResult {
            filter: filter_name.to_string(),
            binary_trades: sigs.len(), binary_wr: 0.0, binary_pf: 0.0, binary_pnl: 0.0,
            lr_trades: 0, lr_wr: 0.0, lr_pf: 0.0, lr_pnl: 0.0, lr_pct_of_binary: 0.0,
        };
    }

    // Simulate all trades up front
    let all_pnls: Vec<f64> = sigs.iter().map(|&(bar, dir)| simulate_trade(bars, bar, dir)).collect();

    let total = sigs.len();
    let split = total / 2;
    let test_pnls = &all_pnls[split..];
    let n_test = test_pnls.len();

    // Binary stats (test half only)
    let (bin_wr, bin_pf, bin_pnl) = stats(test_pnls);

    // ── LR filter ─────────────────────────────────────────────────────────────
    let train_sigs = &sigs[..split];
    let test_sigs  = &sigs[split..];
    let train_pnls = &all_pnls[..split];

    // Collect train feature rows by direction
    let mut train_long_x:  Vec<[f64; N_FEAT]> = Vec::new();
    let mut train_long_y:  Vec<f64>            = Vec::new();
    let mut train_short_x: Vec<[f64; N_FEAT]>  = Vec::new();
    let mut train_short_y: Vec<f64>             = Vec::new();

    for (i, &(bar, dir)) in train_sigs.iter().enumerate() {
        let win = if train_pnls[i] > 0.0 { 1.0 } else { 0.0 };
        if dir == 1  { train_long_x.push(feats[bar]);  train_long_y.push(win); }
        else          { train_short_x.push(feats[bar]); train_short_y.push(win); }
    }

    // Scale + fit long model
    let scaler_long  = Scaler::fit(&train_long_x);
    let scaler_short = Scaler::fit(&train_short_x);

    let tl_xs: Vec<[f64; N_FEAT]> = train_long_x.iter().map(|x| scaler_long.transform(x)).collect();
    let ts_xs: Vec<[f64; N_FEAT]> = train_short_x.iter().map(|x| scaler_short.transform(x)).collect();

    let mut lr_long  = LogReg::new();
    let mut lr_short = LogReg::new();
    if train_long_x.len()  >= 20 { lr_long.fit(&tl_xs,  &train_long_y); }
    if train_short_x.len() >= 20 { lr_short.fit(&ts_xs, &train_short_y); }

    // Evaluate on test set
    let mut lr_pnls: Vec<f64> = Vec::new();
    for (i, &(bar, dir)) in test_sigs.iter().enumerate() {
        let prob = if dir == 1 && train_long_x.len() >= 20 {
            lr_long.predict(&scaler_long.transform(&feats[bar]))
        } else if dir == -1 && train_short_x.len() >= 20 {
            lr_short.predict(&scaler_short.transform(&feats[bar]))
        } else {
            0.0
        };
        if prob > THRESH { lr_pnls.push(test_pnls[i]); }
    }

    let (lr_wr, lr_pf, lr_pnl) = stats(&lr_pnls);
    let lr_pct = if n_test > 0 { lr_pnls.len() as f64 / n_test as f64 * 100.0 } else { 0.0 };

    FilterResult {
        filter: filter_name.to_string(),
        binary_trades: n_test, binary_wr: bin_wr, binary_pf: bin_pf, binary_pnl: bin_pnl,
        lr_trades: lr_pnls.len(), lr_wr, lr_pf, lr_pnl,
        lr_pct_of_binary: lr_pct,
    }
}

fn stats(pnls: &[f64]) -> (f64, f64, f64) {
    if pnls.is_empty() { return (0.0, 0.0, 0.0); }
    let wins: f64  = pnls.iter().filter(|&&p| p > 0.0).count() as f64;
    let tw: f64 = pnls.iter().filter(|&&p| p > 0.0).sum();
    let tl: f64 = pnls.iter().filter(|&&p| p <= 0.0).map(|&p| -p).sum();
    let wr = wins / pnls.len() as f64 * 100.0;
    let pf = if tl > 0.0 { tw / tl } else { tw };
    (wr, pf, pnls.iter().sum())
}

// ── Per-coin evaluation ───────────────────────────────────────────────────────
fn eval_coin(coin: &str) -> Option<CoinResult> {
    let bars = load_1m(coin);
    if bars.is_empty() { eprintln!("  SKIP {}: no 1m data", coin); return None; }

    let (feats, signals) = compute(&bars);
    if signals.len() < 100 {
        eprintln!("  SKIP {}: only {} signals", coin, signals.len());
        return None;
    }

    let baseline  = eval_filter(Filter::Baseline,  "baseline",  &signals, &bars, &feats);
    let tod       = eval_filter(Filter::TimeOfDay, "tod",       &signals, &bars, &feats);
    let liquidity = eval_filter(Filter::Liquidity, "liquidity", &signals, &bars, &feats);

    eprintln!(
        "  {:6}  sigs={:6}  \
         | base bin {:5.1}%WR lr {:4}t  \
         | tod  bin {:5.1}%WR lr {:4}t  \
         | liq  bin {:5.1}%WR lr {:4}t",
        coin, signals.len(),
        baseline.binary_wr,  baseline.lr_trades,
        tod.binary_wr,       tod.lr_trades,
        liquidity.binary_wr, liquidity.lr_trades,
    );

    Some(CoinResult {
        coin: coin.to_string(),
        total_signals: signals.len(),
        baseline, tod, liquidity,
    })
}

// ── Main ─────────────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN15d — Pre-signal filter comparison (Baseline / ToD / Liquidity)");
    println!("================================================================");
    println!("Trade: TP={:.2}% SL={:.2}% MaxHold={}bars Fee={:.2}% Slip={:.3}%",
             TP_PCT*100.0, SL_PCT*100.0, MAX_HOLD, FEE*100.0, SLIP*100.0);
    println!("Filters: Baseline | ToD UTC {}-{} | Liquidity vol_ratio≥{:.1}",
             TOD_MIN, TOD_MAX, LIQ_MIN);
    println!("LR threshold: {:.2} | Breakeven WR ≈ 44%", THRESH);
    println!();

    if std::path::Path::new(OUT_PATH).exists() {
        println!("Results already exist at {}. Delete to re-run.", OUT_PATH);
        return;
    }

    let per_coin: Vec<CoinResult> = SCALP_COINS.par_iter()
        .filter_map(|&coin| {
            if shutdown.load(Ordering::SeqCst) { return None; }
            eval_coin(coin)
        })
        .collect();

    if per_coin.is_empty() { eprintln!("No coins processed."); return; }

    // ── Print results table ───────────────────────────────────────────────────
    println!();
    println!("{:<6}  {:>6} {:>6} {:>6}  {:>6} {:>6} {:>6}  {:>6} {:>6} {:>6}",
        "Coin",
        "BasWR%", "BasTrd", "BLrTrd",
        "TodWR%", "TodTrd", "TLrTrd",
        "LiqWR%", "LiqTrd", "LLrTrd");
    println!("{}", "─".repeat(78));
    for r in &per_coin {
        println!("{:<6}  {:>6.1} {:>6} {:>6}  {:>6.1} {:>6} {:>6}  {:>6.1} {:>6} {:>6}",
            r.coin,
            r.baseline.binary_wr,  r.baseline.binary_trades,  r.baseline.lr_trades,
            r.tod.binary_wr,       r.tod.binary_trades,       r.tod.lr_trades,
            r.liquidity.binary_wr, r.liquidity.binary_trades, r.liquidity.lr_trades,
        );
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    let nc = per_coin.len() as f64;
    let avg = |v: Vec<f64>| v.iter().sum::<f64>() / nc;

    fn make_summary(name: &str, get: fn(&CoinResult) -> &FilterResult, coins: &[CoinResult], nc: f64) -> FilterSummary {
        let avg = |v: Vec<f64>| v.iter().sum::<f64>() / nc;
        let bin_total: usize = coins.iter().map(|r| get(r).binary_trades).sum();
        let lr_total:  usize = coins.iter().map(|r| get(r).lr_trades).sum();
        FilterSummary {
            filter: name.to_string(),
            binary_avg_wr: avg(coins.iter().map(|r| get(r).binary_wr).collect()),
            binary_avg_pf: avg(coins.iter().map(|r| get(r).binary_pf).collect()),
            binary_total_trades: bin_total,
            lr_avg_wr: avg(coins.iter().map(|r| get(r).lr_wr).collect()),
            lr_avg_pf: avg(coins.iter().map(|r| get(r).lr_pf).collect()),
            lr_total_trades: lr_total,
            lr_pct_of_binary: if bin_total > 0 { lr_total as f64 / bin_total as f64 * 100.0 } else { 0.0 },
        }
    }
    let summaries: Vec<FilterSummary> = vec![
        make_summary("Baseline",  |r| &r.baseline,  &per_coin, nc),
        make_summary("ToD",       |r| &r.tod,        &per_coin, nc),
        make_summary("Liquidity", |r| &r.liquidity,  &per_coin, nc),
    ];

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("SUMMARY (OOS — test half, breakeven WR ≈ 44%)");
    println!("{:<12}  {:>8} {:>7} {:>8}  {:>8} {:>7} {:>8} {:>7}",
        "Filter", "BinWR%", "BinPF", "BinTrd", "LRWR%", "LRPF", "LRTrd", "LR%ofBin");
    println!("{}", "─".repeat(80));
    for s in &summaries {
        let bin_t = s.binary_total_trades;
        println!("{:<12}  {:>8.1} {:>7.3} {:>8}  {:>8.1} {:>7.3} {:>8} {:>7.1}%",
            s.filter,
            s.binary_avg_wr, s.binary_avg_pf, bin_t,
            s.lr_avg_wr, s.lr_avg_pf, s.lr_total_trades,
            s.lr_pct_of_binary,
        );
    }

    let output = Output {
        experiment: "RUN15d pre-signal filter comparison — proper Rust implementation".to_string(),
        tp_pct: TP_PCT*100.0, sl_pct: SL_PCT*100.0, fee_pct: FEE*100.0, slip_pct: SLIP*100.0,
        threshold: THRESH, max_hold_bars: MAX_HOLD,
        tod_hours: format!("UTC {}-{}", TOD_MIN, TOD_MAX),
        liq_vol_ratio_min: LIQ_MIN,
        per_coin, summary: summaries,
    };

    let json = serde_json::to_string_pretty(&output).expect("JSON");
    std::fs::write(OUT_PATH, json).expect("write results");
    println!();
    println!("Results saved to {}", OUT_PATH);
}
