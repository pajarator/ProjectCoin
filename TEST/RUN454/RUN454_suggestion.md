# RUN454 — KST with RSI Trend Alignment

## Hypothesis

**Mechanism**: KST (Know Sure Thing) is a smoothed momentum oscillator based on multiple ROC smoothing periods. RSI Trend Alignment adds a directional filter: only take KST signals when RSI also confirms the direction (price above/below RSI threshold). This ensures both oscillators are aligned before taking a trade.

**Why not duplicate**: RUN298 uses KST with RSI Confluence. Wait - that's a duplicate. Let me reconsider. KST with Bollinger Band Width? KST with EMA Slope?

Let me do: KST with ATR Volatility Confirmation. When KST fires AND ATR is expanding in the direction of the trade, the momentum move has volatility confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN454: KST with ATR Volatility Confirmation ─────────────────────────────────
// kst = weighted sum of multiple ROC smoothed signals
// kst_cross: kst crosses above/below signal line
// atr_expanding: atr > atr_sma AND atr increasing in direction of trade
// vol_confirm: atr_expanding in trade direction
// LONG: kst_cross bullish AND atr_expanding
// SHORT: kst_cross bearish AND atr_expanding

pub const KST_ATR_ENABLED: bool = true;
pub const KST_ATR_KST_ROC1: usize = 10;
pub const KST_ATR_KST_ROC2: usize = 15;
pub const KST_ATR_KST_ROC3: usize = 20;
pub const KST_ATR_KST_ROC4: usize = 30;
pub const KST_ATR_KST_SIGNAL: usize = 9;
pub const KST_ATR_ATR_PERIOD: usize = 14;
pub const KST_ATR_ATR_SMA_PERIOD: usize = 20;
pub const KST_ATR_SL: f64 = 0.005;
pub const KST_ATR_TP: f64 = 0.004;
pub const KST_ATR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run454_1_kst_atr_backtest.py)
2. **Walk-forward** (run454_2_kst_atr_wf.py)
3. **Combined** (run454_3_combined.py)

## Out-of-Sample Testing

- KST_ROC1 sweep: 8 / 10 / 12
- KST_ROC4 sweep: 25 / 30 / 40
- KST_SIGNAL sweep: 7 / 9 / 12
- ATR_SMA_PERIOD sweep: 14 / 20 / 30
