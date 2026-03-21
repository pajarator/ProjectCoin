# RUN485 — RSI Gap Momentum with Bollinger Band Position

## Hypothesis

**Mechanism**: RSI Gap Momentum identifies momentum gaps in RSI where RSI opens above/below the prior day's range, signaling potential strength. Bollinger Band Position measures where price sits relative to the bands. When RSI gaps in a direction AND price is at a moderate BB position (not at extremes), the momentum gap has room to run. Extreme BB positions indicate overextension that may limit the momentum move.

**Why not duplicate**: RUN426 uses RSI Gap Momentum with Volume Confirmation. This RUN uses Bollinger Band Position instead — distinct mechanism is BB position as a spatial filter versus volume confirmation. BB Position prevents entries at overextended price levels.

## Proposed Config Changes (config.rs)

```rust
// ── RUN485: RSI Gap Momentum with Bollinger Band Position ─────────────────────────────────
// rsi_gap: rsi opens above/below prior rsi range
// rsi_gap_confirm: rsi gap direction matches price direction
// bb_position: (close - bb_lower) / (bb_upper - bb_lower) percentile
// LONG: rsi_gap bullish AND bb_position between 25-75
// SHORT: rsi_gap bearish AND bb_position between 25-75

pub const RSIGAP_BBP_ENABLED: bool = true;
pub const RSIGAP_BBP_RSI_PERIOD: usize = 14;
pub const RSIGAP_BBP_RSI_GAP_THRESH: f64 = 5.0;
pub const RSIGAP_BBP_BB_PERIOD: usize = 20;
pub const RSIGAP_BBP_BB_STD: f64 = 2.0;
pub const RSIGAP_BBP_BBP_LOW: f64 = 25.0;
pub const RSIGAP_BBP_BBP_HIGH: f64 = 75.0;
pub const RSIGAP_BBP_SL: f64 = 0.005;
pub const RSIGAP_BBP_TP: f64 = 0.004;
pub const RSIGAP_BBP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run485_1_rsigap_bbp_backtest.py)
2. **Walk-forward** (run485_2_rsigap_bbp_wf.py)
3. **Combined** (run485_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 20
- RSI_GAP_THRESH sweep: 3 / 5 / 7
- BB_PERIOD sweep: 15 / 20 / 25
- BBP_LOW sweep: 20 / 25 / 30
- BBP_HIGH sweep: 70 / 75 / 80
