# RUN288 — Volume Spike Reversal: High-Volume Reversal Detection

## Hypothesis

**Mechanism**: When volume spikes to >3× the 20-period average AND price closes in the opposite direction of the prior move → reversal. High volume + price rejection = smart money distribution/accumulation. The large volume confirms institutional involvement.

**Why not duplicate**: No prior RUN uses volume spike + reversal direction. All prior volume spikes use direction but not reversal confirmation. Volume spike reversal is distinct because it requires BOTH high volume AND price rejection.

## Proposed Config Changes (config.rs)

```rust
// ── RUN288: Volume Spike Reversal ────────────────────────────────────────
// vol_spike = volume > vol_ma × 3
// prior_move_up = close[1] > close[2]
// current_close_down = close < open (bearish rejection)
// LONG: vol_spike AND prior_move_down AND current_bullish_rejection
// SHORT: vol_spike AND prior_move_up AND current_bearish_rejection

pub const VOL_SPIKE_REV_ENABLED: bool = true;
pub const VOL_SPIKE_REV_VOL_MA: usize = 20;
pub const VOL_SPIKE_REV_MULT: f64 = 3.0;
pub const VOL_SPIKE_REV_SL: f64 = 0.005;
pub const VOL_SPIKE_REV_TP: f64 = 0.004;
pub const VOL_SPIKE_REV_MAX_HOLD: u32 = 24;
```

---

## Validation Method

1. **Historical backtest** (run288_1_vol_spike_rev_backtest.py)
2. **Walk-forward** (run288_2_vol_spike_rev_wf.py)
3. **Combined** (run288_3_combined.py)

## Out-of-Sample Testing

- VOL_MA sweep: 14 / 20 / 30
- MULT sweep: 2.0 / 3.0 / 4.0
