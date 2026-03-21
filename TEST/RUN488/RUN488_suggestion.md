# RUN488 — Donchian Channel with Volume Profile POC

## Hypothesis

**Mechanism**: Donchian Channel breakout identifies when price exceeds the highest high or lowest low over a period, signaling potential trend continuation. Volume Profile POC (Point of Control) identifies the price level with the highest traded volume over a lookback period. When Donchian breakout occurs AND price is moving away from POC in the direction of the breakout, the move has both trend conviction and volume profile structural support.

**Why not duplicate**: RUN448 uses Donchian Channel with RSI Pullback Confirmation. This RUN uses Volume Profile POC instead — distinct mechanism is volume-based structural confirmation versus RSI oscillator timing. POC identifies where the most trading activity occurred, adding a structural dimension.

## Proposed Config Changes (config.rs)

```rust
// ── RUN488: Donchian Channel with Volume Profile POC ─────────────────────────────────
// donchian_channel: highest_high and lowest_low over period
// donchian_breakout: price crosses above/below channel boundary
// vol_profile_poc: price level with highest volume traded
// poc_distance: price distance from poc in direction of breakout
// LONG: price crosses above donchian_upper AND price > poc AND poc_distance expanding
// SHORT: price crosses below donchian_lower AND price < poc AND poc_distance expanding

pub const DC_POC_ENABLED: bool = true;
pub const DC_POC_DONCHIAN_PERIOD: usize = 20;
pub const DC_POC_POC_PERIOD: usize = 20;
pub const DC_POC_SL: f64 = 0.005;
pub const DC_POC_TP: f64 = 0.004;
pub const DC_POC_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run488_1_dc_poc_backtest.py)
2. **Walk-forward** (run488_2_dc_poc_wf.py)
3. **Combined** (run488_3_combined.py)

## Out-of-Sample Testing

- DONCHIAN_PERIOD sweep: 15 / 20 / 25 / 30
- POC_PERIOD sweep: 14 / 20 / 30
