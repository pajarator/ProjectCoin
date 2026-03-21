# RUN409 — Volume-Price Correlation with Williams %R Extreme

## Hypothesis

**Mechanism**: Volume-Price Correlation measures how tightly volume and price move together. When they have high positive correlation, price moves are backed by volume (institutional). When they diverge (low or negative correlation), price moves lack conviction. Williams %R identifies overbought/oversold extremes. When Williams %R reaches extreme AND Volume-Price Correlation is high, the extreme has institutional backing and is more likely to result in a reversal.

**Why not duplicate**: RUN329 uses Price Volume Rank Correlation. RUN371 uses Williams %R with RSI Confluence. This RUN specifically uses Volume-Price Correlation (which measures institutional backing) with Williams %R extremes — the distinct mechanism is requiring high volume-price correlation at Williams %R extremes, filtering out extremes that lack institutional confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN409: Volume-Price Correlation with Williams %R Extreme ────────────────────────────────
// vol_price_corr = correlation(volume, price_changes, period)
// high_corr = vol_price_corr > CORR_THRESH (institutional backing present)
// williams_r = (highest_high - close) / (highest_high - lowest_low) * -100
// wr_extreme: williams_r < WR_OVERSOLD or > WR_OVERBOUGHT
// LONG: williams_r < WR_OVERSOLD AND high_corr (oversold with institutional backing)
// SHORT: williams_r > WR_OVERBOUGHT AND high_corr (overbought with institutional backing)

pub const VPC_WR_ENABLED: bool = true;
pub const VPC_WR_CORR_PERIOD: usize = 20;
pub const VPC_WR_CORR_THRESH: f64 = 0.6;       // high correlation threshold
pub const VPC_WR_WR_PERIOD: usize = 14;
pub const VPC_WR_WR_OVERSOLD: f64 = -80.0;
pub const VPC_WR_WR_OVERBOUGHT: f64 = -20.0;
pub const VPC_WR_SL: f64 = 0.005;
pub const VPC_WR_TP: f64 = 0.004;
pub const VPC_WR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run409_1_vpc_wr_backtest.py)
2. **Walk-forward** (run409_2_vpc_wr_wf.py)
3. **Combined** (run409_3_combined.py)

## Out-of-Sample Testing

- CORR_PERIOD sweep: 14 / 20 / 30
- CORR_THRESH sweep: 0.5 / 0.6 / 0.7
- WR_PERIOD sweep: 10 / 14 / 21
- WR_OVERSOLD sweep: -85 / -80 / -75
