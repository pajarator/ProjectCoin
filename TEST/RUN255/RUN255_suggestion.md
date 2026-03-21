# RUN255 — Liquidity Pool Sweep: Stop Hunt Detection for Reversal Entries

## Hypothesis

**Mechanism**: Institutional traders often "hunt" for stop losses above/below key levels ( liquidity pools ). When price spikes violently through a level (e.g., recent high/low) and immediately reverses → the stop hunt occurred. After the sweep, the market often returns to the swept zone. Post-sweep reversal is a high-probability entry.

**Why not duplicate**: No prior RUN detects liquidity sweeps. All prior RUNs use standard breakout or mean-reversion approaches. Liquidity sweep detection is a specific institutional behavior pattern that identifies when stop hunts occur.

## Proposed Config Changes (config.rs)

```rust
// ── RUN255: Liquidity Pool Sweep ────────────────────────────────────────
// sweep_detection = price spikes > 0.5% beyond recent high/low within 3 bars
// sweep_confirmation = price reverses > 50% of the spike within 2 bars
// LONG: price swept below recent low AND reversed (bullish sweep)
// SHORT: price swept above recent high AND reversed (bearish sweep)

pub const LIQUIDITY_ENABLED: bool = true;
pub const LIQUIDITY_SWEEP_THRESH: f64 = 0.005;  // 0.5% = minimum sweep
pub const LIQUIDITY_REVERSAL_MIN: f64 = 0.50;    // 50% reversal required
pub const LIQUIDITY_LOOKBACK: usize = 20;         // lookback for highs/lows
pub const LIQUIDITY_SL: f64 = 0.005;
pub const LIQUIDITY_TP: f64 = 0.004;
pub const LIQUIDITY_MAX_HOLD: u32 = 36;
```

---

## Validation Method

1. **Historical backtest** (run255_1_liquidity_backtest.py)
2. **Walk-forward** (run255_2_liquidity_wf.py)
3. **Combined** (run255_3_combined.py)

## Out-of-Sample Testing

- SWEEP_THRESH sweep: 0.003 / 0.005 / 0.007
- REVERSAL_MIN sweep: 0.40 / 0.50 / 0.60
- LOOKBACK sweep: 10 / 20 / 40
