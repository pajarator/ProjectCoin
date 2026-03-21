# RUN419 — Price Ladder RSI Pullback with Volume Surge Confirmation

## Hypothesis

**Mechanism**: The Price Ladder concept tracks acceptance at discrete price levels. When price is rejected multiple times at a ladder level (resistance), it eventually breaks through. After a ladder break, price often pulls back to retest the broken level (now support). RSI Pullback confirms the retest is at an oversold reading. Volume Surge confirms institutional participation in the original breakout. When all three align (pullback to broken level + RSI oversold + volume surge), it's a high-probability long entry.

**Why not duplicate**: RUN310 uses Price Ladder Acceptance. RUN341 uses Price Ladder with RSI Band. This RUN specifically uses the pullback/retest pattern after ladder break with RSI pullback AND volume surge confirmation — distinct from acceptance-only ladder signals or ladder with RSI band (which doesn't use the retest-after-breakout mechanism).

## Proposed Config Changes (config.rs)

```rust
// ── RUN419: Price Ladder RSI Pullback with Volume Surge Confirmation ─────────────────────────────
// price_ladder: track rejections at discrete levels
// ladder_break: price breaks through rejection level
// pullback_retest: price returns to broken level as support
// rsi_pullback: rsi < RSI_PULLBACK_THRESH during retest
// volume_surge: volume > VOL_SURGE_MULT * SMA(volume) confirming original breakout
// LONG: pullback_retest to broken level AND rsi < threshold AND volume surge

pub const LADDER_RSI_VOL_ENABLED: bool = true;
pub const LADDER_RSI_VOL_PERIOD: usize = 20;      // ladder price increment period
pub const LADDER_RSI_VOL_REJECTIONS: u32 = 2;    // rejections before break confirmed
pub const LADDER_RSI_VOL_RSI_PERIOD: usize = 14;
pub const LADDER_RSI_VOL_RSI_PULLBACK: f64 = 35.0;
pub const LADDER_RSI_VOL_VOL_PERIOD: usize = 20;
pub const LADDER_RSI_VOL_SURGE_MULT: f64 = 2.0;
pub const LADDER_RSI_VOL_SL: f64 = 0.005;
pub const LADDER_RSI_VOL_TP: f64 = 0.004;
pub const LADDER_RSI_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run419_1_ladder_rsi_vol_backtest.py)
2. **Walk-forward** (run419_2_ladder_rsi_vol_wf.py)
3. **Combined** (run419_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 15 / 20 / 30
- REJECTIONS sweep: 2 / 3 / 4
- RSI_PULLBACK sweep: 30 / 35 / 40
- SURGE_MULT sweep: 1.5 / 2.0 / 2.5
