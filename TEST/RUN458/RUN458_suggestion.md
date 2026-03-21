# RUN458 — ATR Channel Breakout with RSI Filter

## Hypothesis

**Mechanism**: ATR Channel Breakout uses Average True Range to create dynamic support/resistance channels around price. When price closes beyond the ATR channel boundary, it signals a volatility expansion and potential trend continuation. The RSI Filter ensures breakouts are only taken when RSI is in neutral territory (not overbought/oversold), preventing false breakouts during exhausted moves.

**Why not duplicate**: RUN450 uses Opening Range Gap with VWAP Distance. This RUN uses ATR Channels — a distinct volatility-based channel that adapts to market volatility rather than fixed time ranges. The distinct mechanism is ATR-adaptive channel boundaries that expand/contract with volatility, filtered by RSI neutrality to avoid breakout traps at extreme levels.

## Proposed Config Changes (config.rs)

```rust
// ── RUN458: ATR Channel Breakout with RSI Filter ─────────────────────────────────
// atr_channel: upper = close + MULT * ATR, lower = close - MULT * ATR
// atr_channel_breakout: close crosses above upper (bull) or below lower (bear)
// rsi_filter: rsi in neutral zone (not extreme)
// LONG: close crosses above atr_upper AND rsi between 35-65
// SHORT: close crosses below atr_lower AND rsi between 35-65

pub const ATR_CH_RSI_ENABLED: bool = true;
pub const ATR_CH_RSI_ATR_PERIOD: usize = 14;
pub const ATR_CH_RSI_CHANNEL_MULT: f64 = 2.0;
pub const ATR_CH_RSI_RSI_PERIOD: usize = 14;
pub const ATR_CH_RSI_RSI_LOW: f64 = 35.0;
pub const ATR_CH_RSI_RSI_HIGH: f64 = 65.0;
pub const ATR_CH_RSI_SL: f64 = 0.005;
pub const ATR_CH_RSI_TP: f64 = 0.004;
pub const ATR_CH_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run458_1_atrch_rsi_backtest.py)
2. **Walk-forward** (run458_2_atrch_rsi_wf.py)
3. **Combined** (run458_3_combined.py)

## Out-of-Sample Testing

- ATR_PERIOD sweep: 10 / 14 / 20
- CHANNEL_MULT sweep: 1.5 / 2.0 / 2.5 / 3.0
- RSI_LOW sweep: 30 / 35 / 40
- RSI_HIGH sweep: 60 / 65 / 70
