# RUN431 — Random Walk Index with Volume Confirmation

## Hypothesis

**Mechanism**: The Random Walk Index (RWI) measures the directionalness of price movements by comparing the current price move to what would be expected from a random walk. High RWI values indicate a trending market; low values indicate chop. Volume Confirmation adds institutional backing: when RWI is high (trending) AND volume is above its moving average, the trend has both directional consistency AND volume-backed conviction.

**Why not duplicate**: RUN366 uses Random Walk Index with Trend Mode. This RUN specifically uses Volume Confirmation — the distinct mechanism is using volume above MA to confirm RWI trend signals, filtering out trending markets that lack institutional participation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN431: Random Walk Index with Volume Confirmation ─────────────────────────────────
// random_walk_index = |price_change| / (ATR * sqrt(period))
// high_rwi = rwi > RWI_THRESH (trending market)
// volume_confirmation: volume > SMA(volume, period)
// LONG: rwi_high AND volume > vol_sma AND price trending up
// SHORT: rwi_high AND volume > vol_sma AND price trending down

pub const RWI_VOL_ENABLED: bool = true;
pub const RWI_VOL_RWI_PERIOD: usize = 14;
pub const RWI_VOL_RWI_THRESH: f64 = 1.0;     // above = trending
pub const RWI_VOL_VOL_PERIOD: usize = 20;
pub const RWI_VOL_VOL_MULT: f64 = 1.2;      // volume must be above this * sma
pub const RWI_VOL_SL: f64 = 0.005;
pub const RWI_VOL_TP: f64 = 0.004;
pub const RWI_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run431_1_rwi_vol_backtest.py)
2. **Walk-forward** (run431_2_rwi_vol_wf.py)
3. **Combined** (run431_3_combined.py)

## Out-of-Sample Testing

- RWI_PERIOD sweep: 10 / 14 / 21
- RWI_THRESH sweep: 0.8 / 1.0 / 1.2
- VOL_PERIOD sweep: 14 / 20 / 30
- VOL_MULT sweep: 1.0 / 1.2 / 1.5
