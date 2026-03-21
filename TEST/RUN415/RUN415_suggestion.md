# RUN415 — Pivot Point Zone Detection with RSI Extreme Filter

## Hypothesis

**Mechanism**: Pivot Points identify key support and resistance levels based on prior period's high, low, and close. The "zone" between the pivot point and the first support/resistance creates a conflict zone where price tends to hesitate. RSI Extreme Filter adds timing precision: when price is bouncing in a pivot zone AND RSI reaches extreme (oversold <30 or overbought >70), the bounce has structural support from the pivot zone AND momentum confirmation from RSI extremes.

**Why not duplicate**: RUN305 uses Pivot Point Mean Reversion. This RUN specifically uses Pivot Point zones (not just single levels) with RSI extremes — the distinct mechanism is using the zone concept (range between pivot and S1/R1) as a structural filter combined with RSI extreme timing.

## Proposed Config Changes (config.rs)

```rust
// ── RUN415: Pivot Point Zone Detection with RSI Extreme Filter ───────────────────────────────
// pivot_point = (prev_high + prev_low + prev_close) / 3
// pivot_zone = area between PP and first S/R levels
// price_in_zone: price bouncing within pivot zone
// rsi_extreme: rsi < RSI_OVERSOLD or rsi > RSI_OVERBOUGHT
// LONG: price bouncing in pivot_zone AND rsi < RSI_OVERSOLD
// SHORT: price bouncing in pivot_zone AND rsi > RSI_OVERBOUGHT

pub const PP_RSI_ENABLED: bool = true;
pub const PP_RSI_PERIOD: usize = 20;     // lookback for zone detection
pub const PP_RSI_RSI_PERIOD: usize = 14;
pub const PP_RSI_RSI_OVERSOLD: f64 = 30.0;
pub const PP_RSI_RSI_OVERBOUGHT: f64 = 70.0;
pub const PP_RSI_ZONE_WIDTH: f64 = 0.005; // zone width as % of price
pub const PP_RSI_SL: f64 = 0.005;
pub const PP_RSI_TP: f64 = 0.004;
pub const PP_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run415_1_pp_rsi_backtest.py)
2. **Walk-forward** (run415_2_pp_rsi_wf.py)
3. **Combined** (run415_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- RSI_OVERSOLD sweep: 25 / 30 / 35
- RSI_OVERBOUGHT sweep: 65 / 70 / 75
- ZONE_WIDTH sweep: 0.003 / 0.005 / 0.007
