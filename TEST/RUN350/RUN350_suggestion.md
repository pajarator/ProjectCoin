# RUN350 — Opening Range Breakout with Volume Confirmation

## Hypothesis

**Mechanism**: The opening range (first N bars of the trading session) establishes the market's initial balance. A break of the opening range high/low with volume confirmation indicates the market has chosen a direction. The opening range acts as a support/resistance boundary. Volume must be above average to confirm the breakout is genuine — low-volume breaks are prone to reversal.

**Why not duplicate**: RUN229 uses Opening Range Breakout. RUN138 uses Opening Range Breakout as a filter. This RUN specifically combines the ORB with volume confirmation — the volume filter on the breakout is the distinct mechanism that differentiates it from basic ORB.

## Proposed Config Changes (config.rs)

```rust
// ── RUN350: Opening Range Breakout with Volume Confirmation ─────────────────────────────
// opening_range_high = highest(high, ORB_BARS)
// opening_range_low = lowest(low, ORB_BARS)
// ors_breakout_up = close crosses above opening_range_high AND volume > avg_vol * VOL_CONFIRM
// ors_breakout_down = close crosses below opening_range_low AND volume > avg_vol * VOL_CONFIRM
// Range: defined by first ORB_BARS of the session (e.g., first 30 min = 2 bars at 15m)

pub const ORB_VOL_ENABLED: bool = true;
pub const ORB_VOL_BARS: u32 = 2;            // opening range in bars (2 × 15m = 30 min)
pub const ORB_VOL_VOL_CONFIRM: f64 = 1.5;  // volume must exceed 1.5x avg
pub const ORB_VOL_SL: f64 = 0.005;
pub const ORB_VOL_TP: f64 = 0.004;
pub const ORB_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run350_1_orb_vol_backtest.py)
2. **Walk-forward** (run350_2_orb_vol_wf.py)
3. **Combined** (run350_3_combined.py)

## Out-of-Sample Testing

- BARS sweep: 1 / 2 / 4
- VOL_CONFIRM sweep: 1.2 / 1.5 / 2.0
