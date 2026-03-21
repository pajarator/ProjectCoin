# RUN394 — Donchian Channel Breakout with Choppiness Index Trend Quality Filter

## Hypothesis

**Mechanism**: Donchian Channel Breakout is a classic trend-following signal — price breaking above the upper band signals bullish momentum, below the lower band signals bearish. However, in choppy markets Donchian breakouts whipsaw badly. Add Choppiness Index (CI): CI measures how choppy vs trending the market is on a scale of 0-100. Values above 61.8 indicate a choppy market; values below 38.2 indicate a trending market. Only take Donchian signals when CI confirms trending conditions (CI < 50). This filters out the majority of false breakouts in ranging markets.

**Why not duplicate**: RUN359 uses Donchian Channel with Volume Surge. RUN335 uses Choppiness Index Trend Mode. This RUN specifically combines Donchian breakout signals with Choppiness Index as a trend quality filter — the distinct mechanism is using CI to gate Donchian entries, ensuring signals only fire in trending conditions.

## Proposed Config Changes (config.rs)

```rust
// ── RUN394: Donchian Channel Breakout with Choppiness Index Trend Filter ─────────────────────
// donchian_upper = highest_high over period
// donchian_lower = lowest_low over period
// donchian_break: close crosses above upper or below lower
// choppiness_index = 100 * log10(sum(ATR, period) / (max(high, prev_close) - min(low, prev_close))) / log10(period)
// trending_condition: ci < CI_THRESH (below = trending)
// LONG: donchian_break to upside AND ci < CI_THRESH
// SHORT: donchian_break to downside AND ci < CI_THRESH

pub const DONCHIAN_CI_ENABLED: bool = true;
pub const DONCHIAN_CI_DC_PERIOD: usize = 20;
pub const DONCHIAN_CI_CI_PERIOD: usize = 14;
pub const DONCHIAN_CI_CI_THRESH: f64 = 50.0;   // below this = trending
pub const DONCHIAN_CI_SL: f64 = 0.005;
pub const DONCHIAN_CI_TP: f64 = 0.004;
pub const DONCHIAN_CI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run394_1_donchian_ci_backtest.py)
2. **Walk-forward** (run394_2_donchian_ci_wf.py)
3. **Combined** (run394_3_combined.py)

## Out-of-Sample Testing

- DC_PERIOD sweep: 15 / 20 / 30
- CI_PERIOD sweep: 10 / 14 / 21
- CI_THRESH sweep: 40 / 50 / 60
