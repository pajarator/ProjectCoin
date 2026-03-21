# RUN294 — On Balance Volume Divergence: OBV Trend vs Price Trend

## Hypothesis

**Mechanism**: OBV should confirm price trend. If price makes higher highs but OBV makes lower highs → bearish divergence (distribution). If price makes lower lows but OBV makes higher lows → bullish divergence (accumulation). OBV divergence precedes price reversals.

**Why not duplicate**: RUN223 uses Volume Price Trend. RUN254 uses Volume Delta. OBV divergence is distinct because it compares OBV (cumulative volume direction) to price direction — a different data stream than VPT.

## Proposed Config Changes (config.rs)

```rust
// ── RUN294: OBV Divergence ──────────────────────────────────────────────
// obv = cumulative volume (add if up, subtract if down)
// LONG divergence: price.lower_low AND obv.higher_low
// SHORT divergence: price.higher_high AND obv.lower_high

pub const OBV_DIV_ENABLED: bool = true;
pub const OBV_DIV_PERIOD: usize = 20;        // swing detection lookback
pub const OBV_DIV_SL: f64 = 0.005;
pub const OBV_DIV_TP: f64 = 0.004;
pub const OBV_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run294_1_obv_div_backtest.py)
2. **Walk-forward** (run294_2_obv_div_wf.py)
3. **Combined** (run294_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
