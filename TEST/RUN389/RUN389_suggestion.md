# RUN389 — Accumulation/Distribution Line with SuperTrend Direction Confirmation

## Hypothesis

**Mechanism**: The Accumulation/Distribution (A/D) Line is a volume-based indicator that measures the cumulative flow of money into or out of an asset. It tracks whether buy pressure (accumulation) or sell pressure (distribution) is dominating. SuperTrend provides the trend direction. When A/D triggers a divergence signal (price makes new high but A/D doesn't confirm) AND SuperTrend flips in the direction of the divergence, you have both money flow divergence AND trend confirmation working together.

**Why not duplicate**: RUN301 uses Intraday Intensity Index (volume-based). RUN357 uses Elder Ray Index with ADX Filter. This RUN uses A/D Line specifically (distinct from Elder Ray's high/low comparison method) with SuperTrend as a trend confirmation filter — the distinct mechanism is using A/D divergence signals confirmed by SuperTrend flips.

## Proposed Config Changes (config.rs)

```rust
// ── RUN389: Accumulation/Distribution with SuperTrend Confirmation ───────────────
// ad_line = cumulative((close - open) / (high - low) * volume)
// ad_divergence: price makes new high/low but A/D doesn't confirm
// supertrend: ATR-based trend direction
// LONG: price breaks out AND ad_line rising AND supertrend bullish
// SHORT: price breaks down AND ad_line falling AND supertrend bearish

pub const AD_ST_ENABLED: bool = true;
pub const AD_ST_ST_PERIOD: usize = 10;
pub const AD_ST_ST_MULT: f64 = 3.0;
pub const AD_ST_AD_PERIOD: usize = 14;  // smoothing period for A/D
pub const AD_ST_SL: f64 = 0.005;
pub const AD_ST_TP: f64 = 0.004;
pub const AD_ST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run389_1_ad_st_backtest.py)
2. **Walk-forward** (run389_2_ad_st_wf.py)
3. **Combined** (run389_3_combined.py)

## Out-of-Sample Testing

- ST_PERIOD sweep: 7 / 10 / 14
- ST_MULT sweep: 2.0 / 3.0 / 4.0
- AD_PERIOD sweep: 10 / 14 / 21
