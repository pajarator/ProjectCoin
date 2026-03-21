# RUN406 — Session Volume Imbalance with Stochastic Extreme Confirmation

## Hypothesis

**Mechanism**: Session Volume Imbalance measures the directional bias of volume within a trading session — comparing volume traded on up-ticks vs down-ticks. A strong imbalance (buying pressure vs selling pressure) indicates which side is controlling the session. When the imbalance flips direction (from buying to selling pressure or vice versa) AND Stochastic reaches extreme levels, you have both volume-based directional conviction AND oscillator confirmation of the reversal. The combination catches sessions where volume first dominates one direction then flips.

**Why not duplicate**: RUN301 uses Intraday Intensity Index. RUN372 uses Volume Ratio Spike with RSI. This RUN specifically uses Session Volume Imbalance (measuring up-tick vs down-tick volume direction within a session) with Stochastic extremes — the distinct mechanism is using session-level volume imbalance flips as the primary signal gated by Stochastic extremes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN406: Session Volume Imbalance with Stochastic Extreme Confirmation ──────────────────────
// session_vol_imbalance = (uptick_volume - downtick_volume) / (uptick_volume + downtick_volume)
// imbalance_flip: imbalance crosses from positive to negative or vice versa
// stochastic = stochastic_oscillator(close, period)
// stochastic_extreme: %K in overbought (>80) or oversold (<20) territory
// LONG: imbalance_flip to upside AND stochastic in oversold zone
// SHORT: imbalance_flip to downside AND stochastic in overbought zone

pub const SVOL_STOCH_ENABLED: bool = true;
pub const SVOL_STOCH_IMBALANCE_PERIOD: usize = 20;  // bars to calculate imbalance
pub const SVOL_STOCH_STOCH_PERIOD: usize = 14;
pub const SVOL_STOCH_STOCH_SMOOTH: usize = 3;
pub const SVOL_STOCH_STOCH_OVERSOLD: f64 = 20.0;
pub const SVOL_STOCH_STOCH_OVERBOUGHT: f64 = 80.0;
pub const SVOL_STOCH_SL: f64 = 0.005;
pub const SVOL_STOCH_TP: f64 = 0.004;
pub const SVOL_STOCH_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run406_1_svol_stoch_backtest.py)
2. **Walk-forward** (run406_2_svol_stoch_wf.py)
3. **Combined** (run406_3_combined.py)

## Out-of-Sample Testing

- IMBALANCE_PERIOD sweep: 14 / 20 / 30
- STOCH_PERIOD sweep: 10 / 14 / 21
- STOCH_OVERSOLD sweep: 15 / 20 / 25
- STOCH_OVERBOUGHT sweep: 75 / 80 / 85
