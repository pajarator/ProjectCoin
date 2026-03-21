# RUN192 — Parabolic SAR Trailing Stop: Dynamic Stop-and-Reverse for Trend Persistence

## Hypothesis

**Mechanism**: Parabolic SAR (Stop and Reverse) = price-based trailing stop that accelerates over time. In an uptrend, SAR sits below price; as price rises, SAR steps up faster (AF increment). When price closes below SAR → exit LONG, enter SHORT. The SAR acts as both entry trigger and trailing stop simultaneously. AF controls acceleration sensitivity.

**Why not duplicate**: No prior RUN uses Parabolic SAR. All prior stop RUNs use fixed % stops or ATR stops. Parabolic SAR is a purpose-built trend-following stop with built-in entry/exit logic.

## Proposed Config Changes (config.rs)

```rust
// ── RUN192: Parabolic SAR Trailing Stop ─────────────────────────────────
// SAR = prior_SAR + AF × (EP - prior_SAR)
// AF = acceleration factor (0.01 to 0.20)
// EP = extreme point (highest high in uptrend, lowest low in downtrend)
// AF starts at 0.02, increments by 0.02 each time EP updates, caps at 0.20
// LONG: price crosses above SAR → entry
// SHORT: price crosses below SAR → entry
// SAR also serves as trailing stop

pub const PSAR_ENABLED: bool = true;
pub const PSAR_AF_START: f64 = 0.02;     // initial acceleration factor
pub const PSAR_AF_INCREMENT: f64 = 0.02; // AF increment each step
pub const PSAR_AF_MAX: f64 = 0.20;        // maximum AF cap
pub const PSAR_SL: f64 = 0.005;          // initial SL (tight protection)
pub const PSAR_TP: f64 = 0.004;          // take profit
pub const PSAR_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn psar(highs: &[f64], lows: &[f64], af_start: f64, af_inc: f64, af_max: f64) -> (f64, bool) {
    // Simplified single-bar PSAR approximation
    // Full PSAR requires iterative state across bars
    let len = highs.len();
    if len < 2 {
        return (lows[0], true);
    }

    let close = lows[len - 1]; // use low as proxy for current SAR position
    let sar = close; // placeholder - real implementation needs bar-by-bar state

    // Trend: true = bullish, false = bearish
    let bullish = highs[len-1] > lows[len-2];
    (sar, bullish)
}
```

---

## Validation Method

1. **Historical backtest** (run192_1_psar_backtest.py)
2. **Walk-forward** (run192_2_psar_wf.py)
3. **Combined** (run192_3_combined.py)

## Out-of-Sample Testing

- AF_START sweep: 0.01 / 0.02 / 0.03
- AF_INCREMENT sweep: 0.01 / 0.02 / 0.03
- AF_MAX sweep: 0.15 / 0.20 / 0.25
