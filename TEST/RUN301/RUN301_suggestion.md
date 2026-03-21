# RUN301 — Intraday Intensity Index: Volume-Weighted Close Position

## Hypothesis

**Mechanism**: Intraday Intensity = (2×close - high - low) / (high - low) × volume. This measures where the close trades relative to the high-low range. A high positive II value means close is near the high (buying pressure). A low negative value means close is near the low (selling pressure). Compute a smoothed II (EMA of II values) and look for divergences with price — II rising while price flat or falling = hidden bullish divergence.

**Why not duplicate**: RUN223 uses Volume Price Trend. RUN254 uses Volume Delta. RUN294 uses OBV divergence. II is distinct because it weights the close position within the bar's range — not just whether close > open, but WHERE in the range the close settles. More precise than binary up/down volume.

## Proposed Config Changes (config.rs)

```rust
// ── RUN301: Intraday Intensity Index ─────────────────────────────────────────
// ii = (2*close - high - low) / (high - low) * volume
// ii_smooth = EMA(II, period)
// LONG: ii_smooth crosses above threshold AND price > SMA(20)
// SHORT: ii_smooth crosses below -threshold AND price < SMA(20)
// Divergence: price makes new high but II doesn't = bearish divergence

pub const II_ENABLED: bool = true;
pub const II_PERIOD: usize = 14;             // EMA period for II smoothing
pub const II_THRESHOLD_LONG: f64 = 0.0;     // II must be positive for LONG
pub const II_THRESHOLD_SHORT: f64 = 0.0;    // II must be negative for SHORT
pub const II_ENTRIES_PER_WINDOW: usize = 1;  // entries per signal window
pub const II_SL: f64 = 0.005;
pub const II_TP: f64 = 0.004;
pub const II_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run301_1_ii_backtest.py)
2. **Walk-forward** (run301_2_ii_wf.py)
3. **Combined** (run301_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- THRESHOLD sweep: 0.0 / 0.1 / 0.2 (for smoothing activation)
