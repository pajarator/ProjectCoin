# RUN246 — Multi-Timeframe RSI Confluence: Cross-Frame Overbought/Oversold

## Hypothesis

**Mechanism**: RSI on multiple timeframes (15m, 1h, 4h) should agree for high-confidence signals. When 15m RSI AND 1h RSI AND 4h RSI ALL show oversold (<35) → historically rare → very strong reversal probability. When all three show overbought (>65) → very strong bearish reversal. This is a "three screens" approach ensuring all major timeframes align.

**Why not duplicate**: No prior RUN uses multi-timeframe RSI confluence. All prior multi-timeframe RUNs use EMA crosses or ADX. RSI confluence across timeframes is distinct — it requires *all* timeframes to agree, making it a very high-confidence signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN246: Multi-Timeframe RSI Confluence ───────────────────────────────
// rsi_15m = RSI(close, 14) on 15m
// rsi_1h = RSI(close, 14) on 1h (approximated from 15m aggregation)
// rsi_4h = RSI(close, 14) on 4h (approximated from 15m aggregation)
// LONG: rsi_15m < 35 AND rsi_1h < 35 AND rsi_4h < 35
// SHORT: rsi_15m > 65 AND rsi_1h > 65 AND rsi_4h > 65
// For 1h RSI: aggregate 4 consecutive 15m bars
// For 4h RSI: aggregate 16 consecutive 15m bars

pub const MTF_RSI_ENABLED: bool = true;
pub const MTF_RSI_PERIOD: usize = 14;
pub const MTF_RSI_OVERSOLD: f64 = 35.0;     // oversold threshold
pub const MTF_RSI_OVERBOUGHT: f64 = 65.0;  // overbought threshold
pub const MTF_RSI_SL: f64 = 0.005;
pub const MTF_RSI_TP: f64 = 0.004;
pub const MTF_RSI_MAX_HOLD: u32 = 72;
```

---

## Validation Method

1. **Historical backtest** (run246_1_mtf_rsi_backtest.py)
2. **Walk-forward** (run246_2_mtf_rsi_wf.py)
3. **Combined** (run246_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- OVERSOLD sweep: 30 / 35 / 40
- OVERBOUGHT sweep: 60 / 65 / 70
