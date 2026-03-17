use crate::indicators::*;
use crate::loader::Bar;

/// Generate (entry, exit) boolean signal vectors for a named strategy.
/// Entry/exit are aligned with `bars` index.
pub fn signals(bars: &[Bar], strategy: &str) -> (Vec<bool>, Vec<bool>) {
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
    let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();
    let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();

    match strategy {
        "vwap_rev"  => vwap_reversion(&close, &high, &low, &vol, n),
        "bb_bounce" => bb_bounce(&close, &high, &low, n),
        "adr_rev"   => adr_reversal(&close, &high, &low, n),
        "dual_rsi"  => dual_rsi(&close, n),
        "mean_rev"  => mean_reversion(&close, n),
        _ => panic!("Unknown strategy: {}", strategy),
    }
}

// ── VwapReversion ─────────────────────────────────────────────────────────
// Entry: close < vwap(20) AND rsi14 < 50 AND z20 < -0.5
// Exit:  close > vwap(20) OR z20 > 0

fn vwap_reversion(
    close: &[f64], high: &[f64], low: &[f64], vol: &[f64], n: usize,
) -> (Vec<bool>, Vec<bool>) {
    let vwap = vwap_rolling(high, low, close, vol, 20);
    let rsi14 = rsi(close, 14);
    let z20   = z_score(close, 20);

    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 20..n {
        if vwap[i].is_nan() || rsi14[i].is_nan() || z20[i].is_nan() { continue; }
        entry[i] = close[i] < vwap[i] && rsi14[i] < 50.0 && z20[i] < -0.5;
        exit[i]  = close[i] > vwap[i] || z20[i] > 0.0;
    }
    (entry, exit)
}

// ── BbBounce ───────────────────────────────────────────────────────────────
// Entry: close ≤ bb_lower AND rsi14 < 40
// Exit:  close ≥ bb_mid  OR rsi14 > 60

fn bb_bounce(close: &[f64], _high: &[f64], _low: &[f64], n: usize) -> (Vec<bool>, Vec<bool>) {
    let (_, bb_mid, bb_lower) = bollinger(close, 20, 2.0);
    let rsi14 = rsi(close, 14);

    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 20..n {
        if bb_lower[i].is_nan() || rsi14[i].is_nan() { continue; }
        entry[i] = close[i] <= bb_lower[i] && rsi14[i] < 40.0;
        exit[i]  = close[i] >= bb_mid[i] || rsi14[i] > 60.0;
    }
    (entry, exit)
}

// ── AdrReversal ────────────────────────────────────────────────────────────
// Uses 96-bar rolling low as daily-low proxy (96 × 15m = 1 trading day)
// Entry: close ≤ rolling_low96 * 1.005 AND z20 < -0.5
// Exit:  close ≥ sma20 OR z20 > 0

fn adr_reversal(close: &[f64], _high: &[f64], low: &[f64], n: usize) -> (Vec<bool>, Vec<bool>) {
    let adr_low = rolling_min(low, 96);
    let sma20   = sma(close, 20);
    let z20     = z_score(close, 20);

    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 96..n {
        if adr_low[i].is_nan() || sma20[i].is_nan() || z20[i].is_nan() { continue; }
        entry[i] = close[i] <= adr_low[i] * 1.005 && z20[i] < -0.5;
        exit[i]  = close[i] >= sma20[i] || z20[i] > 0.0;
    }
    (entry, exit)
}

// ── DualRsi ────────────────────────────────────────────────────────────────
// Entry: rsi14 < 35 AND rsi7 < 30
// Exit:  rsi14 > 55 OR rsi7 > 60

fn dual_rsi(close: &[f64], n: usize) -> (Vec<bool>, Vec<bool>) {
    let rsi14 = rsi(close, 14);
    let rsi7  = rsi(close, 7);

    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 15..n {
        if rsi14[i].is_nan() || rsi7[i].is_nan() { continue; }
        entry[i] = rsi14[i] < 35.0 && rsi7[i] < 30.0;
        exit[i]  = rsi14[i] > 55.0 || rsi7[i] > 60.0;
    }
    (entry, exit)
}

// ── MeanReversion ─────────────────────────────────────────────────────────
// (Approximation of OuMeanRev used for DASH)
// Entry: z20 < -1.5 AND rsi14 < 45
// Exit:  z20 > 0

fn mean_reversion(close: &[f64], n: usize) -> (Vec<bool>, Vec<bool>) {
    let z20   = z_score(close, 20);
    let rsi14 = rsi(close, 14);

    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 20..n {
        if z20[i].is_nan() || rsi14[i].is_nan() { continue; }
        entry[i] = z20[i] < -1.5 && rsi14[i] < 45.0;
        exit[i]  = z20[i] > 0.0;
    }
    (entry, exit)
}
