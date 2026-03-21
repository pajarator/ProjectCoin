# RUN400 — Inverted EMA Cross with Volume Spike Confirmation

## Hypothesis

**Mechanism**: An Inverted EMA (I-EMA) is a moving average that bends more sharply during low-volatility periods and flattens during high-volatility periods, the opposite of traditional smoothing. When price crosses the I-EMA after being far away, it often signals a snap-back move. Combine with volume spike confirmation: when price crosses I-EMA AND volume spikes above its recent average, the move has both mean-reversion timing and institutional participation confirmation. Volume spike confirms the move isn't a false spike.

**Why not duplicate**: RUN336 uses Ease of Movement with EMA Slope. RUN374 uses TEMA Crossover with Volume. This RUN specifically uses Inverted EMA (a volatility-adaptive smoothing technique distinct from standard EMA or TEMA) with Volume spike confirmation — the distinct mechanism is I-EMA's unique response to volatility conditions combined with volume spike for trade confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN400: Inverted EMA Cross with Volume Spike Confirmation ──────────────────────────────
// i_ema = inverted_ema(close, period) - adapts opposite to traditional EMA
// i_ema_cross: price crosses above/below i_ema after extended distance
// volume_spike: volume > VOL_SPIKE_MULT * SMA(volume, period)
// LONG: price crosses above i_ema AND volume > vol_spike threshold
// SHORT: price crosses below i_ema AND volume > vol_spike threshold

pub const IEMA_VOL_ENABLED: bool = true;
pub const IEMA_VOL_IEMA_PERIOD: usize = 21;
pub const IEMA_VOL_IEMA_SMOOTH: f64 = 0.5;    // inversion factor
pub const IEMA_VOL_VOL_PERIOD: usize = 20;
pub const IEMA_VOL_SPIKE_MULT: f64 = 2.0;     // volume must be 2x average
pub const IEMA_VOL_SL: f64 = 0.005;
pub const IEMA_VOL_TP: f64 = 0.004;
pub const IEMA_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run400_1_iema_vol_backtest.py)
2. **Walk-forward** (run400_2_iema_vol_wf.py)
3. **Combined** (run400_3_combined.py)

## Out-of-Sample Testing

- IEMA_PERIOD sweep: 14 / 21 / 30
- IEMA_SMOOTH sweep: 0.3 / 0.5 / 0.7
- VOL_PERIOD sweep: 15 / 20 / 30
- SPIKE_MULT sweep: 1.5 / 2.0 / 2.5
