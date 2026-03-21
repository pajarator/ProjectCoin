# RUN233 — Volume-Weighted Bollinger Bandwidth Squeeze: Low-Vol Expansion Signal

## Hypothesis

**Mechanism**: Bollinger Bandwidth = (upper_band - lower_band) / middle_band. When bandwidth drops below its 20-period MA (squeeze), volatility is abnormally low. Low volatility → future expansion. Combine with volume: if volume is also below its 20-period MA during the squeeze → volume confirms the compression. When price breaks out of the squeeze AND volume spikes above its MA → high-probability directional move.

**Why not duplicate**: No prior RUN combines Bollinger Bandwidth squeeze with volume confirmation. RUN182 uses EMA ribbon squeeze but not volume-confirmed. Volume confirmation fundamentally changes the signal by requiring the compression to be confirmed by low volume as well.

## Proposed Config Changes (config.rs)

```rust
// ── RUN233: Volume-Weighted BB Squeeze ───────────────────────────────────
// bb_width = (upper - lower) / middle
// bb_width_ma = SMA(bb_width, 20)
// vol_ma = SMA(volume, 20)
// squeeze = bb_width < bb_width_ma × 0.8 AND volume < vol_ma × 0.8
// Breakout: price closes outside upper/lower band AND volume > vol_ma
// LONG: breakout above upper band during squeeze with volume
// SHORT: breakout below lower band during squeeze with volume

pub const BB_SQUEEZE_ENABLED: bool = true;
pub const BB_SQUEEZE_PERIOD: usize = 20;     // BB period
pub const BB_SQUEEZE_STD: f64 = 2.0;         // BB std dev multiplier
pub const BB_SQUEEZE_WIDTH_MA_PERIOD: usize = 20;
pub const BB_SQUEEZE_WIDTH_THRESH: f64 = 0.8; // squeeze threshold
pub const BB_SQUEEZE_VOL_THRESH: f64 = 0.8;  // volume must be < × this
pub const BB_SQUEEZE_SL: f64 = 0.005;
pub const BB_SQUEEZE_TP: f64 = 0.004;
pub const BB_SQUEEZE_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run233_1_bb_sqz_backtest.py)
2. **Walk-forward** (run233_2_bb_sqz_wf.py)
3. **Combined** (run233_3_combined.py)

## Out-of-Sample Testing

- BB_PERIOD sweep: 15 / 20 / 30
- BB_STD sweep: 1.5 / 2.0 / 2.5
- WIDTH_THRESH sweep: 0.7 / 0.8 / 0.9
- VOL_THRESH sweep: 0.7 / 0.8 / 0.9
