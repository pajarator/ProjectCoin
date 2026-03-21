# RUN173 — ADX Adaptive Z-Score Thresholds: Dynamic Entry Thresholds Based on Trend Strength

## Hypothesis

**Mechanism**: COINCLAW uses fixed Z-score thresholds for entries (e.g., z < -1.5 for LONG). But in strong trending markets, Z-score can stay extreme for many bars — requiring tighter thresholds to avoid chasing. In ranging markets, thresholds should be looser to catch reversals earlier. ADX measures trend strength — use ADX to dynamically scale Z-score thresholds.

**Why not duplicate**: No prior RUN adapts Z-score thresholds based on ADX. All prior Z-score RUNs use fixed thresholds.

## Proposed Config Changes (config.rs)

```rust
// ── RUN173: ADX-Adaptive Z-Score Thresholds ───────────────────────────
// Trend strength → ADX multiplier for Z-score thresholds
// High ADX (>30, strong trend): tighten thresholds (|z| >= 2.0)
// Low ADX (<20, ranging): loosen thresholds (|z| >= 1.0)

pub const ADAPT_Z_ENABLED: bool = true;
pub const ADAPT_Z_ADX_STRONG: f64 = 30.0;  // ADX above this = strong trend
pub const ADAPT_Z_ADX_WEAK: f64 = 20.0;    // ADX below this = ranging
pub const ADAPT_Z_MULT_STRONG: f64 = 1.33; // multiply Z threshold by 1.33 in strong trend
pub const ADAPT_Z_MULT_WEAK: f64 = 0.67;   // multiply Z threshold by 0.67 in ranging
```

Modify regime entry:

```rust
// Instead of: if ind.z < -1.5
// Compute effective threshold based on ADX
fn adaptive_z_threshold(ind: &Ind15m) -> f64 {
    let base = 1.5;  // base Z threshold
    if ind.adx > config::ADAPT_Z_ADX_STRONG {
        base * config::ADAPT_Z_MULT_STRONG
    } else if ind.adx < config::ADAPT_Z_ADX_WEAK {
        base * config::ADAPT_Z_MULT_WEAK
    } else {
        base
    }
}
```

---

## Validation Method

1. **Historical backtest** (run173_1_adaptz_backtest.py)
2. **Walk-forward** (run173_2_adaptz_wf.py)
3. **Combined** (run173_3_combined.py)

## Out-of-Sample Testing

- ADX_STRONG sweep: 25 / 30 / 35
- MULT_STRONG sweep: 1.25 / 1.33 / 1.5
- MULT_WEAK sweep: 0.50 / 0.67 / 0.75
