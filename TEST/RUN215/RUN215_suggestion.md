# RUN215 — RSI Divergence Scanner: Price-Indicator Discord as Reversal Signal

## Hypothesis

**Mechanism**: RSI Divergence = discord between price and RSI direction. **Bearish divergence**: price makes higher high, RSI makes lower high → reversal down likely. **Bullish divergence**: price makes lower low, RSI makes higher low → reversal up likely. Divergences signal the end of a momentum wave — the current move is losing steam.

**Why not duplicate**: No prior RUN systematically uses RSI divergence. All prior RSI RUNs use absolute thresholds (overbought/oversold). Divergence is a fundamentally different signal type — it compares *changes* in price to changes in RSI, not absolute levels.

## Proposed Config Changes (config.rs)

```rust
// ── RUN215: RSI Divergence Scanner ───────────────────────────────────────
// Lookback for swing highs/lows: N bars
// BULLISH DIVERGENCE: price making lower_low AND RSI making higher_low
// BEARISH DIVERGENCE: price making higher_high AND RSI making lower_high
// min_RSI_CHANGE = RSI difference needed between swings (e.g., > 5)
// LONG: bullish divergence detected
// SHORT: bearish divergence detected

pub const RSI_DIV_ENABLED: bool = true;
pub const RSI_DIV_PERIOD: usize = 14;        // base RSI period
pub const RSI_DIV_SWING: usize = 20;         // lookback for swing detection
pub const RSI_DIV_MIN_CHANGE: f64 = 5.0;      // minimum RSI change between swings
pub const RSI_DIV_SL: f64 = 0.005;
pub const RSI_DIV_TP: f64 = 0.004;
pub const RSI_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run215_1_rsi_div_backtest.py)
2. **Walk-forward** (run215_2_rsi_div_wf.py)
3. **Combined** (run215_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- SWING sweep: 14 / 20 / 30
- MIN_CHANGE sweep: 3 / 5 / 7
