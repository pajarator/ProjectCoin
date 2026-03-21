# RUN292 — ADX Disposition: Rising vs Falling ADX in Trend

## Hypothesis

**Mechanism**: ADX rising in a LONG position = trend strengthening, hold. ADX falling in a LONG position = trend weakening, exit. ADX falling in a SHORT position = trend weakening, exit. ADX rising in a SHORT = trend strengthening, hold. Trade: enter when ADX is rising (confirming trend), exit when ADX stops rising.

**Why not duplicate**: No prior RUN uses ADX direction change as exit signal. All prior ADX RUNs use absolute levels. ADX disposition is distinct because it tracks *changes in trend strength*, not just the level.

## Proposed Config Changes (config.rs)

```rust
// ── RUN292: ADX Disposition ──────────────────────────────────────────────
// adx_rising = ADX > ADX[prior]
// adx_falling = ADX < ADX[prior]
// Enter in trend direction when ADX rising (confirming)
// Exit when ADX peaks and starts falling

pub const ADX_DISP_ENABLED: bool = true;
pub const ADX_DISP_PERIOD: usize = 14;
pub const ADX_DISP_SL: f64 = 0.005;
pub const ADX_DISP_TP: f64 = 0.004;
pub const ADX_DISP_MAX_HOLD: u32 = 48;
```

Modify engine to check ADX direction: if ADX was rising and now falling → exit (trend weakening).

---

## Validation Method

1. **Historical backtest** (run292_1_adx_disp_backtest.py)
2. **Walk-forward** (run292_2_adx_disp_wf.py)
3. **Combined** (run292_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
