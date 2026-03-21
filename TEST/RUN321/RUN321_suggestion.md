# RUN321 — Balance of Power Oscillator: Midpoint Trend Authority

## Hypothesis

**Mechanism**: Balance of Power = (close - open) / (high - low). This measures where the close is relative to the midpoint of the high-low range, normalized by the range width. E.g., close exactly at midpoint = BOP 0. Close at high = BOP +1. Close at low = BOP -1. Smooth with EMA and use crossovers as signals. BOP crossing above its signal line = buyers gaining authority. BOP crossing below = sellers gaining authority.

**Why not duplicate**: No prior RUN uses Balance of Power. This is distinct from RVI (RUN312) and Intraday Intensity (RUN301) — BOP specifically measures close relative to the high-low midpoint rather than the open or using volume weighting. It's the cleanest measure of where institutional money is pushing the price within each bar.

## Proposed Config Changes (config.rs)

```rust
// ── RUN321: Balance of Power Oscillator ───────────────────────────────────────
// bop = (close - open) / (high - low)
// bop_smooth = EMA(bop, period)
// bop_signal = EMA(bop_smooth, signal_period)
// LONG: bop_smooth crosses above 0 AND bop > 0
// SHORT: bop_smooth crosses below 0 AND bop < 0
// Exit: bop_smooth crosses back through zero

pub const BOP_ENABLED: bool = true;
pub const BOP_PERIOD: usize = 14;
pub const BOP_SIGNAL: usize = 5;
pub const BOP_SL: f64 = 0.005;
pub const BOP_TP: f64 = 0.004;
pub const BOP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run321_1_bop_backtest.py)
2. **Walk-forward** (run321_2_bop_wf.py)
3. **Combined** (run321_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- SIGNAL sweep: 3 / 5 / 8
