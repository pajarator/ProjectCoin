# RUN450 — Opening Range Gap with VWAP Distance Confirmation

## Hypothesis

**Mechanism**: Opening Range Gap identifies when price gaps above or below the opening range (high/low of first N bars of the session). Such gaps often represent overnight news or sentiment that gets immediately priced in. VWAP Distance measures how far price has moved from the volume-weighted average. When a gap occurs AND VWAP Distance is significant (price far from VWAP), the gap has conviction and is more likely to continue or fill in the direction of the gap with VWAP acting as a magnet.

**Why not duplicate**: RUN350 uses Opening Range Breakout with Volume. This RUN specifically uses Opening Range Gap (distinct from breakout) with VWAP Distance — the distinct mechanism is using gap direction with VWAP distance as a magnet/attraction target.

## Proposed Config Changes (config.rs)

```rust
// ── RUN450: Opening Range Gap with VWAP Distance Confirmation ─────────────────────────────────────
// opening_range = high/low of first OR_PERIOD bars of session
// gap_up: price opens above opening_range high
// gap_down: price opens below opening_range low
// vwap_distance = (price - vwap) / vwap
// significant_distance: |vwap_distance| > VWAP_DIST_THRESH
// LONG: gap_up AND vwap_distance > VWAP_DIST_THRESH (price extended above VWAP)
// SHORT: gap_down AND vwap_distance < -VWAP_DIST_THRESH (price extended below VWAP)

pub const OR_GAP_VWAP_ENABLED: bool = true;
pub const OR_GAP_VWAP_OR_PERIOD: usize = 5;       // opening range bars
pub const OR_GAP_VWAP_VWAP_PERIOD: usize = 20;
pub const OR_GAP_VWAP_DIST_THRESH: f64 = 0.015;   // 1.5% from VWAP
pub const OR_GAP_VWAP_SL: f64 = 0.005;
pub const OR_GAP_VWAP_TP: f64 = 0.004;
pub const OR_GAP_VWAP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run450_1_or_gap_vwap_backtest.py)
2. **Walk-forward** (run450_2_or_gap_vwap_wf.py)
3. **Combined** (run450_3_combined.py)

## Out-of-Sample Testing

- OR_PERIOD sweep: 3 / 5 / 7
- VWAP_PERIOD sweep: 14 / 20 / 30
- DIST_THRESH sweep: 0.01 / 0.015 / 0.02
