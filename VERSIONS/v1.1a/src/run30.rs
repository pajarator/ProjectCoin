/// RUN30 — Day-by-day scalp backtest: v9 vs v11 vs v14 vs v15
///
/// Simulates the scalp layer across 4 COINCLAW versions on 1-year 1m data.
/// Each calendar day resets to a $100/coin ($1800 total) starting portfolio.
/// Outputs a daily P&L table so you can see which version wins, day by day.
///
/// Version differences (scalp layer only — regime trades excluded):
///   v9:  TP=0.2%, no F6 filter, vol_spike_rev + stoch_cross + bb_squeeze_break, no cooldown, no cap
///   v11: TP=0.8%, F6 filter,    vol_spike_rev + stoch_cross,                    no cooldown, no cap
///   v14: TP=0.8%, F6 filter,    vol_spike_rev + stoch_cross,                    cooldown=300 bars, cap=3
///   v15: TP=0.2%, F6 filter,    vol_spike_rev + stoch_cross + bb_squeeze_break, no cooldown, no cap
///
/// NOTE: v14 direction-matching (scalps must match regime) is NOT simulated → v14 is optimistic.
///
/// Run: cargo run --release --features run30 -- --run30

use rayon::prelude::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ── Trade simulation constants ────────────────────────────────────────────────
const FEE: f64       = 0.001;   // 0.10% per side (Binance maker/taker)
const SLIP: f64      = 0.0005;  // 0.05% per side
const MARGIN: f64    = 5.0;     // $5 margin per scalp
const LEVERAGE: f64  = 5.0;     // 5× → $25 notional per trade
const DEFAULT_MAX_HOLD: usize = 60; // baseline; overridden per VersionCfg

// ── Signal constants (match live COINCLAW) ───────────────────────────────────
const VOL_MA_PERIOD:    usize = 20;
const RSI_PERIOD:       usize = 14;
const STOCH_PERIOD:     usize = 14;
const BB_PERIOD:        usize = 20;
const BB_STD:           f64   = 2.0;
const BB_WIDTH_MA:      usize = 20;

const SCALP_VOL_MULT:      f64 = 3.5;
const SCALP_RSI_EXTREME:   f64 = 20.0;
const SCALP_STOCH_EXTREME: f64 = 5.0;
const SCALP_BB_SQUEEZE:    f64 = 0.4;   // bb_width < 0.4 × avg → squeeze
const SCALP_BB_VOL_MULT:   f64 = 2.0;   // min vol multiplier for squeeze break
const F6_ROC_3:            f64 = -0.195;
const F6_BODY_3:           f64 = 0.072;

const LIMIT_OFFSET: f64   = 0.0005; // 0.05% inside spread for limit entry (opt2)
const LIMIT_EXPIRY: usize = 3;      // bars to wait for limit fill before cancelling

const COINS: [&str; 18] = [
    "ATOM", "UNI", "LTC", "AVAX", "NEAR", "BNB", "XRP", "DOGE",
    "DOT",  "SOL", "BTC", "ALGO", "ETH",  "SHIB","ADA", "DASH", "XLM", "LINK",
];

// ── Version configurations ────────────────────────────────────────────────────
#[derive(Clone)]
struct VersionCfg {
    name:           &'static str,
    tp_pct:         f64,
    sl_pct:         f64,
    use_f6:         bool,
    use_bb_squeeze: bool,
    cooldown_bars:  usize,  // 0 = no cooldown
    cap:            usize,  // 999 = unlimited
    max_hold:       usize,  // bars before force-close
    entry_fee:      f64,    // entry-side fee: FEE = taker market, 0.0 = maker/zero-fee
    use_limit:      bool,   // true = attempt limit fill LIMIT_EXPIRY bars before entering
}

fn versions() -> Vec<VersionCfg> {
    vec![
        VersionCfg { name: "v9",   tp_pct: 0.002, sl_pct: 0.001, use_f6: false, use_bb_squeeze: true,  cooldown_bars: 0,   cap: 999, max_hold: DEFAULT_MAX_HOLD, entry_fee: FEE, use_limit: false },
        VersionCfg { name: "v11",  tp_pct: 0.008, sl_pct: 0.001, use_f6: true,  use_bb_squeeze: false, cooldown_bars: 0,   cap: 999, max_hold: DEFAULT_MAX_HOLD, entry_fee: FEE, use_limit: false },
        VersionCfg { name: "v14",  tp_pct: 0.008, sl_pct: 0.001, use_f6: true,  use_bb_squeeze: false, cooldown_bars: 300, cap: 3,   max_hold: DEFAULT_MAX_HOLD, entry_fee: FEE, use_limit: false },
        VersionCfg { name: "v15",  tp_pct: 0.002, sl_pct: 0.001, use_f6: true,  use_bb_squeeze: true,  cooldown_bars: 0,   cap: 999, max_hold: DEFAULT_MAX_HOLD, entry_fee: FEE, use_limit: false },
        // RUN31 Option 1: zero-fee exchange (e.g. Bitfinex), TP=2.5%, MAX_HOLD=480
        VersionCfg { name: "opt1", tp_pct: 0.025, sl_pct: 0.001, use_f6: true,  use_bb_squeeze: true,  cooldown_bars: 0,   cap: 999, max_hold: 480,              entry_fee: 0.0, use_limit: false },
        // RUN31 Option 2: Binance limit entry (maker=0%), TP=2.5%, MAX_HOLD=480
        // Limit enters 0.05% inside spread → effective RT fee ≈ 0.10% (vs 0.15% market)
        VersionCfg { name: "opt2", tp_pct: 0.025, sl_pct: 0.001, use_f6: true,  use_bb_squeeze: true,  cooldown_bars: 0,   cap: 999, max_hold: 480,              entry_fee: 0.0, use_limit: true  },
    ]
}

// ── Data structures ───────────────────────────────────────────────────────────
#[derive(Clone)]
struct Bar { o: f64, h: f64, l: f64, c: f64, v: f64 }

#[derive(Clone)]
struct Ind {
    rsi:          f64,
    vol:          f64,
    vol_ma:       f64,
    stoch_k:      f64,
    stoch_d:      f64,
    stoch_k_prev: f64,
    stoch_d_prev: f64,
    roc_3:        f64,
    avg_body_3:   f64,
    bb_upper:     f64,
    bb_lower:     f64,
    bb_width:     f64,
    bb_width_avg: f64,
    sma20:        f64,
    valid:        bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Dir { Long, Short }

#[derive(Clone)]
struct Position { dir: Dir, entry: f64, bars_held: usize }

// ── Rolling helpers (private to this module) ─────────────────────────────────
fn rolling_mean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let mut sum = 0.0_f64;
    for i in 0..n {
        sum += data[i];
        if i >= w { sum -= data[i - w]; }
        if i + 1 >= w { out[i] = sum / w as f64; }
    }
    out
}

fn rolling_std(data: &[f64], w: usize) -> Vec<f64> {
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

fn rolling_min(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::INFINITY, f64::min);
    }
    out
}

fn rolling_max(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    }
    out
}

fn rsi_calc(closes: &[f64], period: usize) -> Vec<f64> {
    let n = closes.len();
    let mut out = vec![f64::NAN; n];
    if n <= period { return out; }
    let mut avg_g = 0.0_f64;
    let mut avg_l = 0.0_f64;
    for i in 1..=period {
        let d = closes[i] - closes[i - 1];
        if d > 0.0 { avg_g += d; } else { avg_l -= d; }
    }
    avg_g /= period as f64;
    avg_l /= period as f64;
    out[period] = if avg_l == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + avg_g / avg_l) };
    for i in (period + 1)..n {
        let d = closes[i] - closes[i - 1];
        let g = if d > 0.0 { d } else { 0.0 };
        let l = if d < 0.0 { -d } else { 0.0 };
        avg_g = (avg_g * (period - 1) as f64 + g) / period as f64;
        avg_l = (avg_l * (period - 1) as f64 + l) / period as f64;
        out[i] = if avg_l == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + avg_g / avg_l) };
    }
    out
}

// ── Load 1m CSV ───────────────────────────────────────────────────────────────
// Returns (bars, day_keys) where day_key = YYYYMMDD integer
fn load_1m(coin: &str) -> (Vec<Bar>, Vec<u32>) {
    let path = format!(
        "/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_1year.csv",
        coin
    );
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => { eprintln!("  Missing {}: {}", path, e); return (vec![], vec![]); }
    };
    let mut bars = Vec::with_capacity(530_000);
    let mut days = Vec::with_capacity(530_000);
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let ts = it.next().unwrap_or("");
        let o: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let h: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let l: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let c: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let v: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() || v.is_nan() { continue; }
        // Parse "YYYY-MM-DD HH:MM:SS" → YYYYMMDD integer
        let day_key: u32 = if ts.len() >= 10 {
            let y: u32 = ts[0..4].parse().unwrap_or(0);
            let m: u32 = ts[5..7].parse().unwrap_or(0);
            let d: u32 = ts[8..10].parse().unwrap_or(0);
            y * 10000 + m * 100 + d
        } else { 0 };
        bars.push(Bar { o, h, l, c, v });
        days.push(day_key);
    }
    (bars, days)
}

// ── Compute all indicators ────────────────────────────────────────────────────
fn compute_indicators(bars: &[Bar]) -> Vec<Ind> {
    let n = bars.len();
    let c: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let h: Vec<f64> = bars.iter().map(|b| b.h).collect();
    let l: Vec<f64> = bars.iter().map(|b| b.l).collect();
    let v: Vec<f64> = bars.iter().map(|b| b.v).collect();
    let o: Vec<f64> = bars.iter().map(|b| b.o).collect();

    let rsi14     = rsi_calc(&c, RSI_PERIOD);
    let vol_ma    = rolling_mean(&v, VOL_MA_PERIOD);
    let sma20     = rolling_mean(&c, 20);
    let ll14      = rolling_min(&l, STOCH_PERIOD);
    let hh14      = rolling_max(&h, STOCH_PERIOD);
    let bb_ma     = rolling_mean(&c, BB_PERIOD);
    let bb_std    = rolling_std(&c, BB_PERIOD);

    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n {
        if !ll14[i].is_nan() && !hh14[i].is_nan() {
            let range = hh14[i] - ll14[i];
            if range > 0.0 { stoch_k[i] = 100.0 * (c[i] - ll14[i]) / range; }
        }
    }
    let mut stoch_d = vec![f64::NAN; n];
    for i in 2..n {
        if !stoch_k[i].is_nan() && !stoch_k[i-1].is_nan() && !stoch_k[i-2].is_nan() {
            stoch_d[i] = (stoch_k[i] + stoch_k[i-1] + stoch_k[i-2]) / 3.0;
        }
    }

    let mut bb_upper    = vec![f64::NAN; n];
    let mut bb_lower    = vec![f64::NAN; n];
    let mut bb_width_v  = vec![f64::NAN; n];
    for i in 0..n {
        if !bb_ma[i].is_nan() && !bb_std[i].is_nan() && c[i] > 0.0 {
            bb_upper[i]   = bb_ma[i] + BB_STD * bb_std[i];
            bb_lower[i]   = bb_ma[i] - BB_STD * bb_std[i];
            bb_width_v[i] = (bb_upper[i] - bb_lower[i]) / c[i];
        }
    }
    let bb_width_avg = rolling_mean(&bb_width_v, BB_WIDTH_MA);

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let roc_3 = if i >= 3 && c[i-3] > 0.0 {
            (c[i] - c[i-3]) / c[i-3] * 100.0
        } else { 0.0 };
        let avg_body_3 = if i >= 3 {
            (0..3usize).map(|k| {
                let idx = i - k;
                if c[idx] > 0.0 { (c[idx] - o[idx]).abs() / c[idx] * 100.0 } else { 0.0 }
            }).sum::<f64>() / 3.0
        } else { 0.0 };

        let stk_prev = if i > 0 { stoch_k[i-1] } else { f64::NAN };
        let std_prev = if i > 0 { stoch_d[i-1] } else { f64::NAN };

        // warm-up: need at least max(RSI14, BB20, StochD, vol_ma20, bb_width_ma20) + safety margin
        let valid = i >= 60
            && !rsi14[i].is_nan()
            && !vol_ma[i].is_nan()
            && !stoch_k[i].is_nan()
            && !stoch_d[i].is_nan()
            && vol_ma[i] > 0.0;

        out.push(Ind {
            rsi: rsi14[i], vol: v[i], vol_ma: vol_ma[i],
            stoch_k: stoch_k[i], stoch_d: stoch_d[i],
            stoch_k_prev: stk_prev, stoch_d_prev: std_prev,
            roc_3, avg_body_3,
            bb_upper: bb_upper[i], bb_lower: bb_lower[i],
            bb_width: bb_width_v[i], bb_width_avg: bb_width_avg[i],
            sma20: sma20[i],
            valid,
        });
    }
    out
}

// ── F6 filter ─────────────────────────────────────────────────────────────────
#[inline]
fn passes_f6(ind: &Ind, dir: Dir) -> bool {
    match dir {
        Dir::Long  => !(ind.roc_3 <  F6_ROC_3 && ind.avg_body_3 > F6_BODY_3),
        Dir::Short => !(ind.roc_3 > -F6_ROC_3 && ind.avg_body_3 > F6_BODY_3),
    }
}

// ── Signal detection per version ──────────────────────────────────────────────
fn scalp_signal(ind: &Ind, price: f64, ver: &VersionCfg) -> Option<Dir> {
    if !ind.valid || ind.vol_ma == 0.0 { return None; }
    let vol_r  = ind.vol / ind.vol_ma;
    let rsi_lo = SCALP_RSI_EXTREME;
    let rsi_hi = 100.0 - SCALP_RSI_EXTREME;
    let st_lo  = SCALP_STOCH_EXTREME;
    let st_hi  = 100.0 - SCALP_STOCH_EXTREME;

    // 1. vol_spike_rev
    if vol_r > SCALP_VOL_MULT {
        if ind.rsi < rsi_lo && (!ver.use_f6 || passes_f6(ind, Dir::Long))  { return Some(Dir::Long); }
        if ind.rsi > rsi_hi && (!ver.use_f6 || passes_f6(ind, Dir::Short)) { return Some(Dir::Short); }
    }
    // 2. stoch_cross
    if !ind.stoch_k_prev.is_nan() && !ind.stoch_d_prev.is_nan() {
        if ind.stoch_k_prev <= ind.stoch_d_prev && ind.stoch_k > ind.stoch_d
            && ind.stoch_k < st_lo && ind.stoch_d < st_lo
            && (!ver.use_f6 || passes_f6(ind, Dir::Long))
        { return Some(Dir::Long); }
        if ind.stoch_k_prev >= ind.stoch_d_prev && ind.stoch_k < ind.stoch_d
            && ind.stoch_k > st_hi && ind.stoch_d > st_hi
            && (!ver.use_f6 || passes_f6(ind, Dir::Short))
        { return Some(Dir::Short); }
    }
    // 3. bb_squeeze_break (v9 / v15 only — no F6 filter, breakout signal)
    if ver.use_bb_squeeze
        && !ind.bb_width_avg.is_nan() && ind.bb_width_avg > 0.0
        && !ind.bb_upper.is_nan() && !ind.bb_lower.is_nan()
    {
        let squeeze = ind.bb_width < ind.bb_width_avg * SCALP_BB_SQUEEZE;
        if squeeze && vol_r > SCALP_BB_VOL_MULT {
            if price > ind.bb_upper { return Some(Dir::Long); }
            if price < ind.bb_lower { return Some(Dir::Short); }
        }
    }
    None
}

// ── Simulate one version: returns (day_pnl, trades, wins) ────────────────────
fn simulate_version(
    ver: &VersionCfg,
    all_inds: &[Vec<Ind>],
    all_bars: &[Vec<Bar>],
    all_days: &[Vec<u32>],
    ordered_days: &[u32],
) -> (HashMap<u32, f64>, usize, usize) {
    // Fee regime:
    //   entry_fee=0.0 + use_limit=false → opt1 (zero-fee exchange): no exit fees either
    //   entry_fee=0.0 + use_limit=true  → opt2 (limit maker entry): standard taker exit
    //   entry_fee=FEE + use_limit=false → standard market order: standard taker exit
    let (ef, es) = if !ver.use_limit && ver.entry_fee == 0.0 {
        (0.0_f64, 0.0_f64)  // zero-fee exchange
    } else {
        (FEE, SLIP)          // standard taker exit (limit saves on entry price, not exit fee)
    };

    let notional   = MARGIN * LEVERAGE; // $25
    let n_coins    = all_inds.len();
    let n_bars     = all_inds[0].len();
    let mut total_trades = 0usize;
    let mut total_wins   = 0usize;

    let mut positions: Vec<Option<Position>> = vec![None; n_coins];
    let mut cooldowns: Vec<usize>            = vec![0; n_coins];
    let mut daily_pnl: HashMap<u32, f64>    = ordered_days.iter().map(|&d| (d, 0.0)).collect();

    let ref_days = &all_days[0]; // use coin-0 as reference for day boundaries
    let mut current_day = if !ref_days.is_empty() { ref_days[0] } else { 0 };

    for bar_i in 1..n_bars {
        let bar_day = ref_days[bar_i];

        // ── Day boundary: force-close all open positions ───────────────────
        if bar_day != current_day {
            for ci in 0..n_coins {
                if let Some(ref pos) = positions[ci] {
                    let px = all_bars[ci][bar_i - 1].c; // last close before midnight
                    let pnl_frac = match pos.dir {
                        Dir::Long  =>  (px / pos.entry - 1.0) - ef - es,
                        Dir::Short => -(px / pos.entry - 1.0) - ef - es,
                    };
                    *daily_pnl.entry(current_day).or_insert(0.0) += pnl_frac * notional;
                    positions[ci] = None;
                }
                cooldowns[ci] = 0; // cooldown resets with the day
            }
            current_day = bar_day;
        }

        // ── Check exits ───────────────────────────────────────────────────
        for ci in 0..n_coins {
            if let Some(ref pos) = positions[ci] {
                let bar   = &all_bars[ci][bar_i];
                let entry = pos.entry;
                let tp    = entry * if pos.dir == Dir::Long { 1.0 + ver.tp_pct } else { 1.0 - ver.tp_pct };
                let sl    = entry * if pos.dir == Dir::Long { 1.0 - ver.sl_pct } else { 1.0 + ver.sl_pct };

                let result = match pos.dir {
                    Dir::Long => {
                        if bar.l <= sl        { Some((-(ver.sl_pct + ef + es), true)) }
                        else if bar.h >= tp   { Some((ver.tp_pct - ef - es, false)) }
                        else if pos.bars_held >= ver.max_hold { Some(((bar.c / entry - 1.0) - ef - es, false)) }
                        else { None }
                    }
                    Dir::Short => {
                        if bar.h >= sl        { Some((-(ver.sl_pct + ef + es), true)) }
                        else if bar.l <= tp   { Some((ver.tp_pct - ef - es, false)) }
                        else if pos.bars_held >= ver.max_hold { Some((-(bar.c / entry - 1.0) - ef - es, false)) }
                        else { None }
                    }
                };

                if let Some((pnl_frac, is_sl)) = result {
                    *daily_pnl.entry(bar_day).or_insert(0.0) += pnl_frac * notional;
                    total_trades += 1;
                    if pnl_frac > 0.0 { total_wins += 1; }
                    positions[ci] = None;
                    if is_sl && ver.cooldown_bars > 0 {
                        cooldowns[ci] = ver.cooldown_bars;
                    }
                } else {
                    positions[ci].as_mut().unwrap().bars_held += 1;
                }
            }
            if cooldowns[ci] > 0 { cooldowns[ci] -= 1; }
        }

        // ── Collect signals ───────────────────────────────────────────────
        let mut signals: Vec<(usize, Dir)> = Vec::new();
        for ci in 0..n_coins {
            if positions[ci].is_none() && cooldowns[ci] == 0 {
                let ind   = &all_inds[ci][bar_i];
                let price = all_bars[ci][bar_i].c;
                if let Some(dir) = scalp_signal(ind, price, ver) {
                    signals.push((ci, dir));
                }
            }
        }

        // ── Apply cap and open positions ──────────────────────────────────
        // Market: entry at next bar's open (bar_i+1).
        // Limit:  try to fill at limit_price on bars bar_i+1..=bar_i+LIMIT_EXPIRY.
        //         If not filled within expiry, skip the trade (conservative).
        let effective_cap = ver.cap.min(n_coins);
        if bar_i + 1 < n_bars {
            for &(ci, dir) in signals.iter().take(effective_cap) {
                let entry = if ver.use_limit {
                    let signal_px = all_bars[ci][bar_i].c;
                    let limit = match dir {
                        Dir::Long  => signal_px * (1.0 - LIMIT_OFFSET),
                        Dir::Short => signal_px * (1.0 + LIMIT_OFFSET),
                    };
                    let filled = (1..=LIMIT_EXPIRY).find_map(|k| {
                        let b = all_bars[ci].get(bar_i + k)?;
                        match dir {
                            Dir::Long  if b.l <= limit => Some(limit),
                            Dir::Short if b.h >= limit => Some(limit),
                            _ => None,
                        }
                    });
                    match filled {
                        Some(p) => p,
                        None    => continue,  // limit expired, skip this trade
                    }
                } else {
                    all_bars[ci][bar_i + 1].o
                };
                if entry > 0.0 {
                    positions[ci] = Some(Position { dir, entry, bars_held: 0 });
                }
            }
        }
    }

    (daily_pnl, total_trades, total_wins)
}

// ── Output structs ────────────────────────────────────────────────────────────
#[derive(Serialize)]
struct DailyRow {
    date: String,
    v9:   f64,
    v11:  f64,
    v14:  f64,
    v15:  f64,
    opt1: f64,
    opt2: f64,
}

#[derive(Serialize)]
struct VersionSummary {
    version:       &'static str,
    total_pnl:     f64,
    avg_daily_pnl: f64,
    best_day:      f64,
    worst_day:     f64,
    positive_days: usize,
    negative_days: usize,
    flat_days:     usize,
    total_days:    usize,
}

#[derive(Serialize)]
struct Output {
    notes:   String,
    daily:   Vec<DailyRow>,
    summary: Vec<VersionSummary>,
}

// ── TP grid sweep ─────────────────────────────────────────────────────────────
// Tests TP=[0.2%..3%] with proportional MAX_HOLD on F6-filtered reversal signals.
// Runs twice: once with FEE/SLIP, once at zero cost.
fn run_tp_grid(
    all_inds: &[Vec<Ind>],
    all_bars: &[Vec<Bar>],
    all_days: &[Vec<u32>],
    ordered_days: &[u32],
) {
    // TP values to test (reversal signals only: vol_spike_rev + stoch_cross, F6 filter)
    let tp_grid: &[f64] = &[0.002, 0.004, 0.006, 0.008, 0.010, 0.012, 0.015, 0.020, 0.025, 0.030];

    // Two fee scenarios
    let fee_scenarios: &[(&str, f64, f64)] = &[
        ("0%",     0.0,   0.0   ),
        ("0.15%",  0.001, 0.0005),
    ];

    println!("\n=== TP/MAX_HOLD Grid Search — F6 reversal signals (vol_spike_rev + stoch_cross) ===");
    println!("SL fixed at 0.10%.  MAX_HOLD = max(60, round(TP/0.2% × 60)), capped at 480 bars.");
    println!("18 coins, $5 margin × 5× leverage = $25 notional per trade.\n");

    for &(fee_label, fee, slip) in fee_scenarios {
        println!("─── Fees: {} round-trip ─────────────────────────────────────────────", fee_label);
        println!("{:>6}  {:>5}  {:>8}  {:>6}  {:>10}  {:>10}  {:>10}  {:>6}",
            "TP%", "Hold", "Trades", "WR%", "EV/trade$", "Annual$", "$/day", "+days");
        println!("{}", "-".repeat(72));

        let results: Vec<_> = tp_grid.par_iter().map(|&tp| {
            let max_hold = ((tp / 0.002 * 60.0).round() as usize).max(60).min(480);
            let ver = VersionCfg {
                name:          "tp_test",
                tp_pct:        tp,
                sl_pct:        0.001,
                use_f6:        true,
                use_bb_squeeze: false,   // pure reversal signals
                cooldown_bars: 0,
                cap:           999,
                max_hold,
                entry_fee:     FEE,
                use_limit:     false,
            };

            // Temporarily override fee constants by adjusting the pnl formula locally
            // We pass fee/slip as parameters to a local simulation variant
            let (daily, trades, wins) = simulate_with_fees(&ver, all_inds, all_bars, all_days, ordered_days, fee, slip);
            let total_pnl  = ordered_days.iter().map(|d| daily.get(d).cloned().unwrap_or(0.0)).sum::<f64>();
            let avg_day    = total_pnl / ordered_days.len() as f64;
            let pos_days   = ordered_days.iter().filter(|d| daily.get(d).cloned().unwrap_or(0.0) > 1e-6).count();
            let wr         = if trades > 0 { wins as f64 / trades as f64 } else { 0.0 };
            let ev_trade   = if trades > 0 { total_pnl / trades as f64 * 365.0 / ordered_days.len() as f64 } else { 0.0 };
            (tp, max_hold, trades, wr, ev_trade, total_pnl, avg_day, pos_days)
        }).collect();

        // Sort by tp (par_iter doesn't preserve order)
        let mut results = results;
        results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        for (tp, max_hold, trades, wr, ev_trade, total_pnl, avg_day, pos_days) in &results {
            println!("{:>5.1}%  {:>5}  {:>8}  {:>5.1}%  {:>+10.4}  {:>+10.3}  {:>+10.4}  {:>6}",
                tp * 100.0, max_hold, trades, wr * 100.0, ev_trade,
                total_pnl, avg_day, pos_days);
        }
        println!();
    }

    // Save best TP recommendation
    let path = "/home/scamarena/ProjectCoin/run30_tp_grid.json";
    eprintln!("Grid complete. Results printed above.");
    let _ = path; // JSON save omitted for brevity; table is the output
}

// Variant of simulate_version with explicit fee/slip parameters (avoids recompiling)
fn simulate_with_fees(
    ver: &VersionCfg,
    all_inds: &[Vec<Ind>],
    all_bars: &[Vec<Bar>],
    all_days: &[Vec<u32>],
    ordered_days: &[u32],
    fee: f64,
    slip: f64,
) -> (HashMap<u32, f64>, usize, usize) {
    let notional     = MARGIN * LEVERAGE;
    let n_coins      = all_inds.len();
    let n_bars       = all_inds[0].len();
    let mut positions: Vec<Option<Position>> = vec![None; n_coins];
    let mut cooldowns: Vec<usize>            = vec![0; n_coins];
    let mut daily_pnl: HashMap<u32, f64>    = ordered_days.iter().map(|&d| (d, 0.0)).collect();
    let mut total_trades = 0usize;
    let mut total_wins   = 0usize;

    let ref_days   = &all_days[0];
    let mut cur_day = if !ref_days.is_empty() { ref_days[0] } else { 0 };

    for bar_i in 1..n_bars {
        let bar_day = ref_days[bar_i];

        if bar_day != cur_day {
            for ci in 0..n_coins {
                if let Some(ref pos) = positions[ci] {
                    let px = all_bars[ci][bar_i - 1].c;
                    let pf = match pos.dir {
                        Dir::Long  =>  (px / pos.entry - 1.0) - fee - slip,
                        Dir::Short => -(px / pos.entry - 1.0) - fee - slip,
                    };
                    *daily_pnl.entry(cur_day).or_insert(0.0) += pf * notional;
                    positions[ci] = None;
                }
                cooldowns[ci] = 0;
            }
            cur_day = bar_day;
        }

        for ci in 0..n_coins {
            if let Some(ref pos) = positions[ci] {
                let bar   = &all_bars[ci][bar_i];
                let entry = pos.entry;
                let tp_p  = entry * if pos.dir == Dir::Long { 1.0 + ver.tp_pct } else { 1.0 - ver.tp_pct };
                let sl_p  = entry * if pos.dir == Dir::Long { 1.0 - ver.sl_pct } else { 1.0 + ver.sl_pct };

                let result = match pos.dir {
                    Dir::Long => {
                        if bar.l <= sl_p      { Some((-(ver.sl_pct + fee + slip), true)) }
                        else if bar.h >= tp_p { Some((ver.tp_pct - fee - slip, false)) }
                        else if pos.bars_held >= ver.max_hold { Some(((bar.c / entry - 1.0) - fee - slip, false)) }
                        else { None }
                    }
                    Dir::Short => {
                        if bar.h >= sl_p      { Some((-(ver.sl_pct + fee + slip), true)) }
                        else if bar.l <= tp_p { Some((ver.tp_pct - fee - slip, false)) }
                        else if pos.bars_held >= ver.max_hold { Some((-(bar.c / entry - 1.0) - fee - slip, false)) }
                        else { None }
                    }
                };

                if let Some((pf, is_sl)) = result {
                    *daily_pnl.entry(bar_day).or_insert(0.0) += pf * notional;
                    total_trades += 1;
                    if pf > 0.0 { total_wins += 1; }
                    positions[ci] = None;
                    if is_sl && ver.cooldown_bars > 0 { cooldowns[ci] = ver.cooldown_bars; }
                } else {
                    positions[ci].as_mut().unwrap().bars_held += 1;
                }
            }
            if cooldowns[ci] > 0 { cooldowns[ci] -= 1; }
        }

        let mut signals: Vec<(usize, Dir)> = Vec::new();
        for ci in 0..n_coins {
            if positions[ci].is_none() && cooldowns[ci] == 0 {
                let ind   = &all_inds[ci][bar_i];
                let price = all_bars[ci][bar_i].c;
                if let Some(dir) = scalp_signal(ind, price, ver) {
                    signals.push((ci, dir));
                }
            }
        }
        let eff_cap = ver.cap.min(n_coins);
        if bar_i + 1 < n_bars {
            for &(ci, dir) in signals.iter().take(eff_cap) {
                let entry = all_bars[ci][bar_i + 1].o;
                if entry > 0.0 {
                    positions[ci] = Some(Position { dir, entry, bars_held: 0 });
                }
            }
        }
    }
    (daily_pnl, total_trades, total_wins)
}

// ── Entry point ───────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN30 — Day-by-day scalp version comparison (v9 / v11 / v14 / v15)");
    eprintln!("Loading 1m data for {} coins...", COINS.len());

    let mut all_bars_raw: Vec<Vec<Bar>>  = Vec::new();
    let mut all_days_raw: Vec<Vec<u32>>  = Vec::new();
    for coin in &COINS {
        let (bars, days) = load_1m(coin);
        eprintln!("  {} — {} bars", coin, bars.len());
        all_bars_raw.push(bars);
        all_days_raw.push(days);
    }

    // Align to shortest series
    let min_len = all_bars_raw.iter().map(|b| b.len()).min().unwrap_or(0);
    if min_len < 1440 {
        eprintln!("ERROR: insufficient data ({} bars)", min_len);
        return;
    }
    let all_bars: Vec<Vec<Bar>> = all_bars_raw.into_iter()
        .map(|b| b[b.len() - min_len..].to_vec())
        .collect();
    let all_days: Vec<Vec<u32>> = all_days_raw.into_iter()
        .map(|d| d[d.len() - min_len..].to_vec())
        .collect();
    eprintln!("Aligned to {} bars (~{:.1} days)", min_len, min_len as f64 / 1440.0);

    // Build sorted unique day list from coin-0
    let mut seen = HashSet::new();
    let mut ordered_days: Vec<u32> = all_days[0].iter()
        .filter(|&&d| d > 0 && seen.insert(d))
        .cloned()
        .collect();
    ordered_days.sort();

    // Optional: restrict to last N days
    let last_month = std::env::args().any(|a| a == "--last-month");
    if last_month {
        let keep = 31usize.min(ordered_days.len());
        let skip = ordered_days.len() - keep;
        ordered_days = ordered_days[skip..].to_vec();
        eprintln!("--last-month: showing last {} days", ordered_days.len());
    } else {
        eprintln!("Trading days: {}", ordered_days.len());
    }

    eprintln!("Computing indicators in parallel...");
    let all_inds: Vec<Vec<Ind>> = all_bars.par_iter()
        .map(|bars| compute_indicators(bars))
        .collect();

    // ── TP grid mode ──────────────────────────────────────────────────────────
    let tp_grid_mode = std::env::args().any(|a| a == "--tp-grid");
    if tp_grid_mode {
        run_tp_grid(&all_inds, &all_bars, &all_days, &ordered_days);
        return;
    }

    let vers = versions();
    eprintln!("Simulating {} versions...", vers.len());
    let results: Vec<(&str, HashMap<u32, f64>, usize, usize)> = vers.par_iter()
        .filter_map(|ver| {
            if shutdown.load(Ordering::SeqCst) { return None; }
            eprintln!("  {} running...", ver.name);
            let (daily, trades, wins) = simulate_version(ver, &all_inds, &all_bars, &all_days, &ordered_days);
            eprintln!("  {} done ({} trades, {:.1}% WR)", ver.name, trades,
                if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 });
            Some((ver.name, daily, trades, wins))
        })
        .collect();

    if results.len() < vers.len() {
        eprintln!("Interrupted before all versions completed");
        return;
    }

    // Collect into maps by version name
    let mut trade_counts: HashMap<&str, (usize, usize)> = HashMap::new();
    let result_map: HashMap<&str, HashMap<u32, f64>> = results.into_iter().map(|(name, daily, trades, wins)| {
        trade_counts.insert(name, (trades, wins));
        (name, daily)
    }).collect();

    // ── Build daily table ──────────────────────────────────────────────────────
    let get = |name: &str, day: &u32| -> f64 {
        result_map.get(name).and_then(|m| m.get(day)).cloned().unwrap_or(0.0)
    };
    let mut daily_rows: Vec<DailyRow> = Vec::new();
    for &day in &ordered_days {
        let y = day / 10000;
        let m = (day % 10000) / 100;
        let d = day % 100;
        daily_rows.push(DailyRow {
            date: format!("{:04}-{:02}-{:02}", y, m, d),
            v9:   get("v9",   &day),
            v11:  get("v11",  &day),
            v14:  get("v14",  &day),
            v15:  get("v15",  &day),
            opt1: get("opt1", &day),
            opt2: get("opt2", &day),
        });
    }

    // ── Print table ───────────────────────────────────────────────────────────
    println!("\n{:<12} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "Date", "v9 ($)", "v11 ($)", "v14 ($)", "v15 ($)", "opt1($)", "opt2($)");
    println!("{}", "-".repeat(76));
    for row in &daily_rows {
        println!("{:<12} {:>+10.3}  {:>+10.3}  {:>+10.3}  {:>+10.3}  {:>+10.3}  {:>+10.3}",
            row.date, row.v9, row.v11, row.v14, row.v15, row.opt1, row.opt2);
    }
    println!("{}", "=".repeat(76));

    // ── Summary per version ───────────────────────────────────────────────────
    let mut summaries: Vec<VersionSummary> = Vec::new();
    for &vn in &["v9", "v11", "v14", "v15", "opt1", "opt2"] {
        let m = &result_map[vn];
        let pnls: Vec<f64> = ordered_days.iter().map(|d| m.get(d).cloned().unwrap_or(0.0)).collect();
        let total   = pnls.iter().sum::<f64>();
        let avg     = total / pnls.len() as f64;
        let best    = pnls.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let worst   = pnls.iter().cloned().fold(f64::INFINITY, f64::min);
        let pos     = pnls.iter().filter(|&&p| p > 1e-6).count();
        let neg     = pnls.iter().filter(|&&p| p < -1e-6).count();
        let flat    = pnls.len() - pos - neg;
        let (trades, wins) = trade_counts.get(vn).cloned().unwrap_or((0,0));
        let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
        println!("{}: total={:+.3}  avg/day={:+.4}  best={:+.3}  worst={:+.3}  +{}/-{}/~{}  trades={}  WR={:.1}%",
            vn, total, avg, best, worst, pos, neg, flat, trades, wr);
        summaries.push(VersionSummary {
            version: vn, total_pnl: total, avg_daily_pnl: avg,
            best_day: best, worst_day: worst,
            positive_days: pos, negative_days: neg, flat_days: flat,
            total_days: pnls.len(),
        });
    }

    // ── Save JSON ─────────────────────────────────────────────────────────────
    let notes = format!(
        "Scalp-only backtest. {} coins, {:.0} days. \
         Notional=${:.0} (${}margin×{}x lev). \
         Fee={:.2}%/side Slip={:.2}%/side. MaxHold={}bars. \
         v14 direction-matching NOT simulated (v14 results optimistic).",
        COINS.len(), ordered_days.len() as f64,
        MARGIN * LEVERAGE, MARGIN as u32, LEVERAGE as u32,
        FEE * 100.0, SLIP * 100.0, DEFAULT_MAX_HOLD
    );
    let output = Output { notes, daily: daily_rows, summary: summaries };
    let json = serde_json::to_string_pretty(&output).unwrap();
    let path = "/home/scamarena/ProjectCoin/run30_1_results.json";
    std::fs::write(path, &json).ok();
    eprintln!("Saved → {}", path);
}
