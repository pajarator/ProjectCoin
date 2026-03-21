# RUN354 — MFI Cross-DI Hybrid: Volume-Oscillator Composite Signal

## Hypothesis

**Mechanism**: MFI (Money Flow Index) measures buying and selling pressure based on volume and price. +DI/-DI measure directional movement. Combine them: MFI must be rising (confirming volume-backed price movement) AND DI crossover must confirm the direction. MFI confirms the "effort" (volume), DI confirms the "result" (directional move). When they agree, the signal is volume-backed directional momentum.

**Why not duplicate**: RUN187 uses basic MFI. RUN200 uses DMI/ADX. No RUN combines MFI with DI crossover. The distinct mechanism is the composite signal: MFI for volume-weighted momentum confirmation, DI for directional confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN354: MFI Cross-DI Hybrid ───────────────────────────────────────────────
// mfi = money_flow_index(period)
// di_plus_cross_up = +DI crosses above -DI
// di_plus_cross_down = +DI crosses below -DI
// mfi_rising = mfi > mfi[1] AND mfi > mfi_ma
// LONG: di_plus_cross_up AND mfi_rising AND mfi > 50
// SHORT: di_plus_cross_down AND NOT mfi_rising AND mfi < 50

pub const MFI_DI_ENABLED: bool = true;
pub const MFI_DI_MFI_PERIOD: usize = 14;
pub const MFI_DI_MA_PERIOD: usize = 14;     // MFI SMA for smoothing
pub const MFI_DI_DI_PERIOD: usize = 14;
pub const MFI_DI_MFI_MID: f64 = 50.0;      // MFI midpoint
pub const MFI_DI_SL: f64 = 0.005;
pub const MFI_DI_TP: f64 = 0.004;
pub const MFI_DI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run354_1_mfi_di_backtest.py)
2. **Walk-forward** (run354_2_mfi_di_wf.py)
3. **Combined** (run354_3_combined.py)

## Out-of-Sample Testing

- MFI_PERIOD sweep: 10 / 14 / 21
- MFI_MID sweep: 45 / 50 / 55
- DI_PERIOD sweep: 10 / 14 / 21
