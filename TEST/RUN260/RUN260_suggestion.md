# RUN260 — Multiple Timeframe MACD Alignment: Cross-Frame Momentum Consensus

## Hypothesis

**Mechanism**: When MACD on 15m AND 1h AND 4h are ALL bullish (MACD > signal) → strong upward momentum consensus. When ALL are bearish → strong downward. Requiring all three timeframes to agree filters out noise and produces high-conviction signals. Trade in the direction of the multi-timeframe alignment.

**Why not duplicate**: No prior RUN uses multi-timeframe MACD alignment. All prior multi-timeframe RUNs use RSI or EMA crosses. MACD alignment across timeframes is distinct because it measures *momentum consensus*.

## Proposed Config Changes (config.rs)

```rust
// ── RUN260: Multiple Timeframe MACD Alignment ─────────────────────────────
// macd_15m = MACD(12, 26, 9) on 15m
// macd_1h = MACD(12, 26, 9) on 1h (aggregated from 15m)
// macd_4h = MACD(12, 26, 9) on 4h (aggregated from 15m)
// ALL three must agree for entry
// LONG: macd_15m > signal AND macd_1h > signal AND macd_4h > signal
// SHORT: macd_15m < signal AND macd_1h < signal AND macd_4h < signal

pub const MTF_MACD_ENABLED: bool = true;
pub const MTF_MACD_FAST: usize = 12;
pub const MTF_MACD_SLOW: usize = 26;
pub const MTF_MACD_SIGNAL: usize = 9;
pub const MTF_MACD_SL: f64 = 0.005;
pub const MTF_MACD_TP: f64 = 0.004;
pub const MTF_MACD_MAX_HOLD: u32 = 72;
```

---

## Validation Method

1. **Historical backtest** (run260_1_mtf_macd_backtest.py)
2. **Walk-forward** (run260_2_mtf_macd_wf.py)
3. **Combined** (run260_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 8 / 12 / 16
- SLOW sweep: 20 / 26 / 34
- SIGNAL sweep: 7 / 9 / 12
