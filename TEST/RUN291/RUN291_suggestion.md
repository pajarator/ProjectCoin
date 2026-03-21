# RUN291 — Double Seven Pattern: 7-Day Oversold Bounce

## Hypothesis

**Mechanism**: The Double Seven is a candlestick pattern where: (1) RSI(14) stays below 30 for 7 consecutive bars (sustained oversold), (2) then a bullish candle forms. The sustained oversold followed by reversal = high probability bounce. Similar logic for shorts: RSI stays above 70 for 7 bars then bearish candle.

**Why not duplicate**: No prior RUN uses consecutive-bar RSI extremes. All prior RSI RUNs use single-bar thresholds. Double Seven is distinct because it requires *sustained* extreme RSI, not just a one-bar reading.

## Proposed Config Changes (config.rs)

```rust
// ── RUN291: Double Seven Pattern ─────────────────────────────────────────
// double_seven_bull = RSI < 30 for 7 consecutive bars AND close > open
// double_seven_bear = RSI > 70 for 7 consecutive bars AND close < open
// LONG: double_seven_bull confirmed
// SHORT: double_seven_bear confirmed

pub const DOUBLE7_ENABLED: bool = true;
pub const DOUBLE7_RSI_PERIOD: usize = 14;
pub const DOUBLE7_RSI_OVERSOLD: f64 = 30.0;
pub const DOUBLE7_RSI_OVERBOUGHT: f64 = 70.0;
pub const DOUBLE7_CONSEC_BARS: u32 = 7;
pub const DOUBLE7_SL: f64 = 0.005;
pub const DOUBLE7_TP: f64 = 0.004;
pub const DOUBLE7_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run291_1_double7_backtest.py)
2. **Walk-forward** (run291_2_double7_wf.py)
3. **Combined** (run291_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- CONSEC_BARS sweep: 5 / 7 / 9
- OVERSOLD sweep: 20 / 30 / 40
- OVERBOUGHT sweep: 60 / 70 / 80
