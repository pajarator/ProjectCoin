# RUN237 — Williams %R with EMA Trend Filter: Counter-Trend Trade Prevention

## Hypothesis

**Mechanism**: Williams %R is an overbought/oversold oscillator. But buying oversold can be dangerous in a downtrend. Add an EMA filter: only take LONG when Williams %R is oversold (<-80) AND price > EMA200 (major trend is up). Only take SHORT when Williams %R is overbought (>-20) AND price < EMA200 (major trend is down). The EMA filter prevents fighting the major trend.

**Why not duplicate**: No prior RUN combines Williams %R with EMA trend filter. All prior Williams %R RUNs use it without trend confirmation. EMA filter fundamentally changes the signal quality by only allowing trades in the direction of the major trend.

## Proposed Config Changes (config.rs)

```rust
// ── RUN237: Williams %R with EMA Trend Filter ────────────────────────────
// williams_r = (highest_high - close) / (highest_high - lowest_low) × -100
// LONG: williams_r < -80 (oversold) AND close > ema_200
// SHORT: williams_r > -20 (overbought) AND close < ema_200

pub const WILLiams_EMA_ENABLED: bool = true;
pub const WILLIAMS_PERIOD: usize = 14;       // Williams %R lookback
pub const WILLIAMS_OVERSOLD: f64 = -80.0;    // oversold threshold
pub const WILLIAMS_OVERBOUGHT: f64 = -20.0;  // overbought threshold
pub const WILLIAMS_EMA_PERIOD: usize = 200;  // major trend EMA
pub const WILLIAMS_SL: f64 = 0.005;
pub const WILLIAMS_TP: f64 = 0.004;
pub const WILLIAMS_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run237_1_williams_ema_backtest.py)
2. **Walk-forward** (run237_2_williams_ema_wf.py)
3. **Combined** (run237_3_combined.py)

## Out-of-Sample Testing

- WILLIAMS_PERIOD sweep: 10 / 14 / 21
- EMA_PERIOD sweep: 100 / 200 / 300
