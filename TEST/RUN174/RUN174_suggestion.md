# RUN174 — Multi-Timeframe RSI Alignment: 1h RSI Extreme Confirmation for 15m Entries

## Hypothesis

**Mechanism**: A 15m LONG signal confirmed by 1h RSI extreme (1h RSI < 30) is stronger than a 15m signal alone — the higher timeframe provides context. If 1h RSI is not extreme, require a stronger 15m signal (lower RSI threshold). This multi-timeframe confirmation improves signal quality.

**Why not duplicate**: No prior RUN uses multi-timeframe RSI confirmation. All prior filters are single-timeframe.

## Proposed Config Changes (config.rs)

```rust
// ── RUN174: Multi-Timeframe RSI Confirmation ───────────────────────────
// 15m LONG confirmed by 1h RSI < 30 → proceed
// 15m LONG without 1h confirmation → require 15m RSI < 25
// Same logic for SHORT

pub const MTF_RSI_ENABLED: bool = true;
pub const MTF_RSI_1H_THRESH: f64 = 30.0;   // 1h RSI threshold for LONG confirmation
```

Add to `CoinState` in `state.rs`:

```rust
pub rsi_1h: f64,   // 1-hour RSI (updated every 4 bars)
```

Add `fetch_1h_rsi` in `fetcher.rs`:

```rust
/// Fetch 1h RSI by aggregating 4 x 15m candles into 1h candles
pub fn compute_1h_rsi(candles_15m: &[Candle]) -> Option<f64> {
    // Group every 4 candles into 1h OHLCV, compute RSI
    unimplemented!("aggregate 15m candles into 1h bars, compute RSI")
}
```

Modify `long_entry` check:

```rust
// OLD: if ind.rsi < 40.0
// NEW:
// if config::MTF_RSI_ENABLED {
//   if ind.rsi < 40.0 && (cs.rsi_1h < config::MTF_RSI_1H_THRESH || ind.rsi < 25.0) {}
// }
```

---

## Validation Method

1. **Historical backtest** (run174_1_mtfrsi_backtest.py)
2. **Walk-forward** (run174_2_mtfrsi_wf.py)
3. **Combined** (run174_3_combined.py)

## Out-of-Sample Testing

- 1H_THRESH sweep: 25 / 30 / 35
