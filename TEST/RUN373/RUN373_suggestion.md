# RUN373 — MACD Double Signal Line Crossover

## Hypothesis

**Mechanism**: Standard MACD uses one signal line (EMA of MACD). This RUN uses two signal lines: a fast signal (short EMA of MACD) and a slow signal (long EMA of MACD). When the fast signal crosses above the slow signal → early entry. When MACD crosses above the slow signal (while fast is already above slow) → late entry with more confirmation. Trade both signals with different position sizing: early = smaller position, late = larger position.

**Why not duplicate**: No prior RUN uses double signal lines for MACD. RUN249, RUN277, RUN289, RUN308 all use single MACD signal lines. The distinct mechanism is using two signal lines with different periods to create an early/late entry system with tiered position sizing.

## Proposed Config Changes (config.rs)

```rust
// ── RUN373: MACD Double Signal Line Crossover ────────────────────────────────
// macd_line = EMA(fast) - EMA(slow)
// signal_fast = EMA(macd, FAST_SIGNAL)
// signal_slow = EMA(macd, SLOW_SIGNAL)
// early_cross_up = signal_fast crosses above signal_slow
// late_cross = macd crosses above signal_slow while signal_fast > signal_slow
// Early signal = smaller size; Late signal = larger size

pub const MACD_DBL_ENABLED: bool = true;
pub const MACD_DBL_FAST: usize = 12;
pub const MACD_DBL_SLOW: usize = 26;
pub const MACD_DBL_SIGNAL_FAST: usize = 5;
pub const MACD_DBL_SIGNAL_SLOW: usize = 15;
pub const MACD_DBL_EARLY_SIZE: f64 = 0.5;   // 50% of normal size for early signal
pub const MACD_DBL_SL: f64 = 0.005;
pub const MACD_DBL_TP: f64 = 0.004;
pub const MACD_DBL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run373_1_macd_dbl_backtest.py)
2. **Walk-forward** (run373_2_macd_dbl_wf.py)
3. **Combined** (run373_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 8 / 12 / 16
- SLOW sweep: 20 / 26 / 34
- SIGNAL_FAST sweep: 3 / 5 / 7
- SIGNAL_SLOW sweep: 10 / 15 / 20
