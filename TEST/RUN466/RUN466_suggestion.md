# RUN466 — Random Walk Index with Choppiness Index Filter

## Hypothesis

**Mechanism**: Random Walk Index (RWI) measures the directional quality of price movements by comparing price moves to random walk expectations. RWI > 1 indicates trending behavior; RWI < 1 indicates random movement. Choppiness Index (CI) distinguishes between trending and ranging markets: low CI (<38) = trending, high CI (>62) = choppy/ranging. This combination ensures RWI signals are only taken when the market regime supports trend following.

**Why not duplicate**: RUN431 uses Random Walk Index with Volume Confirmation. This RUN uses Choppiness Index instead — the distinct mechanism is regime-aware filtering via CI versus volume-based filtering. CI directly addresses whether the market is trend-able before taking a RWI signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN466: Random Walk Index with Choppiness Index Filter ─────────────────────────────────
// rwi: random_walk_index measuring directional quality vs random noise
// rwi_cross: rwi crosses above/below 1.0 threshold
// choppiness_index: ci < 38 trending, > 62 choppy, 38-62 neutral
// LONG: rwi_bullish > 1.0 AND ci < 45 (trending or transitioning to trending)
// SHORT: rwi_bearish > 1.0 AND ci < 45

pub const RWI_CI_ENABLED: bool = true;
pub const RWI_CI_RWI_PERIOD: usize = 14;
pub const RWI_CI_RWI_THRESH: f64 = 1.0;
pub const RWI_CI_CI_PERIOD: usize = 14;
pub const RWI_CI_CI_THRESH: f64 = 45.0;
pub const RWI_CI_SL: f64 = 0.005;
pub const RWI_CI_TP: f64 = 0.004;
pub const RWI_CI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run466_1_rwi_ci_backtest.py)
2. **Walk-forward** (run466_2_rwi_ci_wf.py)
3. **Combined** (run466_3_combined.py)

## Out-of-Sample Testing

- RWI_PERIOD sweep: 10 / 14 / 20
- RWI_THRESH sweep: 0.8 / 1.0 / 1.2
- CI_PERIOD sweep: 10 / 14 / 20
- CI_THRESH sweep: 40 / 45 / 50
