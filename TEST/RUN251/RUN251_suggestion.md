# RUN251 — ICT Smart Money Concepts: Fair Value Gap Detection and Order Block Zones

## Hypothesis

**Mechanism**: ICT (Inner Circle Trader) concepts: (1) Fair Value Gap = price moved aggressively in one direction leaving a "gap" in the market — price tends to return to fill the FVG before continuing. (2) Order Block = the last candle before a strong directional move — institutional players were accumulating/distributing there. When price returns to an FVG or order block zone → entry with high probability.

**Why not duplicate**: No prior RUN uses ICT concepts (FVG, Order Blocks). All prior support/resistance RUNs use pivot points, Fibonacci, or VWAP. FVG and Order Blocks are specific ICT concepts that identify institutional activity zones.

## Proposed Config Changes (config.rs)

```rust
// ── RUN251: ICT Smart Money Concepts ───────────────────────────────────────
// FVG = 3-bar pattern: bar2's low > bar1's high + small gap OR bar2's high < bar1's low + small gap
// FVG zones act as magnetic price levels (fill before continuing)
// Order Block = candle before >2% move in 15m
// LONG: price returns to FVG zone (bullish FVG) OR order block zone
// SHORT: price returns to FVG zone (bearish FVG) OR order block zone

pub const ICT_ENABLED: bool = true;
pub const ICT_FVG_GAP: f64 = 0.0005;       // minimum gap for FVG
pub const ICT_OB_MOVE_THRESH: f64 = 0.02;   // >2% move defines OB
pub const ICT_SL: f64 = 0.005;
pub const ICT_TP: f64 = 0.004;
pub const ICT_MAX_HOLD: u32 = 36;
```

---

## Validation Method

1. **Historical backtest** (run251_1_ict_backtest.py)
2. **Walk-forward** (run251_2_ict_wf.py)
3. **Combined** (run251_3_combined.py)

## Out-of-Sample Testing

- FVG_GAP sweep: 0.0003 / 0.0005 / 0.001
- OB_MOVE_THRESH sweep: 0.015 / 0.02 / 0.025
