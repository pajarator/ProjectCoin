# RUN273 — Stochastic RSI Percentile Rank: Dual-Oscillator Historical Extremes

## Hypothesis

**Mechanism**: Apply percentile rank to Stochastic RSI values. When StochRSI is at 95th percentile → extremely overbought on a relative basis. When at 5th percentile → extremely oversold. The combination of two oscillation transforms (RSI → Stochastic) AND percentile ranking creates a highly responsive signal.

**Why not duplicate**: RUN199 uses Stochastic RSI with fixed thresholds. RUN262 uses RSI percentile. StochRSI percentile is distinct because it applies percentile ranking to the already-transformed StochRSI data.

## Proposed Config Changes (config.rs)

```rust
// ── RUN273: Stochastic RSI Percentile Rank ──────────────────────────────
// stoch_rsi = stochastic(RSI(close, 14), 14)
// stoch_rsi_pct = percentile rank of stoch_rsi in its history
// stoch_rsi_pct > 90 → extremely overbought → SHORT
// stoch_rsi_pct < 10 → extremely oversold → LONG

pub const STOCH_RSI_PCT_ENABLED: bool = true;
pub const STOCH_RSI_PCT_RSI: usize = 14;
pub const STOCH_RSI_PCT_STOCH: usize = 14;
pub const STOCH_RSI_PCT_WINDOW: usize = 100;
pub const STOCH_RSI_PCT_OVERSOLD: f64 = 10.0;
pub const STOCH_RSI_PCT_OVERBOUGHT: f64 = 90.0;
pub const STOCH_RSI_PCT_SL: f64 = 0.005;
pub const STOCH_RSI_PCT_TP: f64 = 0.004;
pub const STOCH_RSI_PCT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run273_1_stoch_rsi_pct_backtest.py)
2. **Walk-forward** (run273_2_stoch_rsi_pct_wf.py)
3. **Combined** (run273_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- STOCH_PERIOD sweep: 10 / 14 / 21
- WINDOW sweep: 50 / 100 / 200
