# RUN313 — Fisher Transform: Non-Linear Price Transform for Sharp Reversals

## Hypothesis

**Mechanism**: Fisher Transform converts price data into a Gaussian normal distribution using the inverse of the cumulative function. This makes turning points clearer — when Fisher Transform crosses above +2 = price is statistically extended high (expect reversion). When Fisher crosses below -2 = price is statistically extended low (expect bounce). The transformed values are sharp and clearly defined compared to raw prices.

**Why not duplicate**: No prior RUN uses Fisher Transform. This is distinct from Z-score (already in COINCLAW) because Fisher is non-linear and specifically designed to create sharp turning points with near-Gaussian distribution. Z-score is linear; Fisher has a stronger compression effect on extreme values.

## Proposed Config Changes (config.rs)

```rust
// ── RUN313: Fisher Transform ──────────────────────────────────────────────────
// fisher = 0.5 * ln((1+sin_n) / (1-sin_n)) where sin_n = 2*hl2 - hl2_max - hl2_min normalized
// LONG: fisher crosses above FISHER_TRIGGER from below (price extended low)
// SHORT: fisher crosses below -FISHER_TRIGGER from above (price extended high)
// Exit: fisher crosses back through zero (normalization complete)

pub const FISHER_ENABLED: bool = true;
pub const FISHER_PERIOD: usize = 10;          // lookback for high/low range
pub const FISHER_TRIGGER: f64 = 2.0;         // threshold for extreme signal
pub const FISHER_EXIT_ZERO: bool = true;      // exit on zero cross vs threshold
pub const FISHER_SL: f64 = 0.005;
pub const FISHER_TP: f64 = 0.004;
pub const FISHER_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run313_1_fisher_backtest.py)
2. **Walk-forward** (run313_2_fisher_wf.py)
3. **Combined** (run313_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 7 / 10 / 14
- TRIGGER sweep: 1.5 / 2.0 / 2.5
- EXIT_ZERO sweep: true / false
