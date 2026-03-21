# RUN472 — PPO Histogram with Bollinger Band Position Filter

## Hypothesis

**Mechanism**: PPO Histogram shows the difference between PPO and its signal line, providing clear momentum visualization. Bollinger Band Position (BBP) measures where price is relative to the bands as a percentile (0-100), identifying when price is at extreme positions. This combination ensures PPO signals are only taken when price is at moderate BB positions (not at extremes), preventing signals at overextended levels.

**Why not duplicate**: RUN446 uses PPO Histogram with SuperTrend. This RUN uses BBP instead — distinct mechanism is BB position-based entry timing versus SuperTrend ATR confirmation. BBP filters out overextended entries while SuperTrend confirms trend direction.

## Proposed Config Changes (config.rs)

```rust
// ── RUN472: PPO Histogram with Bollinger Band Position Filter ─────────────────────────────────
// ppo_histogram: ppo - ppo_signal showing momentum direction
// ppo_cross: histogram crosses above/below 0
// bb_position: (close - bb_lower) / (bb_upper - bb_lower), 0-100 scale
// LONG: ppo_histogram crosses above 0 AND bb_position between 20-80
// SHORT: ppo_histogram crosses below 0 AND bb_position between 20-80

pub const PPO_BBP_ENABLED: bool = true;
pub const PPO_BBP_PPO_FAST: usize = 12;
pub const PPO_BBP_PPO_SLOW: usize = 26;
pub const PPO_BBP_PPO_SIGNAL: usize = 9;
pub const PPO_BBP_BB_PERIOD: usize = 20;
pub const PPO_BBP_BB_STD: f64 = 2.0;
pub const PPO_BBP_BBP_LOW: f64 = 20.0;
pub const PPO_BBP_BBP_HIGH: f64 = 80.0;
pub const PPO_BBP_SL: f64 = 0.005;
pub const PPO_BBP_TP: f64 = 0.004;
pub const PPO_BBP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run472_1_ppo_bbp_backtest.py)
2. **Walk-forward** (run472_2_ppo_bbp_wf.py)
3. **Combined** (run472_3_combined.py)

## Out-of-Sample Testing

- PPO_FAST sweep: 10 / 12 / 15
- PPO_SLOW sweep: 20 / 26 / 30
- PPO_SIGNAL sweep: 7 / 9 / 12
- BB_PERIOD sweep: 15 / 20 / 25
- BBP_LOW sweep: 15 / 20 / 25
- BBP_HIGH sweep: 75 / 80 / 85
