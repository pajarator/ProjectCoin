# RUN426 — RSI Gap Momentum with Volume Confirmation

## Hypothesis

**Mechanism**: RSI Gap Momentum identifies when RSI makes a significant jump (gap) from one bar to the next — this indicates sudden momentum acceleration beyond normal smoothing. A bullish RSI gap occurs when RSI jumps from below to above a threshold in a single bar. Volume Confirmation adds institutional backing: when RSI gaps AND volume surges above its moving average, the momentum gap has volume-backed conviction and is more likely to continue.

**Why not duplicate**: RUN363 uses RSI Gap Momentum standalone. This RUN specifically adds Volume Confirmation as a secondary filter — the distinct mechanism is using volume surge to confirm RSI momentum gaps, distinguishing between genuine momentum acceleration and noise.

## Proposed Config Changes (config.rs)

```rust
// ── RUN426: RSI Gap Momentum with Volume Confirmation ─────────────────────────────────
// rsi_gap: rsi jumps more than GAP_THRESH points in a single bar
// rsi_gap_up: rsi jumps above RSI_THRESH in one bar
// rsi_gap_down: rsi drops below RSI_THRESH in one bar
// volume_confirmation: volume > VOL_MULT * SMA(volume, period)
// LONG: rsi_gap_up AND volume confirmation
// SHORT: rsi_gap_down AND volume confirmation

pub const RSI_GAP_VOL_ENABLED: bool = true;
pub const RSI_GAP_VOL_RSI_PERIOD: usize = 14;
pub const RSI_GAP_VOL_RSI_THRESH: f64 = 50.0;  // threshold for gap direction
pub const RSI_GAP_VOL_GAP_THRESH: f64 = 15.0;  // minimum RSI jump for gap
pub const RSI_GAP_VOL_VOL_PERIOD: usize = 20;
pub const RSI_GAP_VOL_VOL_MULT: f64 = 1.5;    // volume must be 1.5x average
pub const RSI_GAP_VOL_SL: f64 = 0.005;
pub const RSI_GAP_VOL_TP: f64 = 0.004;
pub const RSI_GAP_VOL_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run426_1_rsi_gap_vol_backtest.py)
2. **Walk-forward** (run426_2_rsi_gap_vol_wf.py)
3. **Combined** (run426_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- RSI_THRESH sweep: 45 / 50 / 55
- GAP_THRESH sweep: 10 / 15 / 20
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
