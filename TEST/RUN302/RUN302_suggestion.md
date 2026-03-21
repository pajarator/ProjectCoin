# RUN302 — Rainbow Chart EMA Oscillator: Multi-Layer Mean Reversion

## Hypothesis

**Mechanism**: The Rainbow Chart plots multiple EMAs (5, 8, 13, 21, 34, 55, 89) creating stacked bands. When price pierces through the fastest EMA (5) after being below it → potential bounce setup. The thicker the rainbow (more separation between EMAs), the stronger the trend. Mean-reversion works when price overshoots through multiple EMA layers — it must traverse the rainbow to reach the other side, creating a self-correcting pullback.

**Why not duplicate**: RUN235 uses Rainbow Chart but as a trend identification tool. This RUN uses the EMA layers as a mean-reversion bounce system — price bouncing off the 5 EMA after being below it, targeting the 13 or 21 EMA. Distinct because it's a counter-trend play within a trending rainbow, not a trend-following rainbow break.

## Proposed Config Changes (config.rs)

```rust
// ── RUN302: Rainbow EMA Oscillator ──────────────────────────────────────────
// ema_layers = [5, 8, 13, 21, 34, 55, 89]
// rainbow_width = EMA(55) - EMA(5)  — measure of trend strength
// LONG: price crosses above EMA(5) AND prior close < EMA(5) AND rainbow_width < threshold
// SHORT: price crosses below EMA(5) AND prior close > EMA(5) AND rainbow_width < threshold
// Exit: price crosses back through EMA(5)
// Target: EMA(13) for tight trades, EMA(21) for wider trades

pub const RAINBOW_ENABLED: bool = true;
pub const RAINBOW_FAST: usize = 5;
pub const RAINBOW_TARGET1: usize = 13;
pub const RAINBOW_TARGET2: usize = 21;
pub const RAINBOW_WIDTH_THRESH: f64 = 0.02;   // max rainbow width for valid signal
pub const RAINBOW_SL: f64 = 0.005;
pub const RAINBOW_TP: f64 = 0.004;
pub const RAINBOW_MAX_HOLD: u32 = 24;
```

---

## Validation Method

1. **Historical backtest** (run302_1_rainbow_backtest.py)
2. **Walk-forward** (run302_2_rainbow_wf.py)
3. **Combined** (run302_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 3 / 5 / 8
- TARGET1 sweep: 8 / 13 / 21
- WIDTH_THRESH sweep: 0.01 / 0.02 / 0.03
