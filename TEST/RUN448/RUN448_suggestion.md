# RUN448 — Donchian Channel with RSI Pullback Confirmation

## Hypothesis

**Mechanism**: Donchian Channel breakout identifies when price breaks above the highest high or below the lowest low over a period — a classic trend-following signal. However, breakouts often fail. RSI Pullback adds timing precision: after a Donchian breakout, if price pulls back to the broken level AND RSI is pulling back from oversold/overbought, the breakout is more likely to continue. This catches breakout confirmation on pullback.

**Why not duplicate**: RUN359 uses Donchian with Volume Surge. RUN394 uses Donchian with Choppiness Index. This RUN specifically uses RSI Pullback (price returning to broken level with RSI confirmation) — the distinct mechanism is using pullback confirmation rather than volume or choppiness.

## Proposed Config Changes (config.rs)

```rust
// ── RUN448: Donchian Channel with RSI Pullback Confirmation ─────────────────────────────────────
// donchian_upper = highest_high over period
// donchian_lower = lowest_low over period
// donchian_break: price crosses above upper or below lower
// pullback: price returns to broken level after initial break
// rsi_pullback: rsi is between 40-60 (recovering from extreme)
// LONG: price breaks donchian_upper AND pulls back to broken level AND rsi recovering
// SHORT: price breaks donchian_lower AND pulls back to broken level AND rsi recovering

pub const DC_RSI_PB_ENABLED: bool = true;
pub const DC_RSI_PB_DC_PERIOD: usize = 20;
pub const DC_RSI_PB_RSI_PERIOD: usize = 14;
pub const DC_RSI_PB_RSI_LOWER: f64 = 40.0;
pub const DC_RSI_PB_RSI_UPPER: f64 = 60.0;
pub const DC_RSI_PB_SL: f64 = 0.005;
pub const DC_RSI_PB_TP: f64 = 0.004;
pub const DC_RSI_PB_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run448_1_dc_rsi_pb_backtest.py)
2. **Walk-forward** (run448_2_dc_rsi_pb_wf.py)
3. **Combined** (run448_3_combined.py)

## Out-of-Sample Testing

- DC_PERIOD sweep: 15 / 20 / 30
- RSI_PERIOD sweep: 10 / 14 / 21
- RSI_LOWER sweep: 35 / 40 / 45
- RSI_UPPER sweep: 55 / 60 / 65
