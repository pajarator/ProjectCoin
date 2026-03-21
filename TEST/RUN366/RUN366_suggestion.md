# RUN366 — Random Walk Index with Trend Mode

## Hypothesis

**Mechanism**: Random Walk Index (RWI) measures whether price movement is random or statistically significant. RWI > 1.0 = statistically significant directional movement (not random). RWI < 1.0 = likely random walk. Use RWI to distinguish trending from ranging markets, then apply different strategies: in trending markets (RWI > threshold), use momentum-following. In ranging markets (RWI < threshold), use mean-reversion.

**Why not duplicate**: RUN231 uses Random Walk Index but as a standalone indicator. This RUN specifically uses RWI to determine the market regime (trending vs ranging) and switches strategy accordingly. The regime-switching based on RWI is the distinct mechanism.

## Proposed Config Changes (config.rs)

```rust
// ── RUN366: Random Walk Index with Trend Mode ────────────────────────────────
// rwi_high(n) = (highest(high, n) - lowest(low, n)) / (ATR(n) * sqrt(n))
// rwi_low(n) = same but inverse
// rwi trending = rwi_high > RWI_TREND_THRESH OR rwi_low > RWI_TREND_THRESH
// rwi ranging = both rwi values below RWI_TREND_THRESH
//
// TRENDING mode: use SMA crossover (momentum-following)
// RANGING mode: use RSI extremes (mean-reversion)

pub const RWI_MODE_ENABLED: bool = true;
pub const RWI_MODE_PERIOD: usize = 14;
pub const RWI_MODE_TREND_THRESH: f64 = 1.0;  // above this = trending
pub const RWI_MODE_RSI_LONG: f64 = 30.0;
pub const RWI_MODE_RSI_SHORT: f64 = 70.0;
pub const RWI_MODE_SMA_PERIOD: usize = 20;
pub const RWI_MODE_SL: f64 = 0.005;
pub const RWI_MODE_TP: f64 = 0.004;
pub const RWI_MODE_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run366_1_rwi_mode_backtest.py)
2. **Walk-forward** (run366_2_rwi_mode_wf.py)
3. **Combined** (run366_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- TREND_THRESH sweep: 0.8 / 1.0 / 1.2
- RSI_LONG sweep: 25 / 30 / 35
- RSI_SHORT sweep: 65 / 70 / 75
