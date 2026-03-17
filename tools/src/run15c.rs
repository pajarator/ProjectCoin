/// RUN15c — ML model comparison for COINCLAW scalp signal filter
///
/// Original RUN15c question: "Do GBM, RF, or more features beat Logistic Regression?"
/// This proper implementation adds: full 1-year 1m data, real TP/SL simulation,
/// fees + slippage, 50/50 OOS split, single pre-specified threshold 0.55.
///
/// Models tested per coin (separate long/short models each):
///   1. LR  (9 features)    — same as run15b baseline
///   2. LR  (17 features)   — original "more features" idea (add EMA, MACD, ATR, etc.)
///   3. RF  (50 trees, depth 5, sqrt(feats) per split)
///   4. GBM (50 estimators, lr=0.1, depth 3, MSE loss on residuals)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rayon::prelude::*;
use serde::Serialize;

use crate::indicators::{bollinger, rsi, rolling_mean, rolling_min, rolling_max};

// ── Constants ─────────────────────────────────────────────────────────────────
const TP_PCT:    f64   = 0.008;
const SL_PCT:    f64   = 0.001;
const MAX_HOLD:  usize = 60;
const FEE:       f64   = 0.001;
const SLIP:      f64   = 0.0005;
const THRESH:    f64   = 0.55;

const LEARN_RATE: f64   = 0.05;
const LR_ITER:    usize = 500;
const LR_LAM:     f64   = 10.0;   // L2 regularisation λ (C=0.1 → λ=10)

const RF_TREES:   usize = 50;
const RF_DEPTH:   u8    = 5;
const GBM_EST:    usize = 50;
const GBM_RATE:   f64   = 0.1;
const GBM_DEPTH:  u8    = 3;
const MIN_LEAF:   usize = 5;

const N_BASIC: usize = 9;
const N_EXT:   usize = 17;

const SCALP_COINS: [&str; 10] = [
    "BTC", "ETH", "BNB", "SOL", "ADA", "XRP", "DOGE", "LTC", "LINK", "DOT",
];
const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const OUT_PATH: &str =
    "/home/scamarena/ProjectCoin/archive/RUN15c/run15c_corrected_results.json";

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

// ── Trade simulation (identical to run15b) ────────────────────────────────────
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
    if direction == 1 {
        (ec2 * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0
    } else {
        (ep - ec2 * (1.0 + SLIP)) / ep * 100.0 - FEE * 200.0
    }
}

// ── Feature computation ───────────────────────────────────────────────────────
/// Basic 9 features (same as run15b):
///   [rsi14, vol_ratio, stoch_k, stoch_d, stoch_cross, bb_pos, roc3, body3, hour]
/// Extended adds 8 more (original RUN15c idea 3):
///   [ema9_rel, ema21_rel, ema_diff, macd_hist, atr_pct, hl_pct, candle_str, vol_steep]
fn compute(bars: &[Bar1m])
    -> (Vec<[f64; N_BASIC]>, Vec<[f64; N_EXT]>, Vec<(usize, i32)>)
{
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
        let r = high14[i] - low14[i];
        if !high14[i].is_nan() && r > 0.0 {
            stoch_k[i] = 100.0 * (close[i] - low14[i]) / r;
        }
    }
    // Stochastic D — NaN-aware manual loop (run15b fix)
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

    // Extended: EMA(9) and EMA(21)
    let (alpha9, alpha21) = (2.0 / 10.0, 2.0 / 22.0);
    let mut e9  = close[0];
    let mut e21 = close[0];
    let mut ema9  = vec![close[0]; n];
    let mut ema21 = vec![close[0]; n];
    for i in 1..n {
        e9  = alpha9  * close[i] + (1.0 - alpha9)  * e9;
        e21 = alpha21 * close[i] + (1.0 - alpha21) * e21;
        ema9[i]  = e9;
        ema21[i] = e21;
    }

    // Extended: MACD histogram normalised by close
    let (a12, a26, asig) = (2.0 / 13.0, 2.0 / 27.0, 2.0 / 10.0);
    let mut em12 = close[0]; let mut em26 = close[0];
    let mut macd_line = vec![0.0_f64; n];
    for i in 1..n {
        em12 = a12 * close[i] + (1.0 - a12) * em12;
        em26 = a26 * close[i] + (1.0 - a26) * em26;
        macd_line[i] = em12 - em26;
    }
    let mut sig_line = macd_line[0];
    let mut macd_hist = vec![0.0_f64; n];
    for i in 1..n {
        sig_line = asig * macd_line[i] + (1.0 - asig) * sig_line;
        macd_hist[i] = if close[i] > 0.0 { (macd_line[i] - sig_line) / close[i] } else { 0.0 };
    }

    // Extended: ATR(14) Wilder smoothing
    let mut atr14 = vec![0.0_f64; n];
    {
        let mut prev = f64::NAN;
        for i in 1..n {
            let tr = (high[i] - low[i])
                .max((high[i] - close[i-1]).abs())
                .max((low[i]  - close[i-1]).abs());
            prev = if prev.is_nan() { tr } else { (prev * 13.0 + tr) / 14.0 };
            atr14[i] = prev;
        }
    }

    // Extended: volume MA(10) for volume steepness
    let vol_ma10 = rolling_mean(&vol, 10);

    // ── Assemble feature arrays and detect signals ─────────────────────────────
    let warmup = 23usize;
    let mut basic_feats: Vec<[f64; N_BASIC]> = vec![[0.0; N_BASIC]; n];
    let mut ext_feats:   Vec<[f64; N_EXT]>   = vec![[0.0; N_EXT]; n];
    let mut signals: Vec<(usize, i32)> = Vec::new();

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
        let hr = bars[i].hour as f64;

        basic_feats[i] = [rsi14[i], vol_ratio[i], sk, sd, cross, bb_pos, roc3, body_ma3[i], hr];

        // Extended: 8 additional features (fallback to 0 on edge-case NaN)
        let ema9_rel  = ema9[i]  / close[i] - 1.0;
        let ema21_rel = ema21[i] / close[i] - 1.0;
        let atr_pct   = if close[i] > 0.0 { atr14[i] / close[i] } else { 0.0 };
        let hl_pct    = (high[i] - low[i]) / close[i].max(1e-12);
        let cand_str  = { let r = high[i]-low[i]; if r > 0.0 { (close[i]-open[i]) / r } else { 0.0 } };
        let vstep     = if !vol_ma10[i].is_nan() && vol_ma10[i] > 0.0 {
            (vol[i] - vol_ma10[i]) / vol_ma10[i]
        } else { 0.0 };

        ext_feats[i] = [
            rsi14[i], vol_ratio[i], sk, sd, cross, bb_pos, roc3, body_ma3[i], hr,
            ema9_rel, ema21_rel, ema9_rel - ema21_rel, macd_hist[i],
            atr_pct, hl_pct, cand_str, vstep,
        ];

        // Signal detection (vol_spike_rev | stoch_cross) — same as run15b
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

    (basic_feats, ext_feats, signals)
}

// ── Standardiser (dynamic feature count) ─────────────────────────────────────
struct Scaler { mean: Vec<f64>, std: Vec<f64> }

impl Scaler {
    fn fit(xs: &[Vec<f64>]) -> Self {
        if xs.is_empty() { return Scaler { mean: vec![], std: vec![] }; }
        let nf = xs[0].len();
        let n  = xs.len() as f64;
        let mut mean = vec![0.0; nf];
        let mut std  = vec![1.0; nf];
        for j in 0..nf { mean[j] = xs.iter().map(|x| x[j]).sum::<f64>() / n; }
        for j in 0..nf {
            let var = xs.iter().map(|x| (x[j] - mean[j]).powi(2)).sum::<f64>() / n;
            std[j] = var.sqrt().max(1e-8);
        }
        Scaler { mean, std }
    }

    fn transform(&self, x: &[f64]) -> Vec<f64> {
        x.iter().enumerate().map(|(j, &v)| (v - self.mean[j]) / self.std[j]).collect()
    }
}

// ── Logistic Regression ───────────────────────────────────────────────────────
struct LogReg { w: Vec<f64>, b: f64 }

impl LogReg {
    fn new(nf: usize) -> Self { LogReg { w: vec![0.0; nf], b: 0.0 } }

    fn sigmoid(x: f64) -> f64 { 1.0 / (1.0 + (-x.clamp(-500.0, 500.0)).exp()) }

    fn predict(&self, x: &[f64]) -> f64 {
        let z: f64 = self.b + self.w.iter().zip(x.iter()).map(|(&w, &xi)| w * xi).sum::<f64>();
        Self::sigmoid(z)
    }

    fn fit(&mut self, xs: &[Vec<f64>], ys: &[f64]) {
        let n = xs.len(); if n == 0 { return; }
        let nf = self.w.len();
        let nf_ = n as f64;
        for _ in 0..LR_ITER {
            let mut dw = vec![0.0_f64; nf];
            let mut db = 0.0_f64;
            for (xi, &yi) in xs.iter().zip(ys.iter()) {
                let err = self.predict(xi) - yi;
                db += err;
                for j in 0..nf { dw[j] += err * xi[j]; }
            }
            self.b -= LEARN_RATE * db / nf_;
            for j in 0..nf {
                self.w[j] -= LEARN_RATE * (dw[j] / nf_ + LR_LAM * self.w[j]);
            }
        }
    }
}

// ── LCG random number generator ───────────────────────────────────────────────
fn lcg_next(s: &mut u64) -> u64 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *s
}
fn lcg_usize(s: &mut u64, n: usize) -> usize {
    if n == 0 { return 0; }
    (lcg_next(s) % n as u64) as usize
}
// Fisher-Yates: choose k from n without replacement
fn sample_k(n: usize, k: usize, rng: &mut u64) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    for i in 0..k.min(n) {
        let j = i + lcg_usize(rng, n - i);
        v.swap(i, j);
    }
    v[..k.min(n)].to_vec()
}

// ── Decision Tree ─────────────────────────────────────────────────────────────
enum Node {
    Leaf(f64),
    Split { feat: usize, thresh: f64, left: Box<Node>, right: Box<Node> },
}

fn predict_node(node: &Node, x: &[f64]) -> f64 {
    match node {
        Node::Leaf(v) => *v,
        Node::Split { feat, thresh, left, right } => {
            if x[*feat].is_nan() || x[*feat] <= *thresh { predict_node(left, x) }
            else { predict_node(right, x) }
        }
    }
}

/// use_gini=true → Gini impurity (RF classification)
/// use_gini=false → variance reduction (GBM regression on residuals)
fn build_node(
    idx: &[usize], X: &[Vec<f64>], y: &[f64],
    depth: u8, max_depth: u8, n_try: usize, rng: &mut u64, use_gini: bool,
) -> Node {
    let n = idx.len() as f64;
    if (n as usize) <= MIN_LEAF || depth >= max_depth {
        let mean = idx.iter().map(|&i| y[i]).sum::<f64>() / n;
        return Node::Leaf(mean);
    }

    let vals: Vec<f64> = idx.iter().map(|&i| y[i]).collect();
    let mean_y = vals.iter().sum::<f64>() / n;
    let base_imp = if use_gini {
        let p1 = vals.iter().filter(|&&v| v > 0.5).count() as f64 / n;
        1.0 - p1 * p1 - (1.0 - p1).powi(2)
    } else {
        vals.iter().map(|&v| (v - mean_y).powi(2)).sum::<f64>() / n
    };
    if base_imp < 1e-8 { return Node::Leaf(mean_y); }

    let n_feats = X[0].len();
    let try_feats = sample_k(n_feats, n_try, rng);

    let (mut best_gain, mut best_feat, mut best_thr) = (1e-9_f64, usize::MAX, f64::NAN);

    for fi in try_feats {
        let mut pairs: Vec<(f64, f64)> = idx.iter()
            .map(|&i| (X[i][fi], y[i]))
            .filter(|(fv, _)| !fv.is_nan())
            .collect();
        if pairs.len() < 4 { continue; }
        pairs.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let np = pairs.len() as f64;
        let (mut ls, mut lq, mut ln) = (0.0_f64, 0.0_f64, 0.0_f64);
        let mut rs: f64 = pairs.iter().map(|p| p.1).sum();
        let mut rq: f64 = pairs.iter().map(|p| p.1 * p.1).sum();
        let mut rn = np;

        for i in 0..pairs.len() - 1 {
            let (fv, lbl) = pairs[i];
            ls += lbl; lq += lbl * lbl; ln += 1.0;
            rs -= lbl; rq -= lbl * lbl; rn -= 1.0;
            if (fv - pairs[i+1].0).abs() < 1e-12 { continue; }

            let gain = if use_gini {
                let p1l = ls / ln; let p1r = rs / rn;
                base_imp
                    - (ln / np) * (1.0 - p1l * p1l - (1.0 - p1l).powi(2))
                    - (rn / np) * (1.0 - p1r * p1r - (1.0 - p1r).powi(2))
            } else {
                let ml = ls / ln; let mr = rs / rn;
                base_imp
                    - (ln / np) * (lq / ln - ml * ml).max(0.0)
                    - (rn / np) * (rq / rn - mr * mr).max(0.0)
            };

            if gain > best_gain { best_gain = gain; best_feat = fi; best_thr = (fv + pairs[i+1].0) / 2.0; }
        }
    }

    if best_feat == usize::MAX { return Node::Leaf(mean_y); }

    let left_idx: Vec<usize> = idx.iter().filter(|&&i| X[i][best_feat].is_nan() || X[i][best_feat] <= best_thr).cloned().collect();
    let right_idx: Vec<usize> = idx.iter().filter(|&&i| !X[i][best_feat].is_nan() && X[i][best_feat] > best_thr).cloned().collect();
    if left_idx.is_empty() || right_idx.is_empty() { return Node::Leaf(mean_y); }

    Node::Split {
        feat:  best_feat,
        thresh: best_thr,
        left:  Box::new(build_node(&left_idx,  X, y, depth+1, max_depth, n_try, rng, use_gini)),
        right: Box::new(build_node(&right_idx, X, y, depth+1, max_depth, n_try, rng, use_gini)),
    }
}

// ── Random Forest ─────────────────────────────────────────────────────────────
struct RF { trees: Vec<Node> }

impl RF {
    fn fit(X: &[Vec<f64>], y: &[f64], seed: u64) -> Self {
        let n = X.len(); if n == 0 { return RF { trees: vec![] }; }
        let nf   = X[0].len();
        let n_try = ((nf as f64).sqrt() as usize).max(1);
        let mut rng = seed;
        let mut trees = Vec::with_capacity(RF_TREES);
        for _ in 0..RF_TREES {
            let boot: Vec<usize> = (0..n).map(|_| lcg_usize(&mut rng, n)).collect();
            trees.push(build_node(&boot, X, y, 0, RF_DEPTH, n_try, &mut rng, true));
        }
        RF { trees }
    }
    fn predict(&self, x: &[f64]) -> f64 {
        if self.trees.is_empty() { return 0.0; }
        let s: f64 = self.trees.iter().map(|t| predict_node(t, x)).sum();
        (s / self.trees.len() as f64).clamp(0.0, 1.0)
    }
}

// ── Gradient Boosting (MSE / residual fitting) ────────────────────────────────
struct GBM { base: f64, trees: Vec<Node> }

impl GBM {
    fn fit(X: &[Vec<f64>], y: &[f64], seed: u64) -> Self {
        let n = X.len(); if n == 0 { return GBM { base: 0.0, trees: vec![] }; }
        let nf = X[0].len();
        let all_idx: Vec<usize> = (0..n).collect();
        let base = y.iter().sum::<f64>() / n as f64;
        let mut preds = vec![base; n];
        let mut trees = Vec::with_capacity(GBM_EST);
        let mut rng = seed;
        for _ in 0..GBM_EST {
            let resid: Vec<f64> = (0..n).map(|i| y[i] - preds[i]).collect();
            let tree = build_node(&all_idx, X, &resid, 0, GBM_DEPTH, nf, &mut rng, false);
            for i in 0..n { preds[i] += GBM_RATE * predict_node(&tree, &X[i]); }
            trees.push(tree);
        }
        GBM { base, trees }
    }
    fn predict(&self, x: &[f64]) -> f64 {
        let raw = self.base + self.trees.iter().map(|t| GBM_RATE * predict_node(t, x)).sum::<f64>();
        raw.clamp(0.0, 1.0)
    }
}

// ── Result structs ────────────────────────────────────────────────────────────
#[derive(Debug, Serialize, Clone)]
struct ModelStats {
    n_trades: usize,
    win_rate: f64,
    profit_factor: f64,
    total_pnl: f64,
    pct_of_binary: f64,
}

impl ModelStats {
    fn zero() -> Self { ModelStats { n_trades: 0, win_rate: 0.0, profit_factor: 0.0, total_pnl: 0.0, pct_of_binary: 0.0 } }

    fn from_probs(test_pnls: &[f64], probs: &[f64], n_binary: usize) -> Self {
        let mut wins = 0usize;
        let mut tw = 0.0_f64; let mut tl = 0.0_f64;
        let mut total_pnl = 0.0_f64; let mut n_trades = 0usize;
        for (&pnl, &p) in test_pnls.iter().zip(probs.iter()) {
            if p > THRESH {
                n_trades += 1; total_pnl += pnl;
                if pnl > 0.0 { wins += 1; tw += pnl; } else { tl -= pnl; }
            }
        }
        let win_rate = if n_trades > 0 { wins as f64 / n_trades as f64 * 100.0 } else { 0.0 };
        let profit_factor = if tl > 0.0 { tw / tl } else { tw };
        let pct_of_binary = if n_binary > 0 { n_trades as f64 / n_binary as f64 * 100.0 } else { 0.0 };
        ModelStats { n_trades, win_rate, profit_factor, total_pnl, pct_of_binary }
    }
}

fn eval_binary(pnls: &[f64]) -> ModelStats {
    let mut wins = 0usize; let mut tw = 0.0_f64; let mut tl = 0.0_f64;
    let mut total_pnl = 0.0_f64;
    for &p in pnls { total_pnl += p; if p > 0.0 { wins += 1; tw += p; } else { tl -= p; } }
    let n = pnls.len();
    ModelStats {
        n_trades: n,
        win_rate: if n > 0 { wins as f64 / n as f64 * 100.0 } else { 0.0 },
        profit_factor: if tl > 0.0 { tw / tl } else { tw },
        total_pnl,
        pct_of_binary: 100.0,
    }
}

#[derive(Debug, Serialize)]
struct CoinResult {
    coin: String,
    total_signals: usize,
    train_signals: usize,
    test_signals: usize,
    binary: ModelStats,
    lr9: ModelStats,
    lr17: ModelStats,
    rf: ModelStats,
    gbm: ModelStats,
}

#[derive(Debug, Serialize)]
struct Summary {
    n_coins: usize,
    binary_avg_wr: f64, lr9_avg_wr: f64, lr17_avg_wr: f64, rf_avg_wr: f64, gbm_avg_wr: f64,
    binary_avg_pf: f64, lr9_avg_pf: f64, lr17_avg_pf: f64, rf_avg_pf: f64, gbm_avg_pf: f64,
    binary_total_trades: usize, lr9_total_trades: usize, lr17_total_trades: usize,
    rf_total_trades: usize, gbm_total_trades: usize,
}

#[derive(Debug, Serialize)]
struct Output {
    experiment: String,
    tp_pct: f64, sl_pct: f64, fee_pct: f64, slip_pct: f64,
    threshold: f64, max_hold_bars: usize,
    per_coin: Vec<CoinResult>,
    summary: Summary,
}

// ── Helper: split signals by direction, return long/short feature vectors ──────
fn split_by_dir(
    X: &[Vec<f64>], dirs: &[i32], wins: &[f64],
) -> (Vec<Vec<f64>>, Vec<f64>, Vec<Vec<f64>>, Vec<f64>) {
    let mut lx = Vec::new(); let mut ly = Vec::new();
    let mut sx = Vec::new(); let mut sy = Vec::new();
    for i in 0..dirs.len() {
        if dirs[i] == 1  { lx.push(X[i].clone()); ly.push(wins[i]); }
        else              { sx.push(X[i].clone()); sy.push(wins[i]); }
    }
    (lx, ly, sx, sy)
}

// ── Per-coin evaluation ───────────────────────────────────────────────────────
fn eval_coin(coin: &str) -> Option<CoinResult> {
    let bars = load_1m(coin);
    if bars.is_empty() { eprintln!("  SKIP {}: no 1m data", coin); return None; }

    let (basic_feats, ext_feats, signals) = compute(&bars);
    if signals.len() < 100 {
        eprintln!("  SKIP {}: only {} signals", coin, signals.len());
        return None;
    }

    let total = signals.len();
    let split = total / 2;
    let train_sigs = &signals[..split];
    let test_sigs  = &signals[split..];

    // Simulate all trades up front (used for all models)
    let all_pnls: Vec<f64> = signals.iter()
        .map(|&(bar, dir)| simulate_trade(&bars, bar, dir))
        .collect();
    let train_pnls = &all_pnls[..split];
    let test_pnls  = &all_pnls[split..];

    let binary = eval_binary(test_pnls);
    let n_test = test_sigs.len();

    // Collect per-signal feature rows and directions
    let train_dirs: Vec<i32> = train_sigs.iter().map(|&(_, d)| d).collect();
    let test_dirs:  Vec<i32> = test_sigs.iter().map(|&(_, d)| d).collect();
    let train_wins: Vec<f64> = train_pnls.iter().map(|&p| if p > 0.0 { 1.0 } else { 0.0 }).collect();

    let train_b9: Vec<Vec<f64>> = train_sigs.iter().map(|&(bar, _)| basic_feats[bar].to_vec()).collect();
    let test_b9:  Vec<Vec<f64>> = test_sigs.iter().map(|&(bar, _)| basic_feats[bar].to_vec()).collect();
    let train_e17: Vec<Vec<f64>> = train_sigs.iter().map(|&(bar, _)| ext_feats[bar].to_vec()).collect();
    let test_e17:  Vec<Vec<f64>> = test_sigs.iter().map(|&(bar, _)| ext_feats[bar].to_vec()).collect();

    // ── LR(9) ────────────────────────────────────────────────────────────────
    let scaler9 = Scaler::fit(&train_b9);
    let train_s9: Vec<Vec<f64>> = train_b9.iter().map(|x| scaler9.transform(x)).collect();
    let test_s9:  Vec<Vec<f64>> = test_b9.iter().map(|x| scaler9.transform(x)).collect();
    let (lx9, ly9, sx9, sy9) = split_by_dir(&train_s9, &train_dirs, &train_wins);
    let mut lr9_long  = LogReg::new(N_BASIC); if lx9.len() >= 20 { lr9_long.fit(&lx9, &ly9); }
    let mut lr9_short = LogReg::new(N_BASIC); if sx9.len() >= 20 { lr9_short.fit(&sx9, &sy9); }
    let lr9_probs: Vec<f64> = test_dirs.iter().enumerate().map(|(i, &d)| {
        if d == 1 && lx9.len() >= 20 { lr9_long.predict(&test_s9[i]) }
        else if d == -1 && sx9.len() >= 20 { lr9_short.predict(&test_s9[i]) }
        else { 0.0 }
    }).collect();
    let lr9 = ModelStats::from_probs(test_pnls, &lr9_probs, n_test);

    // ── LR(17) ───────────────────────────────────────────────────────────────
    let scaler17 = Scaler::fit(&train_e17);
    let train_s17: Vec<Vec<f64>> = train_e17.iter().map(|x| scaler17.transform(x)).collect();
    let test_s17:  Vec<Vec<f64>> = test_e17.iter().map(|x| scaler17.transform(x)).collect();
    let (lx17, ly17, sx17, sy17) = split_by_dir(&train_s17, &train_dirs, &train_wins);
    let mut lr17_long  = LogReg::new(N_EXT); if lx17.len() >= 20 { lr17_long.fit(&lx17, &ly17); }
    let mut lr17_short = LogReg::new(N_EXT); if sx17.len() >= 20 { lr17_short.fit(&sx17, &sy17); }
    let lr17_probs: Vec<f64> = test_dirs.iter().enumerate().map(|(i, &d)| {
        if d == 1 && lx17.len() >= 20 { lr17_long.predict(&test_s17[i]) }
        else if d == -1 && sx17.len() >= 20 { lr17_short.predict(&test_s17[i]) }
        else { 0.0 }
    }).collect();
    let lr17 = ModelStats::from_probs(test_pnls, &lr17_probs, n_test);

    // ── RF(9, unscaled) ──────────────────────────────────────────────────────
    let (rlx, rly, rsx, rsy) = split_by_dir(&train_b9, &train_dirs, &train_wins);
    let rf_long  = if rlx.len() >= 20 { Some(RF::fit(&rlx, &rly, 42)) } else { None };
    let rf_short = if rsx.len() >= 20 { Some(RF::fit(&rsx, &rsy, 43)) } else { None };
    let rf_probs: Vec<f64> = test_dirs.iter().enumerate().map(|(i, &d)| {
        if d == 1  { rf_long.as_ref().map(|m| m.predict(&test_b9[i])).unwrap_or(0.0) }
        else        { rf_short.as_ref().map(|m| m.predict(&test_b9[i])).unwrap_or(0.0) }
    }).collect();
    let rf = ModelStats::from_probs(test_pnls, &rf_probs, n_test);

    // ── GBM(9, unscaled) ─────────────────────────────────────────────────────
    let (glx, gly, gsx, gsy) = split_by_dir(&train_b9, &train_dirs, &train_wins);
    let gbm_long  = if glx.len() >= 20 { Some(GBM::fit(&glx, &gly, 44)) } else { None };
    let gbm_short = if gsx.len() >= 20 { Some(GBM::fit(&gsx, &gsy, 45)) } else { None };
    let gbm_probs: Vec<f64> = test_dirs.iter().enumerate().map(|(i, &d)| {
        if d == 1  { gbm_long.as_ref().map(|m| m.predict(&test_b9[i])).unwrap_or(0.0) }
        else        { gbm_short.as_ref().map(|m| m.predict(&test_b9[i])).unwrap_or(0.0) }
    }).collect();
    let gbm = ModelStats::from_probs(test_pnls, &gbm_probs, n_test);

    eprintln!(
        "  {:6}  sigs={:6}  | bin {:5.1}%WR | lr9 {:4}t | lr17 {:4}t | rf {:4}t | gbm {:4}t",
        coin, total, binary.win_rate, lr9.n_trades, lr17.n_trades, rf.n_trades, gbm.n_trades
    );

    Some(CoinResult {
        coin: coin.to_string(),
        total_signals: total, train_signals: split, test_signals: n_test,
        binary, lr9, lr17, rf, gbm,
    })
}

// ── Main ─────────────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN15c — ML model comparison: LR9 / LR17 / RF / GBM");
    println!("================================================================");
    println!("Trade: TP={:.2}% SL={:.2}% MaxHold={}bars Fee={:.2}% Slip={:.3}%",
             TP_PCT*100.0, SL_PCT*100.0, MAX_HOLD, FEE*100.0, SLIP*100.0);
    println!("Models: LR(9feat), LR(17feat), RF({}trees,depth{}), GBM({}est,lr={},depth{})",
             RF_TREES, RF_DEPTH, GBM_EST, GBM_RATE, GBM_DEPTH);
    println!("Threshold: {:.2} (pre-specified, single)", THRESH);
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
    println!("{:<6} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Coin", "BinWR%", "BinTrd", "LR9WR%", "LR9Trd", "LR17WR%", "LR17Trd", "RFWR%", "RFTrd", "GBMWR%", "GBMTrd");
    println!("{}", "─".repeat(98));
    for r in &per_coin {
        println!("{:<6} {:>8.1} {:>8} {:>8.1} {:>8} {:>8.1} {:>8} {:>8.1} {:>8} {:>8.1} {:>8}",
            r.coin,
            r.binary.win_rate, r.binary.n_trades,
            r.lr9.win_rate,    r.lr9.n_trades,
            r.lr17.win_rate,   r.lr17.n_trades,
            r.rf.win_rate,     r.rf.n_trades,
            r.gbm.win_rate,    r.gbm.n_trades,
        );
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    let nc = per_coin.len() as f64;
    let avg = |v: Vec<f64>| v.iter().sum::<f64>() / nc;

    let summary = Summary {
        n_coins: per_coin.len(),
        binary_avg_wr: avg(per_coin.iter().map(|r| r.binary.win_rate).collect()),
        lr9_avg_wr:    avg(per_coin.iter().map(|r| r.lr9.win_rate).collect()),
        lr17_avg_wr:   avg(per_coin.iter().map(|r| r.lr17.win_rate).collect()),
        rf_avg_wr:     avg(per_coin.iter().map(|r| r.rf.win_rate).collect()),
        gbm_avg_wr:    avg(per_coin.iter().map(|r| r.gbm.win_rate).collect()),
        binary_avg_pf: avg(per_coin.iter().map(|r| r.binary.profit_factor).collect()),
        lr9_avg_pf:    avg(per_coin.iter().map(|r| r.lr9.profit_factor).collect()),
        lr17_avg_pf:   avg(per_coin.iter().map(|r| r.lr17.profit_factor).collect()),
        rf_avg_pf:     avg(per_coin.iter().map(|r| r.rf.profit_factor).collect()),
        gbm_avg_pf:    avg(per_coin.iter().map(|r| r.gbm.profit_factor).collect()),
        binary_total_trades: per_coin.iter().map(|r| r.binary.n_trades).sum(),
        lr9_total_trades:    per_coin.iter().map(|r| r.lr9.n_trades).sum(),
        lr17_total_trades:   per_coin.iter().map(|r| r.lr17.n_trades).sum(),
        rf_total_trades:     per_coin.iter().map(|r| r.rf.n_trades).sum(),
        gbm_total_trades:    per_coin.iter().map(|r| r.gbm.n_trades).sum(),
    };

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("SUMMARY (OOS — second 50% of signals, breakeven WR ≈ 44%)");
    let bt = summary.binary_total_trades;
    println!("  {:<8}  Avg WR: {:5.1}%  Avg PF: {:.3}  Trades: {}",
             "Binary",  summary.binary_avg_wr, summary.binary_avg_pf, bt);
    println!("  {:<8}  Avg WR: {:5.1}%  Avg PF: {:.3}  Trades: {} ({:.1}% of bin)",
             "LR(9)",   summary.lr9_avg_wr,    summary.lr9_avg_pf,    summary.lr9_total_trades,
             summary.lr9_total_trades as f64 / bt as f64 * 100.0);
    println!("  {:<8}  Avg WR: {:5.1}%  Avg PF: {:.3}  Trades: {} ({:.1}% of bin)",
             "LR(17)",  summary.lr17_avg_wr,   summary.lr17_avg_pf,   summary.lr17_total_trades,
             summary.lr17_total_trades as f64 / bt as f64 * 100.0);
    println!("  {:<8}  Avg WR: {:5.1}%  Avg PF: {:.3}  Trades: {} ({:.1}% of bin)",
             "RF",      summary.rf_avg_wr,     summary.rf_avg_pf,     summary.rf_total_trades,
             summary.rf_total_trades as f64 / bt as f64 * 100.0);
    println!("  {:<8}  Avg WR: {:5.1}%  Avg PF: {:.3}  Trades: {} ({:.1}% of bin)",
             "GBM",     summary.gbm_avg_wr,    summary.gbm_avg_pf,    summary.gbm_total_trades,
             summary.gbm_total_trades as f64 / bt as f64 * 100.0);

    let output = Output {
        experiment: "RUN15c ML model comparison — proper Rust implementation".to_string(),
        tp_pct: TP_PCT*100.0, sl_pct: SL_PCT*100.0, fee_pct: FEE*100.0, slip_pct: SLIP*100.0,
        threshold: THRESH, max_hold_bars: MAX_HOLD,
        per_coin, summary,
    };

    let json = serde_json::to_string_pretty(&output).expect("JSON");
    std::fs::write(OUT_PATH, json).expect("write results");
    println!();
    println!("Results saved to {}", OUT_PATH);
}
