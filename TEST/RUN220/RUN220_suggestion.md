# RUN220 — Ultimate Oscillator: Multi-Timeframe Momentum Convergence

## Hypothesis

**Mechanism**: Ultimate Oscillator = weighted average of three RSI-like values from different timeframes (7, 14, 28 periods). The weighting gives more importance to shorter timeframes. By combining multiple timeframes, it reduces the false signal problem that plagues single-timeframe oscillators. Divergence from price signals reversal.

**Why not duplicate**: No prior RUN uses Ultimate Oscillator. All prior oscillators use single-timeframe RSI or MACD. The Ultimate Oscillator's multi-timeframe design is specifically engineered to reduce the "divergence false signal" problem — a fundamentally different approach to momentum.

## Proposed Config Changes (config.rs)

```rust
// ── RUN220: Ultimate Oscillator ─────────────────────────────────────────
// Buying Pressure (BP) = close - min(low, prev_close)
// True Range (TR) = max(H, prev_close) - min(L, prev_close)
// Average7 = EMA(BP/TR, 7), Average14 = EMA(BP/TR, 14), Average28 = EMA(BP/TR, 28)
// Ultimate = 100 × (4×avg7 + 2×avg14 + avg28) / (4+2+1)
// LONG: Ultimate crosses above oversold (30)
// SHORT: Ultimate crosses below overbought (70)

pub const ULTIMATE_ENABLED: bool = true;
pub const ULTIMATE_PERIOD1: usize = 7;       // short period
pub const ULTIMATE_PERIOD2: usize = 14;      // medium period
pub const ULTIMATE_PERIOD3: usize = 28;      // long period
pub const ULTIMATE_OVERSOLD: f64 = 30.0;     // oversold threshold
pub const ULTIMATE_OVERBOUGHT: f64 = 70.0;   // overbought threshold
pub const ULTIMATE_SL: f64 = 0.005;
pub const ULTIMATE_TP: f64 = 0.004;
pub const ULTIMATE_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run220_1_ultimate_backtest.py)
2. **Walk-forward** (run220_2_ultimate_wf.py)
3. **Combined** (run220_3_combined.py)

## Out-of-Sample Testing

- PERIOD1 sweep: 5 / 7 / 10
- PERIOD2 sweep: 10 / 14 / 20
- PERIOD3 sweep: 20 / 28 / 40
