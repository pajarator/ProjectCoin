# RUN412 — Adaptive RSI with Volume Confirmation

## Hypothesis

**Mechanism**: Standard RSI uses a fixed period, but market conditions change. An Adaptive RSI adjusts its period based on market volatility — high volatility = longer period (noise filtering), low volatility = shorter period (responsiveness). Volume Confirmation adds institutional backing check: when Adaptive RSI fires a signal AND volume is above its moving average in the direction of the trade, the signal has both adaptive momentum AND volume-backed conviction.

**Why not duplicate**: RUN375 uses RSI Adaptive Period with KST. This RUN uses Adaptive RSI with Volume Confirmation — the distinct mechanism is using volume confirmation (not KST) as the secondary filter, and specifically the volume MA relationship rather than volume percentile ranking.

## Proposed Config Changes (config.rs)

```rust
// ── RUN412: Adaptive RSI with Volume Confirmation ─────────────────────────────────
// adaptive_rsi: rsi_period = base_period * (atr_ratio / atr_ratio_avg)
// adapts: longer period in volatile markets, shorter in calm
// vol_confirmation: volume > SMA(volume, period) in direction of trade
// rsi_signal: adaptive_rsi crosses above/below thresholds
// LONG: adaptive_rsi < RSI_OVERSOLD AND volume > vol_sma
// SHORT: adaptive_rsi > RSI_OVERBOUGHT AND volume > vol_sma

pub const ADAPT_RSI_VOL_ENABLED: bool = true;
pub const ADAPT_RSI_BASE_PERIOD: usize = 14;
pub const ADAPT_RSI_ATR_PERIOD: usize = 14;
pub const ADAPT_RSI_RSI_OVERSOLD: f64 = 30.0;
pub const ADAPT_RSI_RSI_OVERBOUGHT: f64 = 70.0;
pub const ADAPT_RSI_VOL_PERIOD: usize = 20;
pub const ADAPT_RSI_VOL_MULT: f64 = 1.2;   // volume must be above mult * sma
pub const ADAPT_RSI_SL: f64 = 0.005;
pub const ADAPT_RSI_TP: f64 = 0.004;
pub const ADAPT_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run412_1_adapt_rsi_vol_backtest.py)
2. **Walk-forward** (run412_2_adapt_rsi_vol_wf.py)
3. **Combined** (run412_3_combined.py)

## Out-of-Sample Testing

- BASE_PERIOD sweep: 10 / 14 / 21
- ATR_PERIOD sweep: 10 / 14 / 21
- RSI_OVERSOLD sweep: 25 / 30 / 35
- RSI_OVERBOUGHT sweep: 65 / 70 / 75
- VOL_MULT sweep: 1.0 / 1.2 / 1.5
