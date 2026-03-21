# RUN257 — Session High-Low Rejection: Daily Extreme Wick Reversal

## Hypothesis

**Mechanism**: When price repeatedly touches the daily high (or low) and gets rejected → strong rejection pattern. The wick (shadow) extending beyond the rejection shows where the "smart money" rejected the price. A long lower wick at daily low → bullish reversal. A long upper wick at daily high → bearish reversal. Wick must be >50% of candle body.

**Why not duplicate**: No prior RUN uses session high/low rejection with wick analysis. All prior candlestick RUNs use doji, engulfing, or inside/outside bars. Wick rejection at daily extremes is a distinct price action pattern.

## Proposed Config Changes (config.rs)

```rust
// ── RUN257: Session High-Low Rejection ───────────────────────────────────
// lower_wick = low - min(open, close)  (below-body wick)
// upper_wick = max(open, close) - high  (above-body wick)
// body = |close - open|
// LONG rejection: lower_wick > body × 1.5 AND close near daily low
// SHORT rejection: upper_wick > body × 1.5 AND close near daily high

pub const SESSION_REJ_ENABLED: bool = true;
pub const SESSION_REJ_WICK_MULT: f64 = 1.5;  // wick must be > body × this
pub const SESSION_REJ_BODY_MIN: f64 = 0.001;  // minimum body size for valid candle
pub const SESSION_REJ_SL: f64 = 0.005;
pub const SESSION_REJ_TP: f64 = 0.004;
pub const SESSION_REJ_MAX_HOLD: u32 = 24;
```

Add in `indicators.rs`:

```rust
pub fn session_rejection(opens: &[f64], highs: &[f64], lows: &[f64], closes: &[f64]) -> (bool, bool) {
    let n = closes.len();
    if n == 0 {
        return (false, false);
    }

    let open = opens[n-1];
    let high = highs[n-1];
    let low = lows[n-1];
    let close = closes[n-1];

    let body = (close - open).abs();
    let lower_wick = low - open.min(close);
    let upper_wick = open.max(close) - high;

    let lower_rejection = lower_wick > body * 1.5 && body > 0.001;
    let upper_rejection = upper_wick > body * 1.5 && body > 0.001;

    (lower_rejection, upper_rejection)
}
```

---

## Validation Method

1. **Historical backtest** (run257_1_session_rej_backtest.py)
2. **Walk-forward** (run257_2_session_rej_wf.py)
3. **Combined** (run257_3_combined.py)

## Out-of-Sample Testing

- WICK_MULT sweep: 1.0 / 1.5 / 2.0
- BODY_MIN sweep: 0.0005 / 0.001 / 0.002
