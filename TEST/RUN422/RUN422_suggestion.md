# RUN422 — Demand Index with SuperTrend Confirmation

## Hypothesis

**Mechanism**: The Demand Index combines price and volume to identify whether buyers or sellers are dominating the market. It oscillates around zero — positive values indicate buying pressure, negative values indicate selling pressure. A divergence between price and the Demand Index (price makes new high but DI doesn't confirm) often precedes reversals. SuperTrend provides trend confirmation: when Demand Index diverges AND SuperTrend flips direction, you have both money flow divergence AND trend confirmation working together.

**Why not duplicate**: RUN325 uses Demand Index Zero-Line Cross. RUN357 uses Elder Ray Index with ADX Filter. This RUN specifically uses Demand Index divergence (price-DI disagreement) with SuperTrend directional flip confirmation — the distinct mechanism is divergence detection between price and money flow confirmed by SuperTrend trend changes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN422: Demand Index with SuperTrend Confirmation ─────────────────────────────────
// demand_index = calculated based on price action and volume relationship
// di_divergence: price makes new high/low but DI doesn't confirm
// supertrend: atr-based trend direction and trailing stop
// supertrend_flip: trend changes from bullish to bearish or vice versa
// LONG: di_bullish_divergence (price low, DI higher low) AND supertrend bullish
// SHORT: di_bearish_divergence (price high, DI lower high) AND supertrend bearish

pub const DI_ST_ENABLED: bool = true;
pub const DI_ST_DI_PERIOD: usize = 20;
pub const DI_ST_ST_PERIOD: usize = 10;
pub const DI_ST_ST_MULT: f64 = 3.0;
pub const DI_ST_SL: f64 = 0.005;
pub const DI_ST_TP: f64 = 0.004;
pub const DI_ST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run422_1_di_st_backtest.py)
2. **Walk-forward** (run422_2_di_st_wf.py)
3. **Combined** (run422_3_combined.py)

## Out-of-Sample Testing

- DI_PERIOD sweep: 14 / 20 / 30
- ST_PERIOD sweep: 7 / 10 / 14
- ST_MULT sweep: 2.0 / 3.0 / 4.0
