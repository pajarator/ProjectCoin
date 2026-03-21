# RUN290 — RSI Extreme Extremes: Ultra-Overbought/Oversold

## Hypothesis

**Mechanism**: RSI > 85 = overbought. RSI > 90 = extremely overbought. When RSI reaches extreme levels (90+ or 10-), the probability of mean reversion increases dramatically. Trade RSI extremes: RSI > 90 → SHORT, RSI < 10 → LONG. Require confirmation from a reversal candle.

**Why not duplicate**: No prior RUN uses RSI extreme levels (90+/10-). All prior RSI RUNs use standard thresholds (70/30) or RSI divergence. Extreme levels are distinct because they identify *maximum* momentum conditions.

## Proposed Config Changes (config.rs)

```rust
// ── RUN290: RSI Extreme Extremes ─────────────────────────────────────────
// extreme_overbought = RSI > 90
// extreme_oversold = RSI < 10
// LONG: RSI < 10 AND reversal candle confirmed
// SHORT: RSI > 90 AND reversal candle confirmed

pub const RSI_EXTREME_ENABLED: bool = true;
pub const RSI_EXTREME_PERIOD: usize = 14;
pub const RSI_EXTREME_OVERSOLD: f64 = 10.0;
pub const RSI_EXTREME_OVERBOUGHT: f64 = 90.0;
pub const RSI_EXTREME_SL: f64 = 0.005;
pub const RSI_EXTREME_TP: f64 = 0.004;
pub const RSI_EXTREME_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run290_1_rsi_extreme_backtest.py)
2. **Walk-forward** (run290_2_rsi_extreme_wf.py)
3. **Combined** (run290_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- OVERSOLD sweep: 5 / 10 / 15
- OVERBOUGHT sweep: 85 / 90 / 95
