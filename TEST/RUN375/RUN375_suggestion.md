# RUN375 — RSI Adaptive Period with KST Momentum

## Hypothesis

**Mechanism**: Standard RSI uses a fixed period (e.g., 14). This RUN adapts the RSI period based on market volatility: high-volatility periods → longer RSI period to avoid overbought/oversold noise. Low-volatility periods → shorter RSI period to stay responsive. KST momentum confirms the direction: require KST crossing in the same direction as RSI for entry.

**Why not duplicate**: No prior RUN adapts the RSI period dynamically. This is distinct from all fixed-period RSI approaches. The volatility-adaptive period combined with KST confirmation is the distinct mechanism.

## Proposed Config Changes (config.rs)

```rust
// ── RUN375: RSI Adaptive Period with KST Momentum ────────────────────────────────
// adaptive_rsi_period = BASE_PERIOD * (current_ATR / avg_ATR)
// Higher ATR → longer period (less noise)
// rsi_adaptive = RSI(close, adaptive_period)
// kst_cross_up = kst crosses above kst_signal
// kst_cross_down = kst crosses below kst_signal
// LONG: rsi_adaptive < RSI_OVERSOLD AND kst_cross_up
// SHORT: rsi_adaptive > RSI_OVERBOUGHT AND kst_cross_down

pub const RSI_ADAPT_KST_ENABLED: bool = true;
pub const RSI_ADAPT_BASE_PERIOD: usize = 14;
pub const RSI_ADAPT_ATR_PERIOD: usize = 14;
pub const RSI_ADAPT_ATR_MA_PERIOD: usize = 100;
pub const RSI_ADAPT_RSI_OVERSOLD: f64 = 35.0;
pub const RSI_ADAPT_RSI_OVERBOUGHT: f64 = 65.0;
pub const RSI_ADAPT_KST_ROC1: usize = 8;
pub const RSI_ADAPT_KST_ROC2: usize = 16;
pub const RSI_ADAPT_KST_ROC3: usize = 24;
pub const RSI_ADAPT_KST_ROC4: usize = 32;
pub const RSI_ADAPT_KST_SIGNAL: usize = 8;
pub const RSI_ADAPT_SL: f64 = 0.005;
pub const RSI_ADAPT_TP: f64 = 0.004;
pub const RSI_ADAPT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run375_1_rsi_adapt_kst_backtest.py)
2. **Walk-forward** (run375_2_rsi_adapt_kst_wf.py)
3. **Combined** (run375_3_combined.py)

## Out-of-Sample Testing

- BASE_PERIOD sweep: 10 / 14 / 21
- ATR_MA_PERIOD sweep: 50 / 100 / 200
- RSI_OVERSOLD sweep: 30 / 35 / 40
- RSI_OVERBOUGHT sweep: 60 / 65 / 70
