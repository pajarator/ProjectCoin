# RUN332 — Volume-Weighted Bollinger Band Touch with Trend Filter

## Hypothesis

**Mechanism**: Standard Bollinger Bands don't account for volume. This RUN weights each bar's position within the BB by its volume — high-volume touches of the band are more significant than low-volume touches. Additionally, filter by trend direction (price above/below SMA) to only trade touches in the direction of the trend (trend continuation) or against the trend when at extreme (reversal).

**Why not duplicate**: RUN233 uses BB squeeze with volume. RUN241 uses RSI Bollinger Band squeeze. RUN248 uses BB momentum score. This RUN is distinct: volume-weighted BB positioning with directional trend filter — the volume weighting is applied to the position within the band, not just the touch.

## Proposed Config Changes (config.rs)

```rust
// ── RUN332: Volume-Weighted Bollinger Band Touch with Trend Filter ────────────────
// bb_pos_weighted = sum((bb_position * volume)) / sum(volume)
// bb_position = (close - bb_lower) / (bb_upper - bb_lower)  [0-1 scale]
// trend_up = close > SMA(period)
// trend_down = close < SMA(period)
// LONG: bb_pos_weighted < LOWER_THRESH AND trend_up (buy the dip in uptrend)
// SHORT: bb_pos_weighted > UPPER_THRESH AND trend_down (sell the rally in downtrend)

pub const VWBB_ENABLED: bool = true;
pub const VWBB_PERIOD: usize = 20;
pub const VWBB_STD: f64 = 2.0;
pub const VWBB_LOWER_THRESH: f64 = 0.1;    // near lower band
pub const VWBB_UPPER_THRESH: f64 = 0.9;    // near upper band
pub const VWBB_TREND_PERIOD: usize = 20;
pub const VWBB_SL: f64 = 0.005;
pub const VWBB_TP: f64 = 0.004;
pub const VWBB_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run332_1_vwbb_backtest.py)
2. **Walk-forward** (run332_2_vwbb_wf.py)
3. **Combined** (run332_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 15 / 20 / 30
- STD sweep: 1.5 / 2.0 / 2.5
- LOWER_THRESH sweep: 0.05 / 0.1 / 0.15
- UPPER_THRESH sweep: 0.85 / 0.9 / 0.95
