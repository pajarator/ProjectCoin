/// RUN29 — Optimize MAX_SCALP_OPENS_PER_CYCLE
///
/// Hypothesis: The burst cap of 3 in COINCLAW v13/v14 is blocking profitable
/// correlated-move entries. Test caps 1..18 + unlimited on 1-year 1m data to
/// find the optimal value.
///
/// Simulation faithfully replicates COINCLAW v13 scalp logic:
///   - Signals: stoch_cross + vol_spike_rev (F6 filter gated)
///   - TP = 0.80%, SL = 0.10%, MAX_HOLD = 60 bars
///   - Scalp cooldown: 300 bars (5 min) after SL
///   - Coins iterated in COINCLAW order (index 0..17)
///   - Cap applied per 1m cycle: only first N signalling coins open
///   - Portfolio P&L, WR, trade count, avg simultaneous signals per bar
///
/// Output: run29_1_results.json

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::indicators::{rolling_max, rolling_mean, rolling_min, rsi};

// ── Constants matching COINCLAW v13 scalp config ──────────────────────────────
const TP_PCT:            f64 = 0.008;   // 0.80%
const SL_PCT:            f64 = 0.001;   // 0.10%
const FEE:               f64 = 0.001;   // 0.1% per side
const SLIP:              f64 = 0.0005;  // 0.05% per side
const MAX_HOLD:          usize = 60;    // 60 bars max hold
const SCALP_COOLDOWN:    usize = 300;   // 300 bars = 5 min after SL
const VOL_MA_PERIOD:     usize = 20;
const SCALP_VOL_MULT:    f64 = 3.5;
const SCALP_RSI_EXTREME: f64 = 20.0;
const SCALP_STOCH_EXT:   f64 = 5.0;
const F6_ROC_3:          f64 = -0.195; // % (already scaled)
const F6_BODY_3:         f64 = 0.072;  // % (already scaled)

const COINS: [&str; 18] = [
    "DASH", "UNI", "NEAR", "ADA", "LTC", "SHIB", "LINK", "ETH",
    "DOT", "XRP", "ATOM", "SOL", "DOGE", "XLM", "AVAX", "ALGO", "BNB", "BTC",
];

const CAPS: [usize; 19] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 999];

// ── Data structures ───────────────────────────────────────────────────────────
#[derive(Clone)]
struct Bar {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    v: f64,
}

#[derive(Clone)]
struct ScalpInd {
    rsi:          f64,
    vol:          f64,
    vol_ma:       f64,
    stoch_k:      f64,
    stoch_d:      f64,
    stoch_k_prev: f64,
    stoch_d_prev: f64,
    roc_3:        f64,  // % already scaled
    avg_body_3:   f64,  // % already scaled
    valid:        bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Dir { Long, Short }

#[derive(Clone)]
struct Position {
    dir:       Dir,
    entry:     f64,
    bars_held: usize,
}

#[derive(Default, Clone)]
struct CoinResult {
    trades:     usize,
    wins:       usize,
    pnl:        f64,
    sl_count:   usize,
    tp_count:   usize,
    hold_count: usize,
}

#[derive(Serialize)]
struct CapResult {
    cap:                usize,
    total_trades:       usize,
    total_wins:         usize,
    win_rate:           f64,
    total_pnl:          f64,
    avg_pnl_per_trade:  f64,
    sl_count:           usize,
    tp_count:           usize,
    hold_count:         usize,
    avg_signals_per_bar: f64,
    per_coin:           Vec<CoinCapResult>,
}

#[derive(Serialize)]
struct CoinCapResult {
    coin:      &'static str,
    trades:    usize,
    wins:      usize,
    win_rate:  f64,
    pnl:       f64,
}

#[derive(Serialize)]
struct Output {
    results: Vec<CapResult>,
    best_cap_by_pnl: usize,
    best_cap_by_wr:  usize,
    notes:           String,
}

// ── Load 1m CSV ───────────────────────────────────────────────────────────────
fn load_1m(coin: &str) -> Vec<Bar> {
    let path = format!(
        "/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_1year.csv",
        coin
    );
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => { eprintln!("  Missing {}: {}", path, e); return vec![]; }
    };
    let mut bars = Vec::with_capacity(530_000);
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next();
        let o: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let h: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let l: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let c: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let v: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        if !o.is_nan() && !h.is_nan() && !l.is_nan() && !c.is_nan() && !v.is_nan() {
            bars.push(Bar { o, h, l, c, v });
        }
    }
    bars
}

// ── Compute indicators for all bars ──────────────────────────────────────────
fn compute_indicators(bars: &[Bar]) -> Vec<ScalpInd> {
    let n = bars.len();
    let c: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let h: Vec<f64> = bars.iter().map(|b| b.h).collect();
    let l: Vec<f64> = bars.iter().map(|b| b.l).collect();
    let v: Vec<f64> = bars.iter().map(|b| b.v).collect();
    let o: Vec<f64> = bars.iter().map(|b| b.o).collect();

    let rsi14   = rsi(&c, 14);
    let vol_ma  = rolling_mean(&v, VOL_MA_PERIOD);
    let ll14    = rolling_min(&l, 14);
    let hh14    = rolling_max(&h, 14);

    // Stochastic %K
    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n {
        if !ll14[i].is_nan() && !hh14[i].is_nan() {
            let range = hh14[i] - ll14[i];
            if range > 0.0 { stoch_k[i] = 100.0 * (c[i] - ll14[i]) / range; }
        }
    }
    // NaN-safe stoch_d: only compute where all 3 stoch_k values are valid
    let mut stoch_d = vec![f64::NAN; n];
    for i in 2..n {
        if !stoch_k[i].is_nan() && !stoch_k[i-1].is_nan() && !stoch_k[i-2].is_nan() {
            stoch_d[i] = (stoch_k[i] + stoch_k[i-1] + stoch_k[i-2]) / 3.0;
        }
    }

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        // F6 features: 3-bar ROC and avg body %
        let roc_3 = if i >= 3 && c[i - 3] > 0.0 {
            (c[i] - c[i - 3]) / c[i - 3] * 100.0
        } else { 0.0 };
        let avg_body_3 = if i >= 3 {
            (0..3usize).map(|k| {
                let idx = i - k;
                if c[idx] > 0.0 { (c[idx] - o[idx]).abs() / c[idx] * 100.0 } else { 0.0 }
            }).sum::<f64>() / 3.0
        } else { 0.0 };

        let stk_prev = if i > 0 { stoch_k[i - 1] } else { f64::NAN };
        let std_prev = if i > 0 { stoch_d[i - 1] } else { f64::NAN };

        let valid = i >= 20
            && !rsi14[i].is_nan()
            && !vol_ma[i].is_nan()
            && !stoch_k[i].is_nan()
            && !stoch_d[i].is_nan()
            && vol_ma[i] > 0.0;

        out.push(ScalpInd {
            rsi:          rsi14[i],
            vol:          v[i],
            vol_ma:       vol_ma[i],
            stoch_k:      stoch_k[i],
            stoch_d:      stoch_d[i],
            stoch_k_prev: stk_prev,
            stoch_d_prev: std_prev,
            roc_3,
            avg_body_3,
            valid,
        });
    }
    out
}

// ── F6 filter ─────────────────────────────────────────────────────────────────
fn passes_f6(ind: &ScalpInd, dir: Dir) -> bool {
    match dir {
        Dir::Long  => !(ind.roc_3 <  F6_ROC_3 && ind.avg_body_3 > F6_BODY_3),
        Dir::Short => !(ind.roc_3 > -F6_ROC_3 && ind.avg_body_3 > F6_BODY_3),
    }
}

// ── Signal detection for one coin at bar i ────────────────────────────────────
fn scalp_signal(ind: &ScalpInd) -> Option<Dir> {
    if !ind.valid || ind.vol_ma == 0.0 { return None; }
    let vol_r   = ind.vol / ind.vol_ma;
    let rsi_lo  = SCALP_RSI_EXTREME;
    let rsi_hi  = 100.0 - SCALP_RSI_EXTREME;
    let st_lo   = SCALP_STOCH_EXT;
    let st_hi   = 100.0 - SCALP_STOCH_EXT;

    // 1. vol_spike_rev
    if vol_r > SCALP_VOL_MULT {
        if ind.rsi < rsi_lo && passes_f6(ind, Dir::Long)  { return Some(Dir::Long); }
        if ind.rsi > rsi_hi && passes_f6(ind, Dir::Short) { return Some(Dir::Short); }
    }
    // 2. stoch_cross
    if !ind.stoch_k_prev.is_nan() && !ind.stoch_d_prev.is_nan() {
        if ind.stoch_k_prev <= ind.stoch_d_prev && ind.stoch_k > ind.stoch_d
            && ind.stoch_k < st_lo && ind.stoch_d < st_lo
            && passes_f6(ind, Dir::Long)
        { return Some(Dir::Long); }
        if ind.stoch_k_prev >= ind.stoch_d_prev && ind.stoch_k < ind.stoch_d
            && ind.stoch_k > st_hi && ind.stoch_d > st_hi
            && passes_f6(ind, Dir::Short)
        { return Some(Dir::Short); }
    }
    None
}

// ── Simulate portfolio with a given cap ──────────────────────────────────────
fn simulate(
    all_inds: &[Vec<ScalpInd>],
    all_bars: &[Vec<Bar>],
    cap: usize,
) -> (Vec<CoinResult>, f64) {
    let n_coins = all_inds.len();
    let n_bars  = all_inds[0].len();

    let mut positions:  Vec<Option<Position>> = vec![None; n_coins];
    let mut cooldowns:  Vec<usize>            = vec![0; n_coins];
    let mut results:    Vec<CoinResult>       = vec![CoinResult::default(); n_coins];
    let mut total_signals = 0u64;
    let mut signal_bars   = 0u64;

    for bar_i in 1..n_bars {
        // ── 1. Check exits for open positions ─────────────────────────────
        for ci in 0..n_coins {
            if let Some(ref pos) = positions[ci] {
                let bar  = &all_bars[ci][bar_i];
                let entry = pos.entry;
                let (tp_price, sl_price) = match pos.dir {
                    Dir::Long  => (entry * (1.0 + TP_PCT), entry * (1.0 - SL_PCT)),
                    Dir::Short => (entry * (1.0 - TP_PCT), entry * (1.0 + SL_PCT)),
                };
                let held = pos.bars_held;

                let exit_pnl = match pos.dir {
                    Dir::Long => {
                        if bar.l <= sl_price {
                            Some((sl_price / entry - 1.0 - FEE - SLIP, false, true))
                        } else if bar.h >= tp_price {
                            Some((TP_PCT - FEE - SLIP, true, false))
                        } else if held >= MAX_HOLD {
                            let pnl = bar.c / entry - 1.0 - FEE - SLIP;
                            Some((pnl, pnl > 0.0, false))
                        } else { None }
                    }
                    Dir::Short => {
                        if bar.h >= sl_price {
                            Some((-(sl_price / entry - 1.0) - FEE - SLIP, false, true))
                        } else if bar.l <= tp_price {
                            Some((TP_PCT - FEE - SLIP, true, false))
                        } else if held >= MAX_HOLD {
                            let pnl = -(bar.c / entry - 1.0) - FEE - SLIP;
                            Some((pnl, pnl > 0.0, false))
                        } else { None }
                    }
                };

                if let Some((pnl, win, is_sl)) = exit_pnl {
                    results[ci].trades += 1;
                    results[ci].pnl    += pnl;
                    if win { results[ci].wins += 1; results[ci].tp_count += 1; }
                    else if is_sl { results[ci].sl_count += 1; cooldowns[ci] = SCALP_COOLDOWN; }
                    else { results[ci].hold_count += 1; }
                    positions[ci] = None;
                } else {
                    positions[ci].as_mut().unwrap().bars_held += 1;
                }
            }
            // Decrement cooldown
            if cooldowns[ci] > 0 { cooldowns[ci] -= 1; }
        }

        // ── 2. Collect signals for all coins without a position ───────────
        let mut signals: Vec<(usize, Dir)> = Vec::new();
        for ci in 0..n_coins {
            if positions[ci].is_none() && cooldowns[ci] == 0 {
                if let Some(dir) = scalp_signal(&all_inds[ci][bar_i]) {
                    signals.push((ci, dir));
                }
            }
        }

        if !signals.is_empty() {
            total_signals += signals.len() as u64;
            signal_bars   += 1;
        }

        // ── 3. Apply cap: open first N signals (by coin order) ────────────
        let effective_cap = cap.min(n_coins);
        for &(ci, dir) in signals.iter().take(effective_cap) {
            let entry = all_bars[ci][bar_i].o; // open of next bar as entry
            positions[ci] = Some(Position { dir, entry, bars_held: 0 });
        }
    }

    let avg_signals = if signal_bars > 0 {
        total_signals as f64 / signal_bars as f64
    } else { 0.0 };

    (results, avg_signals)
}

// ── Entry point ───────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN29 — MAX_SCALP_OPENS_PER_CYCLE optimization");
    eprintln!("Loading 1m data for {} coins...", COINS.len());

    // Load all coin data
    let all_bars: Vec<Vec<Bar>> = COINS.iter().map(|coin| {
        let bars = load_1m(coin);
        eprintln!("  {} — {} bars", coin, bars.len());
        bars
    }).collect();

    // Truncate all to the shortest (alignment)
    let min_len = all_bars.iter().map(|b| b.len()).min().unwrap_or(0);
    if min_len < 1000 {
        eprintln!("ERROR: insufficient data (min {} bars)", min_len);
        return;
    }
    let all_bars: Vec<Vec<Bar>> = all_bars.into_iter().map(|b| b[b.len()-min_len..].to_vec()).collect();
    eprintln!("Aligned to {} bars (~{:.1} days)", min_len, min_len as f64 / 1440.0);

    eprintln!("Computing indicators...");
    let all_inds: Vec<Vec<ScalpInd>> = all_bars.par_iter().map(|bars| compute_indicators(bars)).collect();

    // Checkpoint file
    let checkpoint_path = "/home/scamarena/ProjectCoin/run29_1_checkpoint.json";
    let result_path     = "/home/scamarena/ProjectCoin/run29_1_results.json";

    eprintln!("Running {} cap values...", CAPS.len());

    let mut cap_results: Vec<CapResult> = CAPS.par_iter().filter_map(|&cap| {
        if shutdown.load(Ordering::SeqCst) { return None; }

        let cap_label = if cap >= 999 { 18 } else { cap };
        eprintln!("  cap={}", cap_label);

        let (coin_results, avg_signals) = simulate(&all_inds, &all_bars, cap);

        let total_trades = coin_results.iter().map(|r| r.trades).sum::<usize>();
        let total_wins   = coin_results.iter().map(|r| r.wins).sum::<usize>();
        let total_pnl    = coin_results.iter().map(|r| r.pnl).sum::<f64>();
        let sl_count     = coin_results.iter().map(|r| r.sl_count).sum::<usize>();
        let tp_count     = coin_results.iter().map(|r| r.tp_count).sum::<usize>();
        let hold_count   = coin_results.iter().map(|r| r.hold_count).sum::<usize>();
        let win_rate     = if total_trades > 0 { total_wins as f64 / total_trades as f64 } else { 0.0 };
        let avg_pnl      = if total_trades > 0 { total_pnl / total_trades as f64 } else { 0.0 };

        let per_coin = COINS.iter().zip(coin_results.iter()).map(|(name, r)| CoinCapResult {
            coin:     name,
            trades:   r.trades,
            wins:     r.wins,
            win_rate: if r.trades > 0 { r.wins as f64 / r.trades as f64 } else { 0.0 },
            pnl:      r.pnl,
        }).collect();

        Some(CapResult {
            cap: cap_label,
            total_trades,
            total_wins,
            win_rate,
            total_pnl,
            avg_pnl_per_trade: avg_pnl,
            sl_count,
            tp_count,
            hold_count,
            avg_signals_per_bar: avg_signals,
            per_coin,
        })
    }).collect();

    cap_results.sort_by_key(|r| r.cap);

    if cap_results.is_empty() {
        eprintln!("No results (interrupted?)");
        return;
    }

    let best_pnl = cap_results.iter().max_by(|a, b| a.total_pnl.partial_cmp(&b.total_pnl).unwrap()).map(|r| r.cap).unwrap_or(3);
    let best_wr  = cap_results.iter().max_by(|a, b| a.win_rate.partial_cmp(&b.win_rate).unwrap()).map(|r| r.cap).unwrap_or(3);

    // Print summary table
    eprintln!();
    eprintln!("{:<6} {:>8} {:>6} {:>8} {:>10} {:>10}", "cap", "trades", "WR%", "total_pnl", "avg_pnl", "avg_sigs");
    for r in &cap_results {
        eprintln!("{:<6} {:>8} {:>5.1}% {:>+8.4} {:>+10.6} {:>10.2}",
            r.cap, r.total_trades, r.win_rate * 100.0, r.total_pnl, r.avg_pnl_per_trade, r.avg_signals_per_bar);
    }
    eprintln!();
    eprintln!("Best by total PnL: cap={}", best_pnl);
    eprintln!("Best by win rate:  cap={}", best_wr);

    let output = Output {
        results: cap_results,
        best_cap_by_pnl: best_pnl,
        best_cap_by_wr:  best_wr,
        notes: format!(
            "1m data, {} coins, {:.0} bars. TP={:.1}% SL={:.1}% MaxHold={}bars Fee={:.2}% Slip={:.2}%",
            COINS.len(), min_len as f64,
            TP_PCT*100.0, SL_PCT*100.0, MAX_HOLD, FEE*100.0, SLIP*100.0
        ),
    };

    let json = serde_json::to_string_pretty(&output).unwrap();
    std::fs::write(result_path, &json).ok();
    std::fs::remove_file(checkpoint_path).ok();
    eprintln!("Saved → {}", result_path);
}
