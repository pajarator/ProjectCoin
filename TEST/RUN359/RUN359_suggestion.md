# RUN359 — Donchian Channel with Volume Surge Breakout

## Hypothesis

**Mechanism**: Donchian Channel uses the highest high and lowest low over a lookback period. Breakout above the upper band = price is making new highs. Breakout below the lower band = new lows. Volume confirmation is essential: a breakout without volume surge is prone to failure. The breakout must coincide with volume > N× average volume to confirm the move has institutional support.

**Why not duplicate**: RUN189 uses Donchian Channel Breakout. This RUN specifically adds volume surge confirmation — the volume filter is the distinct mechanism. Donchian breakouts without volume confirmation often fail.

## Proposed Config Changes (config.rs)

```rust
// ── RUN359: Donchian Channel with Volume Surge Breakout ────────────────────────────────
// donchian_upper = highest(high, period)
// donchian_lower = lowest(low, period)
// donchian_mid = (donchian_upper + donchian_lower) / 2
// breakout_up = close crosses above donchian_upper AND volume > avg_vol * VOL_MULT
// breakout_down = close crosses below donchian_lower AND volume > avg_vol * VOL_MULT

pub const DONCHIAN_VOL_ENABLED: bool = true;
pub const DONCHIAN_VOL_PERIOD: usize = 20;
pub const DONCHIAN_VOL_VOL_MULT: f64 = 2.0;  // volume must exceed 2x avg
pub const DONCHIAN_VOL_SL: f64 = 0.005;
pub const DONCHIAN_VOL_TP: f64 = 0.004;
pub const DONCHIAN_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run359_1_donchian_vol_backtest.py)
2. **Walk-forward** (run359_2_donchian_vol_wf.py)
3. **Combined** (run359_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- VOL_MULT sweep: 1.5 / 2.0 / 2.5
