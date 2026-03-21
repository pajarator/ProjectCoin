# RUN178 — Scalp-Regime Direction Lock: Scalp Only When Regime Opposes Scalp Direction

## Hypothesis

**Mechanism**: Scalp entries fire on 1m signals regardless of regime. When market is in LONG regime, going SHORT on a scalp is fighting the broader trend — higher failure rate. This RUN locks scalp direction to regime: LONG regime → scalp LONG only; SHORT regime → scalp SHORT only; ISO_SHORT → scalp SHORT only. Scalp direction must match or be neutral to regime.

**Why not duplicate**: RUN12 (Scalp market mode filter) was about scalp direction matching regime direction. RUN95 (Scalp Momentum Alignment) is similar. But none specifically implement a hard LOCK — scalp entries are completely blocked if they oppose regime.

## Proposed Config Changes (config.rs)

```rust
// ── RUN178: Scalp-Regime Direction Lock ────────────────────────────────
// Scalp LONG allowed: LONG regime or ISO_SHORT regime only
// Scalp SHORT allowed: SHORT regime only
// BLOCK scalp entries that oppose current regime mode

pub const SCALP_LOCK_ENABLED: bool = true;
```

Modify `scalp_entry_with_price` in `engine.rs`:

```rust
fn scalp_direction_allowed(regime: Regime, scalp_dir: Direction) -> bool {
    if !config::SCALP_LOCK_ENABLED { return true; }
    match (regime, scalp_dir) {
        (Regime::Long, Direction::Long) => true,
        (Regime::Short, Direction::Short) => true,
        (Regime::IsoShort, Direction::Short) => true,
        _ => false,  // scalp LONG during SHORT regime = blocked
    }
}
```

---

## Validation Method

1. **Historical backtest** (run178_1_scalplock_backtest.py)
2. **Walk-forward** (run178_2_scalplock_wf.py)
3. **Combined** (run178_3_combined.py)

## Out-of-Sample Testing

- Test with vs without lock, measure WR and PF improvement on scalp trades
