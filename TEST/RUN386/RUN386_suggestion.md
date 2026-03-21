# RUN386 — TRIX Momentum with RSI Trade Timing Filter

## Hypothesis

**Mechanism**: TRIX (Triple Exponential Average) is a smoothed momentum oscillator that filters out market noise by triple-smoothing price data. However, TRIX can still lag and produce signals in volatile zones. Pair TRIX with RSI as a trade timing filter: only take TRIX crossover signals when RSI is in a favorable zone (not extreme overbought/oversold, but in the 35-65 comfort zone where momentum moves are cleanest). This combination gets TRIX's trend direction AND RSI's timing precision for entries.

**Why not duplicate**: RUN311 uses TRIX Triple Smooth Oscillator. RUN356 uses RSI Double Smooth with MACD Confluence. This RUN specifically uses TRIX (distinct from RSI-based double smoothing) with RSI as a timing filter rather than as a primary signal — the distinct mechanism is using RSI zone conditions to time TRIX entries, not TRIX entries filtered by RSI extremes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN386: TRIX Momentum with RSI Trade Timing Filter ─────────────────────────
// trix = 100 * (EMA(EMA(EMA(close, N), N), N) / EMA(EMA(EMA(close, N-1), N-1), N-1) - 1)
// trix_signal = EMA(trix, signal_period)
// rsi_zone: rsi between 35 and 65 = valid timing window
// LONG: trix crosses above trix_signal AND rsi between 35-65
// SHORT: trix crosses below trix_signal AND rsi between 35-65

pub const TRIX_RSI_ENABLED: bool = true;
pub const TRIX_RSI_TRIX_PERIOD: usize = 15;
pub const TRIX_RSI_SIGNAL_PERIOD: usize = 9;
pub const TRIX_RSI_RSI_PERIOD: usize = 14;
pub const TRIX_RSI_RSI_LOWER: f64 = 35.0;
pub const TRIX_RSI_RSI_UPPER: f64 = 65.0;
pub const TRIX_RSI_SL: f64 = 0.005;
pub const TRIX_RSI_TP: f64 = 0.004;
pub const TRIX_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run386_1_trix_rsi_backtest.py)
2. **Walk-forward** (run386_2_trix_rsi_wf.py)
3. **Combined** (run386_3_combined.py)

## Out-of-Sample Testing

- TRIX_PERIOD sweep: 12 / 15 / 18
- SIGNAL_PERIOD sweep: 7 / 9 / 12
- RSI_LOWER sweep: 30 / 35 / 40
- RSI_UPPER sweep: 60 / 65 / 70
