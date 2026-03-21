# RUN384 — Ulcer Index Mean Reversion with SuperTrend Confirmation

## Hypothesis

**Mechanism**: The Ulcer Index measures downside volatility — how long price stays below its recent high. Unlike standard volatility measures, Ulcer Index focuses specifically on drawdown pain. When the Ulcer Index reaches high values, volatility is concentrated in downward moves. Mean reversion strategies work well in low Ulcer environments (calm markets). Use Ulcer Index as a regime filter: only take mean reversion signals when Ulcer Index is below a threshold (calm market), and use SuperTrend as the actual entry direction confirmation. When Ulcer is low AND SuperTrend flips, the move is a genuine trend change rather than volatile noise.

**Why not duplicate**: RUN309 uses Ulcer Index Compression. This RUN uses Ulcer Index as a regime filter combined with SuperTrend confirmation — a distinctly different use case. Ulcer as volatility-filter + SuperTrend as entry director is different from Ulcer compression-based entry signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN384: Ulcer Index Mean Reversion with SuperTrend Confirmation ───────────
// ulcer_index = 100 * sqrt(sum((price - highest_price)^2 / period) / period)
// lower_ulcer: ulcer < ULCER_THRESH means calm market (valid for mean reversion)
// supertrend = ATR-based trend line with multiplier
// LONG: ulcer < ULCER_THRESH AND supertrend flips to bullish
// SHORT: ulcer < ULCER_THRESH AND supertrend flips to bearish

pub const ULCER_ST_ENABLED: bool = true;
pub const ULCER_ST_ULCER_PERIOD: usize = 14;
pub const ULCER_ST_THRESH: f64 = 5.0;   // below this = calm market
pub const ULCER_ST_ST_PERIOD: usize = 10;
pub const ULCER_ST_ST_MULT: f64 = 3.0;
pub const ULCER_ST_SL: f64 = 0.005;
pub const ULCER_ST_TP: f64 = 0.004;
pub const ULCER_ST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run384_1_ulcer_st_backtest.py)
2. **Walk-forward** (run384_2_ulcer_st_wf.py)
3. **Combined** (run384_3_combined.py)

## Out-of-Sample Testing

- ULCER_PERIOD sweep: 10 / 14 / 21
- ULCER_THRESH sweep: 4.0 / 5.0 / 6.0
- ST_PERIOD sweep: 7 / 10 / 14
- ST_MULT sweep: 2.0 / 3.0 / 4.0
