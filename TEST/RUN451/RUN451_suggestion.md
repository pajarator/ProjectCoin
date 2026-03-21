# RUN451 — Fisher Transform with Bollinger Band Position Confirmation

## Hypothesis

**Mechanism**: The Fisher Transform converts price data into a Gaussian normal distribution, making turning points clearer. When Fisher Transform fires a signal (crossing above/below its trigger line), Bollinger Band Position confirms whether price is at an extreme within the BB range. When Fisher signals AND price is at an extreme BB position (near upper or lower band), the signal has both statistical turning point AND price structure confirmation.

**Why not duplicate**: RUN313 uses Fisher Transform standalone. RUN396 uses Fisher Transform with ADX Trend Strength Filter. This RUN specifically uses Bollinger Band Position as the confirmation filter — the distinct mechanism is requiring price to be at a specific position within the BB range at Fisher's turning point signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN451: Fisher Transform with Bollinger Band Position Confirmation ─────────────────────────────────────
// fisher_transform = 0.5 * ln((1 + x) / (1 - x)) where x = 2 * (price - lowest_low) / (highest_high - lowest_low) - 1
// fisher_cross: fisher crosses above/below fisher_trigger
// bb_position = (close - bb_lower) / (bb_upper - bb_lower)
// bb_extreme: bb_position < BB_LOWER for longs, bb_position > BB_UPPER for shorts
// LONG: fisher_cross bullish AND bb_position < BB_LOWER
// SHORT: fisher_cross bearish AND bb_position > BB_UPPER

pub const FISHER_BBP_ENABLED: bool = true;
pub const FISHER_BBP_FISHER_PERIOD: usize = 10;
pub const FISHER_BBP_TRIGGER_PERIOD: usize = 5;
pub const FISHER_BBP_BB_PERIOD: usize = 20;
pub const FISHER_BBP_BB_STD: f64 = 2.0;
pub const FISHER_BBP_BB_LOWER: f64 = 0.15;   // bottom 15% of BB
pub const FISHER_BBP_BB_UPPER: f64 = 0.85;   // top 15% of BB
pub const FISHER_BBP_SL: f64 = 0.005;
pub const FISHER_BBP_TP: f64 = 0.004;
pub const FISHER_BBP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run451_1_fisher_bbp_backtest.py)
2. **Walk-forward** (run451_2_fisher_bbp_wf.py)
3. **Combined** (run451_3_combined.py)

## Out-of-Sample Testing

- FISHER_PERIOD sweep: 7 / 10 / 14
- TRIGGER_PERIOD sweep: 3 / 5 / 7
- BB_PERIOD sweep: 15 / 20 / 30
- BB_LOWER sweep: 0.10 / 0.15 / 0.20
- BB_UPPER sweep: 0.80 / 0.85 / 0.90
