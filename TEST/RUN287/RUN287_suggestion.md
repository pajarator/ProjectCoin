# RUN287 — RSI Gap: Oscillator Opening Gap Detection

## Hypothesis

**Mechanism**: Just like price can gap at the open, RSI can "gap" — the difference between current RSI and prior RSI is unusually large. When RSI gaps up > 10 points in one bar → momentum spike, mean-revert. When RSI gaps down > 10 points → momentum drop, mean-revert. RSI gaps identify momentum explosions that are likely to correct.

**Why not duplicate**: No prior RUN uses RSI gaps. All prior RSI RUNs use absolute levels or crossovers. RSI gap is distinct because it measures the *change* in RSI between bars, not the RSI level itself.

## Proposed Config Changes (config.rs)

```rust
// ── RUN287: RSI Gap ───────────────────────────────────────────────────────
// rsi_gap = RSI(close, period) - RSI(close[1], period)
// rsi_gap > 10 → RSI gapped up → overbought short opportunity
// rsi_gap < -10 → RSI gapped down → oversold long opportunity

pub const RSI_GAP_ENABLED: bool = true;
pub const RSI_GAP_PERIOD: usize = 14;
pub const RSI_GAP_THRESH: f64 = 10.0;       // gap size threshold
pub const RSI_GAP_SL: f64 = 0.005;
pub const RSI_GAP_TP: f64 = 0.004;
pub const RSI_GAP_MAX_HOLD: u32 = 24;
```

---

## Validation Method

1. **Historical backtest** (run287_1_rsi_gap_backtest.py)
2. **Walk-forward** (run287_2_rsi_gap_wf.py)
3. **Combined** (run287_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- THRESH sweep: 8 / 10 / 12
