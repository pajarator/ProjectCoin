# RUN146 — Z-Score Trailing Exit: Early Profit Capture on Mean-Reversion Recoveries

## Hypothesis

**Mechanism**: COINCLAW's regime exit fires when z-score crosses 0 (Z0) or price crosses SMA20. But z-score often recovers 40-60% of its entry deviation before the full exit conditions are met. Adding a `Z_TRAIL_RECOVERY_PCT` exit — close when z-score has recovered ≥N% of its entry deviation — captures smaller but more consistent profits while reducing exposure to full mean-reversion cycles.

**Why this is not a duplicate**: RUN88 (Trailing Z-Score Exit) was proposed but unexecuted — it uses the same concept. However, this RUN improves on it by using a **3-bar smoothed z-score** (z_ma3) to filter noise, avoiding premature exits from z-score whipsaws. Additionally, this combines with the existing Z0/SMA exits (not replacing them) — it's a *partial* exit that takes profits early while keeping the position open for the full exit signal.

**Why it could work**: Mean-reversion moves often partially reverse before fully completing. Capturing 0.2-0.4% profits on 50% of trades (even if the other 50% give it back at the full exit) improves win rate and reduces holding time. If implemented as a partial exit (close 50% of position at Z_TRAIL threshold), it both books profit and lets the remaining position run to the full exit.

---

## Proposed Config Changes (config.rs)

```rust
// ── RUN6: Z-Score Trailing Exit (Partial Profit-Taking) ──────────────
// Exit Type A (partial exit): when z-score recovers Z_TRAIL_RECOVERY_PCT of entry deviation
// Exit Type B (full exit): when z-score fully crosses Z_TRAIL_EXIT_Z (tighter than Z0)
// Both are additions to existing SMA/Z0 exits, not replacements

pub const Z_TRAIL_ENABLED: bool = true;
pub const Z_TRAIL_RECOVERY_PCT: f64 = 0.40;    // exit when z has recovered 40% of entry deviation
pub const Z_TRAIL_PARTIAL_SIZE: f64 = 0.50;    // close 50% of position at this exit
pub const Z_TRAIL_EXIT_Z: f64 = -0.25;         // full exit when z crosses this (tighter than Z0=-0.5)
pub const Z_TRAIL_USE_SMOOTHED: bool = true;   // use z_ma3 (3-bar avg) instead of raw z
```

Add to `Ind15m` in `indicators.rs`:
```rust
pub z_ma3: f64,        // 3-bar rolling average of z-score (noise filter)
pub z_ma3_prev: f64,   // previous bar's z_ma3 for crossover detection
```

Add to `check_exit` in engine.rs:
```rust
// RUN6: Z-score trailing exit — book partial profits when z recovers X% of entry deviation
fn check_z_trail_exit(state: &mut SharedState, ci: usize) -> Option<(f64, &'static str)> {
    let cs = &state.coins[ci];
    let pos = cs.pos.as_ref()?;
    if pos.trade_type != Some(TradeType::Regime) { return None; }
    if !config::Z_TRAIL_ENABLED { return None; }

    let ind = cs.ind_15m.as_ref()?;
    let z = if config::Z_TRAIL_USE_SMOOTHED { ind.z_ma3 } else { ind.z };
    let z_prev = if config::Z_TRAIL_USE_SMOOTHED { ind.z_ma3_prev } else { ind.z_prev };
    if z.is_nan() || z_prev.is_nan() { return None; }

    // Compute entry z_deviation = entry_z - 0 (center); how far was z from mean at entry?
    // We track entry_z in the Position struct
    let entry_z = cs.pos.as_ref()?.entry_z.unwrap_or(z);
    let deviation_at_entry = (z - 0.0).abs();  // how far z was from mean at entry

    // recovery_pct = (entry_z - current_z) / entry_z  (for z moving toward 0)
    let recovery_pct = if pos.dir == "long" && entry_z < 0.0 {
        ((entry_z - z) / entry_z.abs()).abs()
    } else if pos.dir == "short" && entry_z > 0.0 {
        ((entry_z - z) / entry_z.abs()).abs()
    } else {
        0.0
    };

    if recovery_pct >= config::Z_TRAIL_RECOVERY_PCT {
        let exit_price = ind.p;
        return Some((exit_price, "Z_TRAIL"));
    }

    // Full exit when smoothed z crosses Z_TRAIL_EXIT_Z (tighter than standard Z0)
    if pos.dir == "long" && z <= config::Z_TRAIL_EXIT_Z && z_prev > config::Z_TRAIL_EXIT_Z {
        return Some((ind.p, "Z_TRAIL_FULL"));
    }
    if pos.dir == "short" && z >= -config::Z_TRAIL_EXIT_Z && z_prev < -config::Z_TRAIL_EXIT_Z {
        return Some((ind.p, "Z_TRAIL_FULL"));
    }

    None
}
```

Note: `Position` needs an `entry_z` field to track the z-score at entry time.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (Z0 + SMA exits only)
- **Comparison**: same trades with Z_TRAIL partial + full exits added

**Metrics to measure**:
- Z_TRAIL exit win rate (hypothesis: >65% — early partial profits)
- Avg partial profit per Z_TRAIL exit (hypothesis: +0.15-0.25%)
- Whether remaining position still captures full mean-reversion move
- Overall portfolio P&L improvement
- Holding time reduction (book profits faster → more trades possible)

**Hypothesis**: Z_TRAIL partial exits (50% at 40% recovery) should improve portfolio P&L by >5% while reducing avg holding time by >20%, without sacrificing the full exit on remaining position.

---

## Validation Method

1. **Historical backtest** (run6_1_ztrail_backtest.py):
   - 18 coins, 1-year 15m data
   - Simulate COINCLAW v16 with existing exits
   - Re-simulate with Z_TRAIL partial exit added
   - Record: Z_TRAIL exit frequency, partial profit size, whether full exit subsequently fires
   - Output: per-coin WR improvement, avg hold time, P&L change

2. **Walk-forward** (run6_2_ztrail_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep Z_TRAIL_RECOVERY_PCT: 0.30 / 0.40 / 0.50 / 0.60
   - Sweep Z_TRAIL_PARTIAL_SIZE: 0.33 / 0.50 / 0.67
   - Compare: partial-only vs partial+full exit

3. **Combined comparison** (run6_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + Z_TRAIL
   - Portfolio stats, exit reason distribution, holding time distribution

---

## Out-of-Sample Testing

- RECOVERY_PCT sweep: 0.30 / 0.40 / 0.50 / 0.60
- PARTIAL_SIZE sweep: 0.33 / 0.50 / 0.67
- USE_SMOOTHED: true / false
- OOS: final 4 months held out from all parameter selection
