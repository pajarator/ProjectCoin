# RUN322 — Trend Resonance Factor: Multi-Timeframe Trend Alignment

## Hypothesis

**Mechanism**: A trend is most reliable when multiple timeframes agree. Define "trend up" as price above SMA on each timeframe. Compute a Trend Resonance Factor = count of timeframes where trend aligns (0-3). TRF = 3 (all 3 TF agree) → highest conviction entry. TRF = 2 → moderate conviction. TRF = 1 → low conviction, filtered out. This is a trend-following system, not mean-reversion — enter when all timeframes show the same direction.

**Why not duplicate**: RUN260 uses MTF MACD alignment. RUN63 uses BTC trend confirmation for regime entries. RUN246 uses multi-timeframe RSI. No RUN uses simple SMA-based trend alignment across 3 timeframes with a counting/scoring mechanism. The distinct mechanism is the resonance score (0-3) as a conviction filter.

## Proposed Config Changes (config.rs)

```rust
// ── RUN322: Trend Resonance Factor ────────────────────────────────────────────
// trend_up(tf) = close > SMA(close, period, tf)
// trend_down(tf) = close < SMA(close, period, tf)
// resonance = count of TFs where trend matches direction (max 3)
// LONG: resonance >= 2 AND all trending TFs are up
// SHORT: resonance >= 2 AND all trending TFs are down
// Exit: any TF trend flips

pub const TREND_RES_ENABLED: bool = true;
pub const TREND_RES_PERIOD: usize = 20;      // SMA period
pub const TREND_RES_TFS: usize = 3;          // number of timeframes (15m, 1h, 4h)
pub const TREND_RES_MIN: usize = 2;          // minimum TFs that must agree
pub const TREND_RES_SL: f64 = 0.005;
pub const TREND_RES_TP: f64 = 0.004;
pub const TREND_RES_MAX_HOLD: u32 = 72;
```

---

## Validation Method

1. **Historical backtest** (run322_1_trend_res_backtest.py)
2. **Walk-forward** (run322_2_trend_res_wf.py)
3. **Combined** (run322_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 15 / 20 / 30
- MIN sweep: 2 / 3
