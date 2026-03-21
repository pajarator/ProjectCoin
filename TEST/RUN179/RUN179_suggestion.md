# RUN179 — Trade Confidence Partial Exit Scaling: Z-Score Magnitude as Exit Size Determinant

## Hypothesis

**Mechanism**: Entries at more extreme z-scores have higher conviction — the price is further from fair value. When entering at z < -2.0, take larger partial profits (67% of position at first exit). When entering at z = -1.5, take smaller partial profits (33%). This dynamically scales exit aggressiveness based on entry confidence.

**Why not duplicate**: No prior RUN scales partial exit size based on entry z-score magnitude. RUN53 (Partial Exit/Scale-Out) was proposed but uses fixed 50% exit. RUN46 (Partial Reversion Signal Exit) uses Z-score for exit timing, not size.

## Proposed Config Changes (config.rs)

```rust
// ── RUN179: Trade Confidence Partial Exit Scaling ────────────────────────
// Entry Z magnitude determines partial exit size
// |z| >= 2.0 → exit 67% at first exit
// |z| >= 1.5 → exit 50% at first exit
// |z| < 1.5 → exit 33% at first exit

pub const CONF_EXIT_ENABLED: bool = true;
pub const CONF_EXIT_Z_HIGH: f64 = 2.0;    // high confidence threshold
pub const CONF_EXIT_Z_MID: f64 = 1.5;     // mid confidence threshold
pub const CONF_EXIT_SIZE_HIGH: f64 = 0.67;  // exit 67% of position
pub const CONF_EXIT_SIZE_MID: f64 = 0.50;  // exit 50%
pub const CONF_EXIT_SIZE_LOW: f64 = 0.33;  // exit 33%
```

Modify `check_exit` in `engine.rs`:

```rust
fn confidence_exit_size(entry_z: f64) -> f64 {
    if !config::CONF_EXIT_ENABLED { return 0.50; }  // default 50%
    let z_mag = entry_z.abs();
    if z_mag >= config::CONF_EXIT_Z_HIGH {
        return config::CONF_EXIT_SIZE_HIGH;
    } else if z_mag >= config::CONF_EXIT_Z_MID {
        return config::CONF_EXIT_SIZE_MID;
    } else {
        return config::CONF_EXIT_SIZE_LOW;
    }
}
```

---

## Validation Method

1. **Historical backtest** (run179_1_confexit_backtest.py)
2. **Walk-forward** (run179_2_confexit_wf.py)
3. **Combined** (run179_3_combined.py)

## Out-of-Sample Testing

- Z_HIGH sweep: 1.5 / 2.0 / 2.5
- SIZE_HIGH sweep: 0.50 / 0.67 / 0.75
