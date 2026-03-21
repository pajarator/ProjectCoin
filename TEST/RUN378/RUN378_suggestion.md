# RUN378 — Price Structure MTF Confirmation: Trendline/Break Confirmation Across Timeframes

## Hypothesis

**Mechanism**: A break of a trendline or key level on multiple timeframes simultaneously is a much stronger signal than a single-timeframe break. If 15m price breaks a key level AND 1h also confirms the break, the signal has multi-timeframe institutional backing. The 1h timeframe provides the structural context, while the 15m provides the entry timing.

**Why not duplicate**: No prior RUN uses multi-timeframe structural confirmation for trendline/breakout levels. RUN260 uses MTF MACD alignment. This RUN specifically uses the concept of structural breaks (trendlines, key levels) confirmed across timeframes — the distinct mechanism is structural-level MTF confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN378: Price Structure MTF Confirmation ─────────────────────────────────
// key_level = swing high/low or trendline on 1h timeframe
// mtf_break = 15m price crosses above/below key_level AND 1h confirms
// 1h confirm: close on 1h candle is also beyond the key_level
// LONG: 15m breaks above key_level AND 1h candle closes above
// SHORT: 15m breaks below key_level AND 1h candle closes below

pub const STRUCT_MTF_ENABLED: bool = true;
pub const STRUCT_MTF_1H_LOOKBACK: usize = 20;  // lookback for swing H/L on 1h
pub const STRUCT_MTF_CONFIRM_BARS: u32 = 2;     // 1h must confirm within N bars
pub const STRUCT_MTF_SL: f64 = 0.005;
pub const STRUCT_MTF_TP: f64 = 0.004;
pub const STRUCT_MTF_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run378_1_struct_mtf_backtest.py)
2. **Walk-forward** (run378_2_struct_mtf_wf.py)
3. **Combined** (run378_3_combined.py)

## Out-of-Sample Testing

- 1H_LOOKBACK sweep: 14 / 20 / 30
- CONFIRM_BARS sweep: 1 / 2 / 4
