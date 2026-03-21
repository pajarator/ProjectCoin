# RUN227 — SMA Breadth Oscillator: Percentage of Coins Above Moving Average

## Hypothesis

**Mechanism**: Track what percentage of the 18 coins have their close above their 20-period SMA. When breadth > 70% → most coins are rallying → strong bullish environment. When breadth < 30% → most coins falling → strong bearish environment. Use breadth as a regime filter: in HIGH breadth environments, mean-reversion SHORTs are more likely to fail; in LOW breadth, mean-reversion LONGs are more likely to fail.

**Why not duplicate**: No prior RUN uses market breadth (percentage of coins above SMA). All prior regime RUNs use BTC dominance, funding rates, or single-coin indicators. Breadth is unique because it measures the *market-wide* participation in a move.

## Proposed Config Changes (config.rs)

```rust
// ── RUN227: SMA Breadth Oscillator ───────────────────────────────────────
// breadth = coins_above_sma(20) / total_coins × 100
// breadth > 70 → overbought regime (SHORT bias)
// breadth < 30 → oversold regime (LONG bias)
// breadth 30-70 → neutral
// Use breadth to filter entry confidence

pub const BREADTH_ENABLED: bool = true;
pub const BREADTH_SMA_PERIOD: usize = 20;    // SMA lookback
pub const BREADTH_OVERBOUGHT: f64 = 70.0;   // overbought threshold
pub const BREADTH_OVERSOLD: f64 = 30.0;     // oversold threshold
pub const BREADTH_STRONG_THRESH: f64 = 80.0; // very strong (avoid counter-trend)
pub const BREADTH_WEAK_THRESH: f64 = 20.0;   // very weak (avoid counter-trend)
```

Modify engine to check breadth before each trade:
- If breadth > BREADTH_STRONG_THRESH → reduce SHORT entries, boost LONG exits
- If breadth < BREADTH_WEAK_THRESH → reduce LONG entries, boost SHORT exits

---

## Validation Method

1. **Historical backtest** (run227_1_breadth_backtest.py)
2. **Walk-forward** (run227_2_breadth_wf.py)
3. **Combined** (run227_3_combined.py)

## Out-of-Sample Testing

- SMA_PERIOD sweep: 10 / 20 / 50
- OVERBOUGHT sweep: 60 / 70 / 80
- OVERSOLD sweep: 20 / 30 / 40
