# RUN365 — Volume-Price Correlation Divergence

## Hypothesis

**Mechanism**: The correlation between price changes and volume changes over a rolling window reveals whether the current move is backed by genuine volume or just noise. When correlation is high (>0.7) → price and volume are moving together = genuine trend. When correlation is low (<0.3) → divergence = likely reversal. Use correlation as a filter: only enter when correlation confirms the directional move.

**Why not duplicate**: No prior RUN uses correlation between price and volume as an indicator. This is distinct from all volume-based and momentum-based indicators because it measures the statistical relationship between price and volume direction, not their absolute levels.

## Proposed Config Changes (config.rs)

```rust
// ── RUN365: Volume-Price Correlation Divergence ───────────────────────────────
// corr(price_ret, vol_ret, period) = rolling correlation between % changes
// corr_low = correlation < CORR_LOW (divergence = reversal signal)
// corr_high = correlation > CORR_HIGH (aligned = trend confirmation)
// LONG: price rising AND volume declining AND correlation < CORR_LOW (divergence warning)
// SHORT: price falling AND volume declining AND correlation < CORR_LOW
// Confirm: when correlation rises above CORR_HIGH, trend is confirmed

pub const VP_CORR_ENABLED: bool = true;
pub const VP_CORR_PERIOD: usize = 20;
pub const VP_CORR_LOW: f64 = 0.3;         // below this = divergence
pub const VP_CORR_HIGH: f64 = 0.7;         // above this = confirmed trend
pub const VP_CORR_SL: f64 = 0.005;
pub const VP_CORR_TP: f64 = 0.004;
pub const VP_CORR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run365_1_vp_corr_backtest.py)
2. **Walk-forward** (run365_2_vp_corr_wf.py)
3. **Combined** (run365_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- CORR_LOW sweep: 0.2 / 0.3 / 0.4
- CORR_HIGH sweep: 0.6 / 0.7 / 0.8
