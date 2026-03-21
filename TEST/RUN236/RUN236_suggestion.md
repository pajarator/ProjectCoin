# RUN236 — ADX-MACD Confluence: Strong Trend + Momentum Confirmation

## Hypothesis

**Mechanism**: Combine ADX (trend strength) with MACD (momentum direction). A trade is only valid when BOTH indicators agree AND ADX shows strength. For LONG: MACD must be bullish AND ADX > 25 (strong trend). For SHORT: MACD bearish AND ADX > 25. When ADX < 20 (weak/no trend) → skip or exit. The confluence of trend + momentum produces higher-quality signals than either indicator alone.

**Why not duplicate**: No prior RUN combines ADX with MACD as a confluence filter. All prior ADX RUNs use ADX alone. All prior MACD RUNs use MACD alone. The confluence approach is fundamentally different — it requires *both* trend strength AND momentum direction simultaneously.

## Proposed Config Changes (config.rs)

```rust
// ── RUN236: ADX-MACD Confluence ───────────────────────────────────────────
// LONG: macd_bullish AND adx > 25
// SHORT: macd_bearish AND adx > 25
// NO TRADE: adx < 20 (weak trend)
// EXIT: adx drops below 20 OR opposite MACD signal

pub const ADX_MACD_ENABLED: bool = true;
pub const ADX_MACD_PERIOD: usize = 14;      // ADX period
pub const ADX_MACD_STRONG: f64 = 25.0;     // strong trend threshold
pub const ADX_MACD_WEAK: f64 = 20.0;       // no-trade threshold
pub const ADX_MACD_FAST: usize = 12;        // MACD fast
pub const ADX_MACD_SLOW: usize = 26;        // MACD slow
pub const ADX_MACD_SIGNAL: usize = 9;        // MACD signal
pub const ADX_MACD_SL: f64 = 0.005;
pub const ADX_MACD_TP: f64 = 0.004;
pub const ADX_MACD_MAX_HOLD: u32 = 72;
```

---

## Validation Method

1. **Historical backtest** (run236_1_adx_macd_backtest.py)
2. **Walk-forward** (run236_2_adx_macd_wf.py)
3. **Combined** (run236_3_combined.py)

## Out-of-Sample Testing

- ADX_PERIOD sweep: 10 / 14 / 21
- ADX_STRONG sweep: 20 / 25 / 30
- ADX_WEAK sweep: 15 / 20 / 25
