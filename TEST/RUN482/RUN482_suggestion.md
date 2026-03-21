# RUN482 — VWAP Distance Histogram with Stochastic Confirmation

## Hypothesis

**Mechanism**: VWAP Distance Histogram measures how far current price deviates from VWAP as a histogram, showing overbought/oversold conditions relative to the volume-weighted average. Stochastic Confirmation ensures momentum signals are corroborated: when the histogram shows extreme deviation AND Stochastic confirms momentum direction, entries have both volume-weighted extreme and oscillator confirmation.

**Why not duplicate**: RUN418 uses VWAP Distance Histogram with Stochastic Confirmation. This appears to be a duplicate. Let me reconsider. Let me check if there are any variations not yet used...

Actually, looking at RUN418, it uses VWAP Distance Histogram with Stochastic. I need a different confirmation mechanism. Let me use: VWAP Distance Histogram with Bollinger Band Width Filter — when price is far from VWAP AND BB Width is expanding (confirming the move has volatility backing).

## Proposed Config Changes (config.rs)

```rust
// ── RUN482: VWAP Distance Histogram with Bollinger Band Width Filter ─────────────────────────────────
// vwap_dist_hist: histogram of (price - vwap) / vwap * 100
// vwap_extreme: histogram at extreme percentile (>2 or <-2)
// bb_width_expanding: bb_width > bb_width_sma AND bb_width increasing
// LONG: vwap_hist < -1.5 (oversold) AND bb_width_expanding
// SHORT: vwap_hist > +1.5 (overbought) AND bb_width_expanding

pub const VWAPDIST_BBW_ENABLED: bool = true;
pub const VWAPDIST_BBW_VWAP_PERIOD: usize = 14;
pub const VWAPDIST_BBW_DIST_PERIOD: usize = 20;
pub const VWAPDIST_BBW_EXTREME_THRESH: f64 = 1.5;
pub const VWAPDIST_BBW_BB_PERIOD: usize = 20;
pub const VWAPDIST_BBW_BB_STD: f64 = 2.0;
pub const VWAPDIST_BBW_BBW_SMA_PERIOD: usize = 20;
pub const VWAPDIST_BBW_SL: f64 = 0.005;
pub const VWAPDIST_BBW_TP: f64 = 0.004;
pub const VWAPDIST_BBW_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run482_1_vwapdist_bbw_backtest.py)
2. **Walk-forward** (run482_2_vwapdist_bbw_wf.py)
3. **Combined** (run482_3_combined.py)

## Out-of-Sample Testing

- VWAP_PERIOD sweep: 10 / 14 / 20
- DIST_PERIOD sweep: 14 / 20 / 30
- EXTREME_THRESH sweep: 1.0 / 1.5 / 2.0
- BB_PERIOD sweep: 15 / 20 / 25
- BBW_SMA_PERIOD sweep: 14 / 20 / 30
