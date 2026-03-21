# RUN434 — Price Momentum Rotation with Volume Divergence

## Hypothesis

**Mechanism**: Price Momentum Rotation tracks which coins in the universe are leading vs lagging in momentum. When a coin's momentum rank rises (it becomes a leader), institutional interest is shifting toward it. Volume Divergence confirms the rotation has volume backing: price is making momentum highs but volume isn't confirming. When momentum rotation is rising AND volume divergence is present, the rotation signal has cross-coin conviction.

**Why not duplicate**: RUN370 uses Price Momentum Rotation with Breadth. RUN365 uses Volume-Price Correlation Divergence. This RUN specifically combines Price Momentum Rotation (cross-coin leadership ranking) with Volume Divergence — the distinct mechanism is using cross-coin momentum rank changes to identify sector/style rotation with volume divergence confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN434: Price Momentum Rotation with Volume Divergence ─────────────────────────────────────
// momentum_rotation = rank of coin's ROC within the 18-coin universe
// rotation_rising: momentum rank improving (coin becoming a leader)
// volume_divergence: price makes new high but volume doesn't confirm
// LONG: rotation_rising AND vol_divergence present (rotation with bearish div)
// SHORT: rotation_falling AND vol_divergence present (de-rotation with bearish div)

pub const MOM_ROT_VOL_DIV_ENABLED: bool = true;
pub const MOM_ROT_VOL_DIV_MOM_PERIOD: usize = 14;
pub const MOM_ROT_VOL_DIV_ROT_PERIOD: usize = 20;
pub const MOM_ROT_VOL_DIV_VOL_PERIOD: usize = 20;
pub const MOM_ROT_VOL_DIV_SL: f64 = 0.005;
pub const MOM_ROT_VOL_DIV_TP: f64 = 0.004;
pub const MOM_ROT_VOL_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run434_1_mom_rot_vol_div_backtest.py)
2. **Walk-forward** (run434_2_mom_rot_vol_div_wf.py)
3. **Combined** (run434_3_combined.py)

## Out-of-Sample Testing

- MOM_PERIOD sweep: 10 / 14 / 21
- ROT_PERIOD sweep: 14 / 20 / 30
- VOL_PERIOD sweep: 14 / 20 / 30
