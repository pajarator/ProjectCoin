# RUN478 — Donchian Channel with ADX Disposition Filter

## Hypothesis

**Mechanism**: Donchian Channel breakout identifies when price exceeds the highest high or lowest low over a period, signaling trend continuation. ADX Disposition Filter ensures the breakout occurs in a trending environment: when ADX is rising and above a threshold, the market has directional conviction. This prevents false breakouts in choppy, range-bound conditions where Donchian breakouts frequently fail.

**Why not duplicate**: RUN394 uses Donchian Channel Breakout with Choppiness Index Trend Filter. This RUN uses ADX Disposition instead — distinct mechanism is ADX's directional strength measurement versus Choppiness Index's choppy/trending classification. ADX specifically measures trend quality, not just choppiness.

## Proposed Config Changes (config.rs)

```rust
// ── RUN478: Donchian Channel with ADX Disposition Filter ─────────────────────────────────
// donchian_channel: highest_high and lowest_low over period
// donchian_breakout: price crosses above/below channel boundary
// adx_disposition: adx rising AND above threshold = strong trend
// LONG: price crosses above donchian_upper AND adx_rising AND adx > 20
// SHORT: price crosses below donchian_lower AND adx_rising AND adx > 20

pub const DC_ADX_ENABLED: bool = true;
pub const DC_ADX_DONCHIAN_PERIOD: usize = 20;
pub const DC_ADX_ADX_PERIOD: usize = 14;
pub const DC_ADX_ADX_THRESH: f64 = 20.0;
pub const DC_ADX_ADX_CHANGE_THRESH: f64 = 1.5;
pub const DC_ADX_SL: f64 = 0.005;
pub const DC_ADX_TP: f64 = 0.004;
pub const DC_ADX_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run478_1_dc_adx_backtest.py)
2. **Walk-forward** (run478_2_dc_adx_wf.py)
3. **Combined** (run478_3_combined.py)

## Out-of-Sample Testing

- DONCHIAN_PERIOD sweep: 15 / 20 / 25 / 30
- ADX_PERIOD sweep: 10 / 14 / 20
- ADX_THRESH sweep: 15 / 20 / 25
- ADX_CHANGE_THRESH sweep: 1.0 / 1.5 / 2.0
