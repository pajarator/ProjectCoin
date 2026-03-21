# RUN170 — MACD Histogram Slope Reversal: 3-Bar Momentum Confirmation for Scalp Entries

## Hypothesis

**Mechanism**: MACD histogram slope (3-bar rate of change) tells you whether momentum is accelerating or decelerating. When MACD histogram is rising (slope > 0) AND price is near the entry level, momentum is building in your direction — confirm entry. When histogram is falling (slope < 0), momentum is fading — block entry.

**Why not duplicate**: No prior RUN uses MACD histogram slope. RUN111 (MACD Histogram Slope Exit) was proposed but unexecuted.

## Proposed Config Changes (config.rs)

```rust
// ── RUN170: MACD Histogram Slope Confirmation ──────────────────────────
// hist_slope = (macd_hist - macd_hist_3bars_ago) / 3
// Slope > 0 AND direction matches → confirm entry
// Slope < 0 AND direction matches → block entry

pub const MACD_SLOPE_ENABLED: bool = true;
pub const MACD_SLOPE_LOOKBACK: usize = 3;    // bars to measure slope
pub const MACD_SLOPE_THRESH: f64 = 0.0001;   // minimum slope magnitude
```

Add to `CoinState` in `state.rs`:

```rust
pub macd_hist_rolling: Vec<f64>,   // rolling MACD histogram history
```

Add in `indicators.rs` (already has MACD):

```rust
pub macd_hist_slope: f64,   // (hist - hist_3ago) / 3
```

Add in `engine.rs`:

```rust
fn macd_slope_confirm(cs: &CoinState, proposed_dir: Direction) -> bool {
    if !config::MACD_SLOPE_ENABLED { return true; }
    let slope = cs.ind_15m.as_ref()?.macd_hist_slope;
    if slope.is_nan() { return true; }
    match proposed_dir {
        Direction::Long if slope < -config::MACD_SLOPE_THRESH => return false,
        Direction::Short if slope > config::MACD_SLOPE_THRESH => return false,
        _ => {}
    }
    true
}
```

---

## Validation Method

1. **Historical backtest** (run170_1_macdslope_backtest.py)
2. **Walk-forward** (run170_2_macdslope_wf.py)
3. **Combined** (run170_3_combined.py)

## Out-of-Sample Testing

- LOOKBACK sweep: 2 / 3 / 5 bars
- THRESH sweep: 0.00005 / 0.0001 / 0.0002
