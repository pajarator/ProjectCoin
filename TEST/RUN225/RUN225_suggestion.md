# RUN225 — Volatility Compression Breakout: Low-Volatility Expansion Predictor

## Hypothesis

**Mechanism**: Markets alternate between high-volatility expansion and low-volatility compression. When ATR(14) drops below the 20-period ATR MA (compression) AND price is within 0.5% of a support/resistance level, a volatility expansion is imminent. Trade the breakout: if price breaks above resistance during compression → LONG. If price breaks below support → SHORT.

**Why not duplicate**: No prior RUN trades volatility compression specifically. All prior volatility RUNs use ATR for stops or Keltner channels for entries. This strategy specifically uses the *transition* from low volatility to high volatility as the entry signal — a distinct timing mechanism.

## Proposed Config Changes (config.rs)

```rust
// ── RUN225: Volatility Compression Breakout ──────────────────────────────
// atr_current = ATR(14)
// atr_ma = SMA(ATR, 20)
// compression = atr_current < atr_ma × 0.8
// When compression AND price within 0.5% of S/R → breakout imminent
// LONG: price breaks above resistance during compression
// SHORT: price breaks below support during compression

pub const VOL_COMPRESS_ENABLED: bool = true;
pub const VOL_COMPRESS_ATR_PERIOD: usize = 14;
pub const VOL_COMPRESS_ATR_MA_PERIOD: usize = 20;
pub const VOL_COMPRESS_THRESH: f64 = 0.80;    // ATR must be < 80% of ATR MA
pub const VOL_COMPRESS_SR_DIST: f64 = 0.005;  // price within 0.5% of S/R
pub const VOL_COMPRESS_SL: f64 = 0.005;
pub const VOL_COMPRESS_TP: f64 = 0.004;
pub const VOL_COMPRESS_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run225_1_vol_comp_backtest.py)
2. **Walk-forward** (run225_2_vol_comp_wf.py)
3. **Combined** (run225_3_combined.py)

## Out-of-Sample Testing

- ATR_PERIOD sweep: 10 / 14 / 20
- ATR_MA_PERIOD sweep: 15 / 20 / 30
- COMPRESS_THRESH sweep: 0.7 / 0.8 / 0.9
- SR_DIST sweep: 0.003 / 0.005 / 0.007
