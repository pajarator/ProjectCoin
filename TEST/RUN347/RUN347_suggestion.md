# RUN347 — Bollinger Band Width with KST Momentum Confluence

## Hypothesis

**Mechanism**: BB Width (%B width as a fraction of the band) measures volatility compression. Low BB Width = market coiled tight. High BB Width = volatility expansion. When BB Width is compressed AND KST momentum crosses its signal line in the same direction → the combination of volatility compression + momentum alignment is a high-probability breakout setup. The BB Width tells you WHEN to expect the move; KST tells you in which direction.

**Why not duplicate**: RUN233 uses BB squeeze with volume. RUN241 uses RSI Bollinger Band squeeze. RUN248 uses BB momentum score. RUN261 uses BB width percentile. This RUN specifically combines BB Width compression with KST momentum crossover — BB Width for timing, KST for direction.

## Proposed Config Changes (config.rs)

```rust
// ── RUN347: Bollinger Band Width with KST Momentum Confluence ────────────────────────
// bb_width = (bb_upper - bb_lower) / bb_mid
// bb_width_percentile = percentile_rank(bb_width, lookback)
// kst_cross_up = kst crosses above kst_signal
// kst_cross_down = kst crosses below kst_signal
//
// LONG: bb_width_percentile < BB_WIDTH_COMPRESS AND kst_cross_up
// SHORT: bb_width_percentile < BB_WIDTH_COMPRESS AND kst_cross_down
// Wait for BB expansion after compression to confirm breakout

pub const BBW_KST_ENABLED: bool = true;
pub const BBW_KST_BB_PERIOD: usize = 20;
pub const BBW_KST_BB_STD: f64 = 2.0;
pub const BBW_KST_WIDTH_LOOKBACK: usize = 100;
pub const BBW_KST_COMPRESS: f64 = 15.0;   // bottom 15th percentile = compressed
pub const BBW_KST_KST_ROC1: usize = 8;
pub const BBW_KST_KST_ROC2: usize = 16;
pub const BBW_KST_KST_ROC3: usize = 24;
pub const BBW_KST_KST_ROC4: usize = 32;
pub const BBW_KST_KST_SIGNAL: usize = 8;
pub const BBW_KST_SL: f64 = 0.005;
pub const BBW_KST_TP: f64 = 0.004;
pub const BBW_KST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run347_1_bbw_kst_backtest.py)
2. **Walk-forward** (run347_2_bbw_kst_wf.py)
3. **Combined** (run347_3_combined.py)

## Out-of-Sample Testing

- COMPRESS sweep: 10 / 15 / 20
- BB_STD sweep: 1.5 / 2.0 / 2.5
- WIDTH_LOOKBACK sweep: 50 / 100 / 200
