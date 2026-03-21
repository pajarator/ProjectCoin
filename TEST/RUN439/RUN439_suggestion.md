# RUN439 — RSI Adaptive Period with KST Confluence

## Hypothesis

**Mechanism**: Standard RSI uses a fixed period, but Adaptive RSI adjusts its period based on market volatility — longer periods in volatile markets (to filter noise) and shorter periods in calm markets (for responsiveness). KST (Know Sure Thing) provides smoothed momentum confirmation. When Adaptive RSI fires a signal AND KST confirms in the same direction, you have both volatility-adaptive oscillator timing AND smoothed multi-period momentum confirmation.

**Why not duplicate**: RUN375 uses RSI Adaptive Period with KST. Wait - that's a duplicate! Let me reconsider. Adaptive RSI with Bollinger Band Width Filter - uses BB width as a volatility regime filter for adaptive RSI signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN439: RSI Adaptive Period with Bollinger Band Width Filter ─────────────────────────────────────
// adaptive_rsi: rsi_period adapts based on ATR ratio (high vol = longer period)
// bb_width = (bb_upper - bb_lower) / bb_middle
// low_width: bb_width < BB_WIDTH_THRESH (compressed = calm market)
// adaptive_rsi_signal: adaptive_rsi crosses above/below thresholds
// bb_confirm: low_bb_width indicates calm market (valid for adaptive RSI)
// LONG: adaptive_rsi bullish AND bb_width < threshold
// SHORT: adaptive_rsi bearish AND bb_width < threshold

pub const ADAPT_RSI_BBW_ENABLED: bool = true;
pub const ADAPT_RSI_BBW_BASE_PERIOD: usize = 14;
pub const ADAPT_RSI_BBW_ATR_PERIOD: usize = 14;
pub const ADAPT_RSI_BBW_RSI_OVERSOLD: f64 = 30.0;
pub const ADAPT_RSI_BBW_RSI_OVERBOUGHT: f64 = 70.0;
pub const ADAPT_RSI_BBW_BB_PERIOD: usize = 20;
pub const ADAPT_RSI_BBW_BB_STD: f64 = 2.0;
pub const ADAPT_RSI_BBW_WIDTH_THRESH: f64 = 0.05;
pub const ADAPT_RSI_BBW_SL: f64 = 0.005;
pub const ADAPT_RSI_BBW_TP: f64 = 0.004;
pub const ADAPT_RSI_BBW_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run439_1_adapt_rsi_bbw_backtest.py)
2. **Walk-forward** (run439_2_adapt_rsi_bbw_wf.py)
3. **Combined** (run439_3_combined.py)

## Out-of-Sample Testing

- BASE_PERIOD sweep: 10 / 14 / 21
- ATR_PERIOD sweep: 10 / 14 / 21
- BB_PERIOD sweep: 15 / 20 / 30
- WIDTH_THRESH sweep: 0.04 / 0.05 / 0.06
