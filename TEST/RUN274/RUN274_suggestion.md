# RUN274 — ZigZag Trend Strength: Swing High/Low Rate of Change

## Hypothesis

**Mechanism**: ZigZag indicator identifies swing highs and lows. The rate at which ZigZag forms new highs (or lows) measures trend strength. When ZigZag is making higher highs every N bars → strong uptrend. When it's making lower lows → strong downtrend. Trade when ZigZag confirms a directional series.

**Why not duplicate**: No prior RUN uses ZigZag. All prior trend RUNs use price directly or indicators. ZigZag trend strength is distinct because it explicitly tracks *swing points*, filtering out noise and identifying the underlying market structure.

## Proposed Config Changes (config.rs)

```rust
// ── RUN274: ZigZag Trend Strength ─────────────────────────────────────────
// zigzag = identify swing highs/lows using threshold (e.g., 5% reversal)
// zigzag_up_count = number of consecutive higher highs in last N swings
// zigzag_down_count = number of consecutive lower lows in last N swings
// zigzag_up_count >= 3 → strong uptrend → LONG
// zigzag_down_count >= 3 → strong downtrend → SHORT

pub const ZIGZAG_ENABLED: bool = true;
pub const ZIGZAG_THRESH: f64 = 0.05;         // 5% reversal for swing
pub const ZIGZAG_MIN_SWINGS: u32 = 3;       // consecutive swings for signal
pub const ZIGZAG_LOOKBACK: usize = 20;       // number of swings to check
pub const ZIGZAG_SL: f64 = 0.005;
pub const ZIGZAG_TP: f64 = 0.004;
pub const ZIGZAG_MAX_HOLD: u32 = 72;
```

---

## Validation Method

1. **Historical backtest** (run274_1_zigzag_backtest.py)
2. **Walk-forward** (run274_2_zigzag_wf.py)
3. **Combined** (run274_3_combined.py)

## Out-of-Sample Testing

- THRESH sweep: 0.03 / 0.05 / 0.07
- MIN_SWINGS sweep: 2 / 3 / 4
