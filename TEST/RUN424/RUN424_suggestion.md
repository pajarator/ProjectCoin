# RUN424 — Money Flow Index with EMA Trend Alignment

## Hypothesis

**Mechanism**: Money Flow Index (MFI) is a volume-weighted RSI that measures buying and selling pressure through both price and volume. Unlike RSI which only uses price, MFI captures volume-backed momentum. Add EMA Trend Alignment as a filter: only take MFI signals when price is aligned with the EMA direction. This prevents buying in downtrends or selling in uptrends — only taking MFI extremes when the broader trend agrees.

**Why not duplicate**: RUN303 uses MFI Percentile Rank. RUN390 uses MFI with VWAP Mean Reversion Distance. This RUN specifically uses EMA Trend Alignment as the confirmation filter — requiring the broader trend to agree with the MFI signal direction, distinct from percentile ranking or VWAP distance mechanisms.

## Proposed Config Changes (config.rs)

```rust
// ── RUN424: Money Flow Index with EMA Trend Alignment ─────────────────────────────────
// mfi = 100 - (100 / (1 + money_flow_ratio))
// mfi_extreme: mfi < MFI_OVERSOLD or mfi > MFI_OVERBOUGHT
// ema_trend: close > EMA(close, period) = bullish, else bearish
// LONG: mfi < MFI_OVERSOLD AND close > EMA (oversold in uptrend)
// SHORT: mfi > MFI_OVERBOUGHT AND close < EMA (overbought in downtrend)

pub const MFI_EMA_ENABLED: bool = true;
pub const MFI_EMA_MFI_PERIOD: usize = 14;
pub const MFI_EMA_MFI_OVERSOLD: f64 = 20.0;
pub const MFI_EMA_MFI_OVERBOUGHT: f64 = 80.0;
pub const MFI_EMA_EMA_PERIOD: usize = 20;
pub const MFI_EMA_SL: f64 = 0.005;
pub const MFI_EMA_TP: f64 = 0.004;
pub const MFI_EMA_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run424_1_mfi_ema_backtest.py)
2. **Walk-forward** (run424_2_mfi_ema_wf.py)
3. **Combined** (run424_3_combined.py)

## Out-of-Sample Testing

- MFI_PERIOD sweep: 10 / 14 / 21
- MFI_OVERSOLD sweep: 15 / 20 / 25
- MFI_OVERBOUGHT sweep: 75 / 80 / 85
- EMA_PERIOD sweep: 15 / 20 / 30
