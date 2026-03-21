# RUN280 — Bollinger Band %B with Volume Confirmation

## Hypothesis

**Mechanism**: %B = (close - lower_band) / (upper_band - lower_band). It shows where price is relative to the bands. Combine %B with volume: when %B < 0 (below lower band) AND volume > vol_ma → strong oversold → LONG. When %B > 1 (above upper band) AND volume > vol_ma → strong overbought → SHORT.

**Why not duplicate**: RUN248 uses BB Momentum Score (position + momentum). This RUN uses %B with volume confirmation as entry criteria — a distinct combination of oscillator position and volume.

## Proposed Config Changes (config.rs)

```rust
// ── RUN280: Bollinger Band %B with Volume ─────────────────────────────────
// bb_pct_b = (close - lower) / (upper - lower)
// vol_ma = SMA(volume, 20)
// LONG: bb_pct_b < 0 AND volume > vol_ma × 1.5
// SHORT: bb_pct_b > 1 AND volume > vol_ma × 1.5

pub const BB_PCTB_VOL_ENABLED: bool = true;
pub const BB_PCTB_VOL_PERIOD: usize = 20;
pub const BB_PCTB_VOL_STD: f64 = 2.0;
pub const BB_PCTB_VOL_VOL_MA: usize = 20;
pub const BB_PCTB_VOL_VOL_MULT: f64 = 1.5;
pub const BB_PCTB_VOL_SL: f64 = 0.005;
pub const BB_PCTB_VOL_TP: f64 = 0.004;
pub const BB_PCTB_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run280_1_bb_pctb_vol_backtest.py)
2. **Walk-forward** (run280_2_bb_pctb_vol_wf.py)
3. **Combined** (run280_3_combined.py)

## Out-of-Sample Testing

- BB_PERIOD sweep: 15 / 20 / 30
- BB_STD sweep: 1.5 / 2.0 / 2.5
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
