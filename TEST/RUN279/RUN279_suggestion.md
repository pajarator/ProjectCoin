# RUN279 — ADX Consecutive Bars: Trend Strength Acceleration

## Hypothesis

**Mechanism**: ADX rising for N consecutive bars = trend is accelerating. ADX falling for N consecutive bars = trend is weakening. When ADX has risen for 3+ consecutive bars → strong trend confirmed → enter in trend direction. When ADX falls for 3+ bars → trend ending → exit.

**Why not duplicate**: No prior RUN uses ADX consecutive bars. All prior ADX RUNs use absolute thresholds. Consecutive ADX movement is distinct because it measures *acceleration* of trend strength, not just the level.

## Proposed Config Changes (config.rs)

```rust
// ── RUN279: ADX Consecutive Bars ─────────────────────────────────────────
// adx_rising = count of consecutive bars where ADX > ADX[prior]
// adx_falling = count of consecutive bars where ADX < ADX[prior]
// adx_rising >= 3 → strong uptrend → LONG in uptrend direction
// adx_falling >= 3 → trend weakening → exit

pub const ADX_CONSEC_ENABLED: bool = true;
pub const ADX_CONSEC_PERIOD: usize = 14;
pub const ADX_CONSEC_THRESH: u32 = 3;
pub const ADX_CONSEC_SL: f64 = 0.005;
pub const ADX_CONSEC_TP: f64 = 0.004;
pub const ADX_CONSEC_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run279_1_adx_consec_backtest.py)
2. **Walk-forward** (run279_2_adx_consec_wf.py)
3. **Combined** (run279_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- THRESH sweep: 2 / 3 / 4
