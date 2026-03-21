# RUN289 — MACD Zero Line Rejection: Momentum Zone Reversal

## Hypothesis

**Mechanism**: When MACD line approaches zero from above and bounces → bullish rejection of zero line. When MACD approaches from below and bounces → bearish rejection. The zero line is a major support/resistance for MACD. Rejections at zero are stronger signals than simple crossovers.

**Why not duplicate**: No prior RUN uses MACD zero line rejection. All prior MACD RUNs use crossover or histogram. Zero line rejection is distinct because it identifies when MACD finds support/resistance at the zero level.

## Proposed Config Changes (config.rs)

```rust
// ── RUN289: MACD Zero Line Rejection ───────────────────────────────────
// macd_approaching_zero_from_above = macd > 0 AND macd[prior] > 0 AND macd[prior] > macd
// macd_approaching_zero_from_below = macd < 0 AND macd[prior] < 0 AND macd[prior] < macd
// LONG: macd bounces off zero from above (macd[prior] < 0 AND macd > 0)
// SHORT: macd bounces off zero from below (macd[prior] > 0 AND macd < 0)

pub const MACD_ZERO_REJ_ENABLED: bool = true;
pub const MACD_ZERO_REJ_FAST: usize = 12;
pub const MACD_ZERO_REJ_SLOW: usize = 26;
pub const MACD_ZERO_REJ_SIGNAL: usize = 9;
pub const MACD_ZERO_REJ_SL: f64 = 0.005;
pub const MACD_ZERO_REJ_TP: f64 = 0.004;
pub const MACD_ZERO_REJ_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run289_1_macd_zero_rej_backtest.py)
2. **Walk-forward** (run289_2_macd_zero_rej_wf.py)
3. **Combined** (run289_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 8 / 12 / 16
- SLOW sweep: 20 / 26 / 34
