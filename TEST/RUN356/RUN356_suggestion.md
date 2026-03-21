# RUN356 — RSI Double Smooth with MACD Confluence

## Hypothesis

**Mechanism**: Apply two EMA smooths to RSI before generating signals. First smooth = EMA(RSI, fast). Second smooth = EMA(first_smooth, slow). The double smoothing reduces false signals and lag. Require MACD to confirm direction: MACD histogram must be rising (for LONG) or falling (for SHORT) at the time of the RSI signal. Both indicators must agree on the direction before entry.

**Why not duplicate**: RUN276 uses RSI Double EMA crossover but without MACD confirmation. This RUN adds MACD confluence on top of the double-smoothed RSI — the distinct mechanism is the dual-oscillator confirmation: double-smoothed RSI for timing, MACD for direction confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN356: RSI Double Smooth with MACD Confluence ────────────────────────────────
// rsi1 = EMA(RSI(close, RSI_PERIOD), FAST_EMA)
// rsi2 = EMA(rsi1, SLOW_EMA)
// macd_histogram rising = macd > macd[1]
// macd_histogram falling = macd < macd[1]
// LONG: rsi2 crosses above RSI_OVERSOLD AND macd_histogram rising
// SHORT: rsi2 crosses below RSI_OVERBOUGHT AND macd_histogram falling

pub const RSI_DBL_MACD_ENABLED: bool = true;
pub const RSI_DBL_MACD_RSI_PERIOD: usize = 14;
pub const RSI_DBL_MACD_FAST: usize = 5;
pub const RSI_DBL_MACD_SLOW: usize = 14;
pub const RSI_DBL_MACD_RSI_OVERSOLD: f64 = 35.0;
pub const RSI_DBL_MACD_RSI_OVERBOUGHT: f64 = 65.0;
pub const RSI_DBL_MACD_MACD_FAST: usize = 12;
pub const RSI_DBL_MACD_MACD_SLOW: usize = 26;
pub const RSI_DBL_MACD_MACD_SIGNAL: usize = 9;
pub const RSI_DBL_MACD_SL: f64 = 0.005;
pub const RSI_DBL_MACD_TP: f64 = 0.004;
pub const RSI_DBL_MACD_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run356_1_rsi_dbl_macd_backtest.py)
2. **Walk-forward** (run356_2_rsi_dbl_macd_wf.py)
3. **Combined** (run356_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 3 / 5 / 8
- SLOW sweep: 10 / 14 / 21
- RSI_OVERSOLD sweep: 30 / 35 / 40
- RSI_OVERBOUGHT sweep: 60 / 65 / 70
