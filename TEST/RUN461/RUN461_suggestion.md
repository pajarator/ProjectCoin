# RUN461 — Chaikin Money Flow with Donchian Channel Breakout

## Hypothesis

**Mechanism**: Chaikin Money Flow (CMF) measures the amount of money flow volume over a period, combining price and volume to show buying/selling pressure. CMF > 0 indicates accumulation, CMF < 0 indicates distribution. Donchian Channel Breakout provides clear trend direction via price exceeding the N-period high/low. The combination ensures breakouts have underlying money flow conviction: only take Donchian breakouts when CMF is also confirming the directional flow.

**Why not duplicate**: RUN389 uses A/D Line with SuperTrend. This RUN uses CMF with Donchian instead — distinct because CMF is a different money flow formula than A/D Line, and Donchian is a pure price breakout mechanism rather than SuperTrend's ATR-based trailing stop.

## Proposed Config Changes (config.rs)

```rust
// ── RUN461: Chaikin Money Flow with Donchian Channel Breakout ─────────────────────────────────
// cmf: chaikin_money_flow(close, high, low, volume, period)
// cmf_cross: cmf crosses above/below 0
// donchian_breakout: price exceeds donchian_upper or falls below donchian_lower
// LONG: donchian_breakout bullish AND cmf > 0
// SHORT: donchian_breakout bearish AND cmf < 0

pub const CMF_DC_ENABLED: bool = true;
pub const CMF_DC_CMF_PERIOD: usize = 20;
pub const CMF_DC_CMF_THRESH: f64 = 0.0;
pub const CMF_DC_DONCHIAN_PERIOD: usize = 20;
pub const CMF_DC_SL: f64 = 0.005;
pub const CMF_DC_TP: f64 = 0.004;
pub const CMF_DC_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run461_1_cmf_dc_backtest.py)
2. **Walk-forward** (run461_2_cmf_dc_wf.py)
3. **Combined** (run461_3_combined.py)

## Out-of-Sample Testing

- CMF_PERIOD sweep: 14 / 20 / 30
- DONCHIAN_PERIOD sweep: 15 / 20 / 25 / 30
- CMF_THRESH sweep: -0.05 / 0.0 / 0.05
