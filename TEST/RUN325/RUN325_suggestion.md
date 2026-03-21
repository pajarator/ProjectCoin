# RUN325 — Demand Index Zero-Line Cross: Accumulation vs Distribution

## Hypothesis

**Mechanism**: Demand Index = a complex volume-weighted indicator that compares today's volume and price change to yesterdays. When DI is positive and rising → accumulation (buyers are more aggressive than sellers). When DI is negative and falling → distribution. DI crossing above 0 from below = shift from supply to demand dominance → LONG. DI crossing below 0 from above = shift from demand to supply dominance → SHORT.

**Why not duplicate**: No prior RUN uses Demand Index. This is distinct from OBV (RUN294), MFI (RUN187), A/D line (RUN123), VWAP (various), and Volume Delta (RUN254). DI is a more sophisticated composite that incorporates both the price change sign and relative volume, making it a forward-looking measure of supply/demand balance.

## Proposed Config Changes (config.rs)

```rust
// ── RUN325: Demand Index Zero-Line Cross ───────────────────────────────────────
// demand_index = composite of volume × price_change direction × relative strength
// di_smooth = EMA(di, period)
// LONG: di_smooth crosses above 0
// SHORT: di_smooth crosses below 0
// Optional: require di_smooth to be rising/falling for N bars before entry

pub const DEMAND_IDX_ENABLED: bool = true;
pub const DEMAND_IDX_PERIOD: usize = 20;
pub const DEMAND_IDX_CONFIRM_BARS: u32 = 2;  // require N bars direction confirmation
pub const DEMAND_IDX_SL: f64 = 0.005;
pub const DEMAND_IDX_TP: f64 = 0.004;
pub const DEMAND_IDX_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run325_1_demand_idx_backtest.py)
2. **Walk-forward** (run325_2_demand_idx_wf.py)
3. **Combined** (run325_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- CONFIRM_BARS sweep: 1 / 2 / 3
