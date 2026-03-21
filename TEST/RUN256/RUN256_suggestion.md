# RUN256 — Supply/Demand Zone Break: Institutional Range Breakout

## Hypothesis

**Mechanism**: Supply/Demand zones form when price consolidates in a range with high volume (institutional accumulation/distribution). When price breaks out of the range with volume > 2× the range's average volume → the zone is being "discovered" and price will continue in the breakout direction. Strong zones (multiple tests) create stronger breakouts.

**Why not duplicate**: No prior RUN uses supply/demand zone detection. All prior range breakout RUNs use Donchian or price channel. Supply/demand zones are distinct because they identify *institutional activity zones* based on volume during consolidation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN256: Supply/Demand Zone Break ─────────────────────────────────────
// Range = high - low < ATR × 1.5 (consolidation)
// Range volume > avg_volume × 1.5 (institutional interest)
// Zone tests = number of times price touched range edges
// LONG: price breaks above range_high AND volume > range_avg_vol × 2
// SHORT: price breaks below range_low AND volume > range_avg_vol × 2

pub const SD_ZONE_ENABLED: bool = true;
pub const SD_ZONE_ATR_MULT: f64 = 1.5;     // range must be < ATR × this
pub const SD_ZONE_VOL_MULT: f64 = 2.0;       // breakout volume must exceed × this
pub const SD_ZONE_TESTS_MIN: u32 = 2;        // minimum zone tests for valid zone
pub const SD_ZONE_SL: f64 = 0.005;
pub const SD_ZONE_TP: f64 = 0.004;
pub const SD_ZONE_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run256_1_sd_zone_backtest.py)
2. **Walk-forward** (run256_2_sd_zone_wf.py)
3. **Combined** (run256_3_combined.py)

## Out-of-Sample Testing

- ATR_MULT sweep: 1.0 / 1.5 / 2.0
- VOL_MULT sweep: 1.5 / 2.0 / 2.5
- TESTS_MIN sweep: 1 / 2 / 3
