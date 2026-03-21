# RUN221 — Stochastic Momentum Index (SMI): Center-of-Range Momentum

## Hypothesis

**Mechanism**: SMI = (close - midpoint of high-low range over period) / (half of high-low range over period). The midpoint is (HH+LL)/2, not the extreme. SMI ranges from -100 to +100. When SMI crosses above -40 from below → bullish. When SMI crosses below +40 from above → bearish. SMI is more refined than standard Stochastic because it uses the center of the range rather than extremes.

**Why not duplicate**: No prior RUN uses SMI. All prior Stochastic RUNs use standard Stochastic (price vs extremes). SMI is specifically designed to be less sensitive and more reliable than standard Stochastic — a meaningful refinement.

## Proposed Config Changes (config.rs)

```rust
// ── RUN221: Stochastic Momentum Index (SMI) ──────────────────────────────
// hhll_range = (highest_high + lowest_low) / 2 over period
// close_diff = close - hhll_range
// half_range = (highest_high - lowest_low) / 2
// SMI = close_diff / half_range × 100
// SMI_MA = EMA(SMI, signal_period) [for signal line]
// LONG: SMI crosses above -40
// SHORT: SMI crosses below +40

pub const SMI_ENABLED: bool = true;
pub const SMI_PERIOD: usize = 14;             // lookback period
pub const SMI_SIGNAL: usize = 3;             // signal EMA period
pub const SMI_OVERSOLD: f64 = -40.0;         // oversold threshold
pub const SMI_OVERBOUGHT: f64 = 40.0;         // overbought threshold
pub const SMI_SL: f64 = 0.005;
pub const SMI_TP: f64 = 0.004;
pub const SMI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run221_1_smi_backtest.py)
2. **Walk-forward** (run221_2_smi_wf.py)
3. **Combined** (run221_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- SIGNAL sweep: 3 / 5 / 7
- OVERSOLD sweep: -50 / -40 / -30
- OVERBOUGHT sweep: 30 / 40 / 50
