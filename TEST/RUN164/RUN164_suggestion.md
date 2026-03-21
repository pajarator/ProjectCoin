# RUN164 — VWAP Deviation Score: Continuous Distance-from-VWAP as Scalper Entry Trigger

## Hypothesis

**Mechanism**: COINCLAW uses z-score for mean-reversion but VWAP deviation is a complementary measure — it tells you how far price has drifted from the volume-weighted average. A continuous VWAP deviation score (not just binary crossover) can trigger scalp entries when deviation exceeds a threshold and is beginning to compress.

**Implementation**: Compute VWAP deviation ratio: `(price - vwap) / atr14`. When this ratio exceeds ±1.5 AND is contracting (deviation_ma3 < deviation_prev), enter in the mean-reversion direction. This combines magnitude (overextended) with momentum (reversing).

**Why not duplicate**: RUN60 used Z-score momentum filter. RUN70 used Z-score convergence. RUN129 (VWAP Deviation Percentile) was proposed but unexecuted — this uses continuous VWAP deviation ratio, not percentile.

## Proposed Config Changes (config.rs)

```rust
// ── RUN164: VWAP Deviation Score Scalp Trigger ─────────────────────────
// VWAP_dev_ratio = (price - vwap) / ATR14
// Entry: |dev_ratio| > DEV_RATIO_THRESH AND dev_ratio_ma3 is contracting
// Exit: dev_ratio crosses 0 OR MAX_HOLD bars

pub const VDEV_ENABLED: bool = true;
pub const VDEV_RATIO_THRESH: f64 = 1.5;    // deviation must exceed 1.5x ATR
pub const VDEV_CONTRACTION: f64 = 0.80;    // dev_ratio_ma3 must be < 0.80x dev_ratio_prev
pub const VDEV_SL: f64 = 0.003;          // 0.3% stop
pub const VDEV_TP: f64 = 0.002;          // 0.2% take profit
pub const VDEV_MAX_HOLD: u32 = 8;         // ~2 hours at 15m bars
```

Add to `Ind15m` in `indicators.rs`:

```rust
pub vwap_dev_ratio: f64,      // (price - vwap) / atr14
pub vwap_dev_ratio_ma3: f64,  // 3-bar smoothed deviation ratio
pub vwap_dev_ratio_prev: f64,
```

Add entry logic in `engine.rs`:

```rust
fn check_vwap_dev_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::VDEV_ENABLED { return None; }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }

    let dev = ind.vwap_dev_ratio;
    let dev_ma = ind.vwap_dev_ratio_ma3;
    let dev_prev = ind.vwap_dev_ratio_prev;
    if dev.is_nan() || dev_ma.is_nan() || dev_prev.is_nan() { return None; }

    // Deviation exceeding threshold AND contracting
    if dev.abs() < config::VDEV_RATIO_THRESH { return None; }
    if dev_ma.abs() >= dev_prev.abs() * config::VDEV_CONTRACTION { return None; }

    // LONG: price below VWAP (dev < 0) and contracting toward 0
    if dev < -config::VDEV_RATIO_THRESH {
        return Some((Direction::Long, "vwap_dev"));
    }
    // SHORT: price above VWAP (dev > 0) and contracting toward 0
    if dev > config::VDEV_RATIO_THRESH {
        return Some((Direction::Short, "vwap_dev"));
    }
    None
}
```

---

## Validation Method

1. **Historical backtest** (run164_1_vdev_backtest.py): 18 coins, 1-year 15m, sweep thresholds
2. **Walk-forward** (run164_2_vdev_wf.py): 3-window walk-forward
3. **Combined** (run164_3_combined.py): COINCLAW v16 vs +vwap_dev

## Out-of-Sample Testing

- RATIO_THRESH sweep: 1.0 / 1.5 / 2.0
- CONTRACTION sweep: 0.70 / 0.80 / 0.90
