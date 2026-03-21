/// RUN35 — Scalp exit strategy grid search (1m, 1-year, 18 coins)
///
/// Tests 16 alternative scalp exit mechanisms against the baseline TP=0.8%, SL=0.1%.
/// Zero-fee assumption (matching v16 zero-fee exchange deployment).
///
/// Grid:
///   1. baseline         — TP=0.8%, SL=0.1% (current v16)
///   2-5. breakeven      — Move SL to entry after 0.15/0.20/0.30/0.40% gain
///   6-8. time_decay     — TP decays over bars (slow/medium/fast)
///   9-12. wider_bracket — Different TP/SL ratios
///   13-14. rsi_exit     — Close when RSI crosses threshold while profitable
///   15. stoch_exit      — Stoch_K crosses 50 while profitable
///   16. be020+decay     — Combined breakeven + time decay
///
/// Run: cargo run --release --features run35 -- --run35

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// ── Constants (matching config.rs) ───────────────────────────────────────────
const SCALP_RISK: f64 = 0.05;   // 5% of balance per scalp
const LEVERAGE: f64   = 5.0;
const INITIAL_BAL: f64 = 100.0;

// Entry params
const SCALP_VOL_MULT: f64      = 3.5;
const SCALP_RSI_EXTREME: f64   = 20.0;
const SCALP_STOCH_EXTREME: f64 = 5.0;
const SCALP_BB_SQUEEZE: f64    = 0.4;
const F6_DIR_ROC_3: f64        = -0.195;
const F6_AVG_BODY_3: f64       = 0.072;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

// ── Exit configuration ───────────────────────────────────────────────────────
#[derive(Clone)]
struct ExitCfg {
    label: &'static str,
    desc: &'static str,
    tp: f64,
    sl: f64,
    max_hold: u32,
    be_act: f64,                  // breakeven activation (0.0 = disabled)
    decay: [(u32, f64); 2],       // [(bar_thresh, new_tp); 2], (0, _) = disabled
    rsi_long_exit: f64,           // close long when RSI > this (0.0 = disabled)
    rsi_short_exit: f64,          // close short when RSI < this (100.0 = disabled)
    stoch_exit: bool,             // close when stoch_k crosses 50 while profitable
}

fn build_grid() -> Vec<ExitCfg> {
    vec![
        // 1. Baseline (current v16)
        ExitCfg { label: "baseline", desc: "TP=0.8% SL=0.1%",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.0,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        // 2-5. Breakeven stop
        ExitCfg { label: "be_015", desc: "BE@0.15% → SL=0",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.0015,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        ExitCfg { label: "be_020", desc: "BE@0.20% → SL=0",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.002,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        ExitCfg { label: "be_030", desc: "BE@0.30% → SL=0",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.003,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        ExitCfg { label: "be_040", desc: "BE@0.40% → SL=0",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.004,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        // 6-8. Time decay TP
        ExitCfg { label: "decay_slow", desc: "TP 0.8→0.5@60b→0.3@120b",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.0,
            decay: [(60, 0.005), (120, 0.003)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        ExitCfg { label: "decay_med", desc: "TP 0.8→0.5@45b→0.3@90b",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.0,
            decay: [(45, 0.005), (90, 0.003)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        ExitCfg { label: "decay_fast", desc: "TP 0.8→0.4@30b→0.2@90b",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.0,
            decay: [(30, 0.004), (90, 0.002)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        // 9-12. Wider bracket (different R:R)
        ExitCfg { label: "wide_2_4", desc: "SL=0.2% TP=0.4% (2:1)",
            tp: 0.004, sl: 0.002, max_hold: 480, be_act: 0.0,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        ExitCfg { label: "wide_3_5", desc: "SL=0.3% TP=0.5% (5:3)",
            tp: 0.005, sl: 0.003, max_hold: 480, be_act: 0.0,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        ExitCfg { label: "wide_3_3", desc: "SL=0.3% TP=0.3% (1:1)",
            tp: 0.003, sl: 0.003, max_hold: 480, be_act: 0.0,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        ExitCfg { label: "wide_5_5", desc: "SL=0.5% TP=0.5% (1:1 wide)",
            tp: 0.005, sl: 0.005, max_hold: 480, be_act: 0.0,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
        // 13-14. RSI indicator exit
        ExitCfg { label: "rsi_65_35", desc: "RSI exit L>65 S<35",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.0,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 65.0, rsi_short_exit: 35.0, stoch_exit: false },
        ExitCfg { label: "rsi_60_40", desc: "RSI exit L>60 S<40",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.0,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 60.0, rsi_short_exit: 40.0, stoch_exit: false },
        // 15. Stochastic exit
        ExitCfg { label: "stoch_50", desc: "Stoch K crosses 50 while profitable",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.0,
            decay: [(0, 0.0), (0, 0.0)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: true },
        // 16. Combined: breakeven + time decay
        ExitCfg { label: "be020_decay", desc: "BE@0.2% + TP 0.8→0.4@60→0.2@120",
            tp: 0.008, sl: 0.001, max_hold: 480, be_act: 0.002,
            decay: [(60, 0.004), (120, 0.002)],
            rsi_long_exit: 0.0, rsi_short_exit: 100.0, stoch_exit: false },
    ]
}

// ── Data ─────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum Dir { Long, Short }

struct CoinData {
    name: &'static str,
    close: Vec<f64>,
    open: Vec<f64>,
    rsi: Vec<f64>,
    vol: Vec<f64>,
    vol_ma: Vec<f64>,
    stoch_k: Vec<f64>,
    stoch_d: Vec<f64>,
    bb_upper: Vec<f64>,
    bb_lower: Vec<f64>,
    bb_width: Vec<f64>,
    bb_width_avg: Vec<f64>,
    roc_3: Vec<f64>,
    avg_body_3: Vec<f64>,
}

struct ScalpPos {
    dir: Dir,
    entry: f64,
    notional: f64,
    bars_held: u32,
    be_active: bool,
}

// ── Rolling helpers (matching run32/indicators.rs) ───────────────────────────
fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let mut sum = 0.0;
    for i in 0..n {
        sum += data[i];
        if i >= w { sum -= data[i - w]; }
        if i + 1 >= w { out[i] = sum / w as f64; }
    }
    out
}

fn rstd(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        let s = &data[i + 1 - w..=i];
        let m = s.iter().sum::<f64>() / w as f64;
        let v = s.iter().map(|x| (x - m).powi(2)).sum::<f64>() / w as f64;
        out[i] = v.sqrt();
    }
    out
}

fn rmin(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::INFINITY, f64::min);
    }
    out
}

fn rmax(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    }
    out
}

fn rsi_calc(c: &[f64], period: usize) -> Vec<f64> {
    let n = c.len();
    let mut out = vec![f64::NAN; n];
    if n < period + 1 { return out; }
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        let d = c[i] - c[i - 1];
        if d > 0.0 { gains[i] = d; } else { losses[i] = -d; }
    }
    let ag = rmean(&gains, period);
    let al = rmean(&losses, period);
    for i in 0..n {
        if !ag[i].is_nan() && !al[i].is_nan() {
            out[i] = if al[i] == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag[i] / al[i]) };
        }
    }
    out
}

// ── CSV loader ───────────────────────────────────────────────────────────────
fn load_1m(coin: &str) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
    let path = format!(
        "/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_1year.csv", coin
    );
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => { eprintln!("  Missing {}: {}", path, e); return None; }
    };
    let cap = 530_000;
    let mut opens  = Vec::with_capacity(cap);
    let mut highs  = Vec::with_capacity(cap);
    let mut lows   = Vec::with_capacity(cap);
    let mut closes = Vec::with_capacity(cap);
    let mut vols   = Vec::with_capacity(cap);
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next();
        let o: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let h: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let l: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let c: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let v: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        if o.is_nan()||h.is_nan()||l.is_nan()||c.is_nan()||v.is_nan() { continue; }
        opens.push(o); highs.push(h); lows.push(l); closes.push(c); vols.push(v);
    }
    if closes.len() < 100 { return None; }
    Some((opens, highs, lows, closes, vols))
}

// ── Indicator computation (vectorized for all bars) ──────────────────────────
fn compute_1m_data(
    name: &'static str,
    o: Vec<f64>, h: Vec<f64>, l: Vec<f64>, c: Vec<f64>, v: Vec<f64>,
) -> CoinData {
    let n = c.len();

    let rsi = rsi_calc(&c, 14);
    let vol_ma = rmean(&v, 20);

    // Stochastic K/D
    let ll = rmin(&l, 14);
    let hh = rmax(&h, 14);
    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n {
        if !ll[i].is_nan() && !hh[i].is_nan() {
            let range = hh[i] - ll[i];
            if range > 0.0 { stoch_k[i] = 100.0 * (c[i] - ll[i]) / range; }
        }
    }
    let stoch_d = rmean(&stoch_k, 3);

    // Bollinger Bands
    let bb_sma = rmean(&c, 20);
    let bb_std = rstd(&c, 20);
    let mut bb_upper = vec![f64::NAN; n];
    let mut bb_lower = vec![f64::NAN; n];
    let mut bb_width_raw = vec![f64::NAN; n];
    for i in 0..n {
        if !bb_sma[i].is_nan() && !bb_std[i].is_nan() {
            bb_upper[i] = bb_sma[i] + 2.0 * bb_std[i];
            bb_lower[i] = bb_sma[i] - 2.0 * bb_std[i];
            bb_width_raw[i] = bb_upper[i] - bb_lower[i];
        }
    }
    let bb_width_avg = rmean(&bb_width_raw, 20);

    // F6 filter inputs
    let mut roc_3 = vec![f64::NAN; n];
    for i in 3..n {
        if c[i - 3] > 0.0 { roc_3[i] = (c[i] - c[i - 3]) / c[i - 3] * 100.0; }
    }
    let mut avg_body_3 = vec![f64::NAN; n];
    for i in 2..n {
        let b0 = (c[i] - o[i]).abs() / c[i] * 100.0;
        let b1 = (c[i - 1] - o[i - 1]).abs() / c[i - 1] * 100.0;
        let b2 = (c[i - 2] - o[i - 2]).abs() / c[i - 2] * 100.0;
        avg_body_3[i] = (b0 + b1 + b2) / 3.0;
    }

    CoinData {
        name, close: c, open: o,
        rsi, vol: v, vol_ma, stoch_k, stoch_d,
        bb_upper, bb_lower, bb_width: bb_width_raw, bb_width_avg,
        roc_3, avg_body_3,
    }
}

// ── Entry signal (matching live scalp_entry_with_price) ──────────────────────
fn f6_pass(data: &CoinData, i: usize, dir: Dir) -> bool {
    if data.roc_3[i].is_nan() || data.avg_body_3[i].is_nan() { return false; }
    let sign = if dir == Dir::Long { 1.0 } else { -1.0 };
    data.roc_3[i] * sign < F6_DIR_ROC_3 && data.avg_body_3[i] > F6_AVG_BODY_3
}

fn scalp_signal(data: &CoinData, i: usize) -> Option<Dir> {
    if i < 40 { return None; }
    if data.vol_ma[i].is_nan() || data.vol_ma[i] <= 0.0 { return None; }
    if data.rsi[i].is_nan() { return None; }

    let vol_r = data.vol[i] / data.vol_ma[i];
    let rsi_lo = SCALP_RSI_EXTREME;
    let rsi_hi = 100.0 - SCALP_RSI_EXTREME;

    // 1. vol_spike_rev
    if vol_r > SCALP_VOL_MULT {
        if data.rsi[i] < rsi_lo && f6_pass(data, i, Dir::Long) {
            return Some(Dir::Long);
        }
        if data.rsi[i] > rsi_hi && f6_pass(data, i, Dir::Short) {
            return Some(Dir::Short);
        }
    }

    // 2. stoch_cross
    if i >= 1 {
        let sk = data.stoch_k[i]; let sd = data.stoch_d[i];
        let skp = data.stoch_k[i - 1]; let sdp = data.stoch_d[i - 1];
        if !sk.is_nan() && !sd.is_nan() && !skp.is_nan() && !sdp.is_nan() {
            let lo = SCALP_STOCH_EXTREME;
            let hi = 100.0 - SCALP_STOCH_EXTREME;
            if skp <= sdp && sk > sd && sk < lo && sd < lo && f6_pass(data, i, Dir::Long) {
                return Some(Dir::Long);
            }
            if skp >= sdp && sk < sd && sk > hi && sd > hi && f6_pass(data, i, Dir::Short) {
                return Some(Dir::Short);
            }
        }
    }

    // 3. bb_squeeze_break
    if !data.bb_width_avg[i].is_nan() && data.bb_width_avg[i] > 0.0
        && !data.bb_upper[i].is_nan()
    {
        let squeeze = data.bb_width[i] < data.bb_width_avg[i] * SCALP_BB_SQUEEZE;
        if squeeze && vol_r > 2.0 {
            if data.close[i] > data.bb_upper[i] { return Some(Dir::Long); }
            if data.close[i] < data.bb_lower[i] { return Some(Dir::Short); }
        }
    }

    None
}

// ── Simulation ───────────────────────────────────────────────────────────────
#[derive(Serialize, Clone)]
struct CoinResult {
    coin: String,
    trades: usize,
    wins: usize,
    losses: usize,
    flat: usize,
    pnl: f64,
    avg_win: f64,
    avg_loss: f64,
}

fn simulate_coin(data: &CoinData, cfg: &ExitCfg) -> CoinResult {
    let n = data.close.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<ScalpPos> = None;
    let mut cooldown: u32 = 0;
    let mut win_pnls: Vec<f64> = Vec::new();
    let mut loss_pnls: Vec<f64> = Vec::new();
    let mut flat_count = 0usize;

    for i in 1..n {
        if let Some(ref mut p) = pos {
            let price = data.close[i];
            let pnl_pct = if p.dir == Dir::Long {
                (price - p.entry) / p.entry
            } else {
                (p.entry - price) / p.entry
            };

            // Update breakeven activation
            if cfg.be_act > 0.0 && pnl_pct >= cfg.be_act {
                p.be_active = true;
            }

            // Effective SL (0 if breakeven activated)
            let eff_sl = if p.be_active { 0.0 } else { cfg.sl };

            // Effective TP (time decay)
            let eff_tp = if cfg.decay[1].0 > 0 && p.bars_held >= cfg.decay[1].0 {
                cfg.decay[1].1
            } else if cfg.decay[0].0 > 0 && p.bars_held >= cfg.decay[0].0 {
                cfg.decay[0].1
            } else {
                cfg.tp
            };

            let mut closed = false;
            let mut exit_pnl_pct = 0.0;

            // SL check (stop fills at exactly SL level)
            if pnl_pct <= -eff_sl {
                exit_pnl_pct = if p.be_active { 0.0 } else { -cfg.sl };
                closed = true;
            }
            // TP check (limit fills at exactly TP level)
            else if pnl_pct >= eff_tp {
                exit_pnl_pct = eff_tp;
                closed = true;
            }
            // Indicator exits (while profitable, at market)
            else if pnl_pct > 0.0 {
                // RSI exit
                if cfg.rsi_long_exit > 0.0 && p.dir == Dir::Long
                    && !data.rsi[i].is_nan() && data.rsi[i] > cfg.rsi_long_exit
                {
                    exit_pnl_pct = pnl_pct;
                    closed = true;
                }
                if !closed && cfg.rsi_short_exit < 100.0 && p.dir == Dir::Short
                    && !data.rsi[i].is_nan() && data.rsi[i] < cfg.rsi_short_exit
                {
                    exit_pnl_pct = pnl_pct;
                    closed = true;
                }
                // Stochastic exit: stoch_k crosses 50 (extreme condition over)
                if !closed && cfg.stoch_exit && !data.stoch_k[i].is_nan() {
                    if p.dir == Dir::Long && data.stoch_k[i] > 50.0 {
                        exit_pnl_pct = pnl_pct;
                        closed = true;
                    }
                    if !closed && p.dir == Dir::Short && data.stoch_k[i] < 50.0 {
                        exit_pnl_pct = pnl_pct;
                        closed = true;
                    }
                }
            }

            // Increment bars held
            p.bars_held += 1;

            // MAX_HOLD check
            if !closed && p.bars_held >= cfg.max_hold {
                exit_pnl_pct = pnl_pct;
                closed = true;
            }

            if closed {
                let pnl_dollars = p.notional * exit_pnl_pct;
                bal += pnl_dollars;
                if pnl_dollars > 1e-10 {
                    win_pnls.push(pnl_dollars);
                } else if pnl_dollars < -1e-10 {
                    loss_pnls.push(pnl_dollars);
                } else {
                    flat_count += 1;
                }
                pos = None;
                cooldown = 2;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            // Check entry
            if let Some(dir) = scalp_signal(data, i) {
                let entry_price = data.close[i];
                if entry_price > 0.0 {
                    pos = Some(ScalpPos {
                        dir,
                        entry: entry_price,
                        notional: bal * SCALP_RISK * LEVERAGE,
                        bars_held: 0,
                        be_active: false,
                    });
                }
            }
        }
    }

    // Force-close any remaining position at last bar
    if let Some(ref p) = pos {
        let price = data.close[n - 1];
        let pnl_pct = if p.dir == Dir::Long {
            (price - p.entry) / p.entry
        } else {
            (p.entry - price) / p.entry
        };
        let pnl_dollars = p.notional * pnl_pct;
        bal += pnl_dollars;
        if pnl_dollars > 1e-10 { win_pnls.push(pnl_dollars); }
        else if pnl_dollars < -1e-10 { loss_pnls.push(pnl_dollars); }
        else { flat_count += 1; }
    }

    let total = win_pnls.len() + loss_pnls.len() + flat_count;
    CoinResult {
        coin: data.name.to_string(),
        trades: total,
        wins: win_pnls.len(),
        losses: loss_pnls.len(),
        flat: flat_count,
        pnl: bal - INITIAL_BAL,
        avg_win: if win_pnls.is_empty() { 0.0 }
            else { win_pnls.iter().sum::<f64>() / win_pnls.len() as f64 },
        avg_loss: if loss_pnls.is_empty() { 0.0 }
            else { loss_pnls.iter().sum::<f64>() / loss_pnls.len() as f64 },
    }
}

// ── Output ───────────────────────────────────────────────────────────────────
#[derive(Serialize)]
struct ConfigResult {
    label: String,
    desc: String,
    total_trades: usize,
    wins: usize,
    losses: usize,
    flat: usize,
    win_rate: f64,
    total_pnl: f64,
    avg_win: f64,
    avg_loss: f64,
    coins: Vec<CoinResult>,
}

#[derive(Serialize)]
struct Output {
    notes: String,
    configs: Vec<ConfigResult>,
}

// ── Entry point ──────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN35 — Scalp exit strategy grid search (1m, 1-year, 18 coins)");
    eprintln!("Zero-fee assumption. Entries: vol_spike_rev + stoch_cross + bb_squeeze_break (F6 filtered).");
    eprintln!();

    // Phase 1: Load 1m data
    eprintln!("Loading 1m data for {} coins...", N_COINS);
    let mut raw: Vec<Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_1m(name);
        if let Some(ref d) = loaded {
            eprintln!("  {} — {} bars", name, d.3.len());
        }
        raw.push(loaded);
    }

    // Check all coins loaded
    let mut all_ok = true;
    for (i, r) in raw.iter().enumerate() {
        if r.is_none() {
            eprintln!("ERROR: failed to load {}", COIN_NAMES[i]);
            all_ok = false;
        }
    }
    if !all_ok { return; }

    if shutdown.load(Ordering::SeqCst) { return; }

    // Phase 2: Compute indicators in parallel
    eprintln!("\nComputing 1m indicators for all coins...");
    let start = std::time::Instant::now();
    let coin_data: Vec<CoinData> = raw.into_par_iter().enumerate()
        .map(|(ci, r)| {
            let (o, h, l, c, v) = r.unwrap();
            compute_1m_data(COIN_NAMES[ci], o, h, l, c, v)
        })
        .collect();
    eprintln!("Indicators computed in {:.1}s", start.elapsed().as_secs_f64());

    if shutdown.load(Ordering::SeqCst) { return; }

    // Phase 3: Run grid search
    let grid = build_grid();
    eprintln!("\nSimulating {} exit configs × {} coins...", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label.to_string(), desc: cfg.desc.to_string(),
                total_trades: 0, wins: 0, losses: 0, flat: 0,
                win_rate: 0.0, total_pnl: 0.0, avg_win: 0.0, avg_loss: 0.0,
                coins: vec![],
            };
        }

        let coin_results: Vec<CoinResult> = coin_data.iter()
            .map(|cd| simulate_coin(cd, cfg))
            .collect();

        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let flat: usize = coin_results.iter().map(|c| c.flat).sum();
        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };

        // Compute portfolio avg_win and avg_loss
        let all_avg_win: f64 = coin_results.iter()
            .filter(|c| c.wins > 0)
            .map(|c| c.avg_win * c.wins as f64)
            .sum::<f64>();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let all_avg_loss: f64 = coin_results.iter()
            .filter(|c| c.losses > 0)
            .map(|c| c.avg_loss * c.losses as f64)
            .sum::<f64>();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();

        let avg_w = if total_wins > 0 { all_avg_win / total_wins as f64 } else { 0.0 };
        let avg_l = if total_losses > 0 { all_avg_loss / total_losses as f64 } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {:<16} {:>6} trades  WR:{:>5.1}%  PnL:${:>+8.2}",
            d, total_cfgs, cfg.label, total_trades, wr, total_pnl);

        ConfigResult {
            label: cfg.label.to_string(),
            desc: cfg.desc.to_string(),
            total_trades, wins, losses, flat,
            win_rate: wr, total_pnl,
            avg_win: avg_w, avg_loss: avg_l,
            coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("\nInterrupted — partial results not saved.");
        return;
    }

    // Phase 4: Print summary table
    eprintln!();
    println!("\n{:<18} {:>7} {:>6} {:>6} {:>5} {:>7} {:>8} {:>8} {:>10}",
        "Config", "Trades", "Wins", "Losses", "Flat", "WR%", "AvgWin", "AvgLoss", "Total$");
    println!("{}", "-".repeat(90));
    for r in &results {
        println!("{:<18} {:>7} {:>6} {:>6} {:>5} {:>6.1}% ${:>+7.4} ${:>+7.4} ${:>+9.2}",
            r.label, r.total_trades, r.wins, r.losses, r.flat,
            r.win_rate, r.avg_win, r.avg_loss, r.total_pnl);
    }
    println!("{}", "=".repeat(90));

    // Find best config
    let best = results.iter().max_by(|a, b| a.total_pnl.partial_cmp(&b.total_pnl).unwrap());
    if let Some(b) = best {
        println!("\nBest: {} ({}) — ${:+.2}, {:.1}% WR", b.label, b.desc, b.total_pnl, b.win_rate);
    }

    // Per-coin breakdown for top 3 configs
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a, b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap());
    println!("\n--- Per-coin breakdown (top 3 + baseline) ---");
    let baseline = results.iter().find(|r| r.label == "baseline");
    let mut show: Vec<&ConfigResult> = sorted.iter().take(3).cloned().collect();
    if let Some(bl) = baseline {
        if !show.iter().any(|r| r.label == "baseline") {
            show.push(bl);
        }
    }
    for r in &show {
        println!("\n  {} ({}): ${:+.2}, {:.1}% WR", r.label, r.desc, r.total_pnl, r.win_rate);
        println!("  {:<6} {:>5} {:>4} {:>4} {:>4} {:>8}", "Coin", "Trd", "Win", "Los", "Flt", "PnL$");
        for c in &r.coins {
            println!("  {:<6} {:>5} {:>4} {:>4} {:>4} ${:>+7.3}",
                c.coin, c.trades, c.wins, c.losses, c.flat, c.pnl);
        }
    }

    // Save JSON
    let notes = format!(
        "Scalp exit strategy grid search. {} coins, 1-year 1m data. \
         Zero-fee. SCALP_RISK={}% LEVERAGE={}x. \
         Entries: vol_spike_rev + stoch_cross + bb_squeeze_break (F6 filtered). \
         {} exit configs tested.",
        N_COINS, (SCALP_RISK * 100.0) as u32, LEVERAGE as u32, results.len()
    );
    let output = Output { notes, configs: results };
    let json = serde_json::to_string_pretty(&output).unwrap();
    let path = "/home/scamarena/ProjectCoin/run35_1_results.json";
    std::fs::write(path, &json).ok();
    eprintln!("\nSaved → {}", path);
}
