# RUN387 — Price Rate of Change with Bollinger Band Width Compression Filter

## Hypothesis

**Mechanism**: Bollinger Band Width (BBW) measures the distance between upper and lower bands — when it compresses to low values, the market is in a "squeeze" state and is coiled for a explosive move. ROC (Rate of Change) measures momentum. Combine them: when BBW is compressed (squeeze) AND ROC triggers a crossover signal, the momentum move has a higher probability of continuation because the squeeze releases pent-up energy. This is the opposite of taking signals during expanded volatility.

**Why not duplicate**: RUN332 uses Volume-Weighted Bollinger Band Touch. RUN347 uses BB Width with KST Confluence. This RUN specifically uses BB Width as a squeeze/compression filter for ROC momentum signals — the distinct mechanism is using BB width as a regime filter (low width = valid compression zone), which is different from BB touch or KST confluence.

## Proposed Config Changes (config.rs)

```rust
// ── RUN387: Price ROC with Bollinger Band Width Compression Filter ─────────────
// roc = (close - close_n_periods_ago) / close_n_periods_ago * 100
// bb_width = (bb_upper - bb_lower) / bb_middle  (normalized)
// squeeze: bb_width < BB_WIDTH_THRESH means compressed volatile market
// LONG: roc triggers bullish signal AND bb_width < squeeze threshold
// SHORT: roc triggers bearish signal AND bb_width < squeeze threshold

pub const ROC_BBW_ENABLED: bool = true;
pub const ROC_BBW_ROC_PERIOD: usize = 12;
pub const ROC_BBW_ROC_THRESH: f64 = 2.0;   // roc must exceed this to trigger
pub const ROC_BBW_BB_PERIOD: usize = 20;
pub const ROC_BBW_BB_STD: f64 = 2.0;
pub const ROC_BBW_WIDTH_THRESH: f64 = 0.05; // squeeze threshold (normalized)
pub const ROC_BBW_SL: f64 = 0.005;
pub const ROC_BBW_TP: f64 = 0.004;
pub const ROC_BBW_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run387_1_roc_bbw_backtest.py)
2. **Walk-forward** (run387_2_roc_bbw_wf.py)
3. **Combined** (run387_3_combined.py)

## Out-of-Sample Testing

- ROC_PERIOD sweep: 8 / 12 / 16
- ROC_THRESH sweep: 1.5 / 2.0 / 2.5
- BB_PERIOD sweep: 15 / 20 / 30
- WIDTH_THRESH sweep: 0.04 / 0.05 / 0.06
