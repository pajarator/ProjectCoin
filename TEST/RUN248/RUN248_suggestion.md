# RUN248 — Bollinger Band Momentum Score: Position + Direction Combined

## Hypothesis

**Mechanism**: BB Position = (close - lower_band) / (upper_band - lower_band). This gives 0-100 reading of where price is within the bands. BB Momentum = BB position - prior BB position. When BB position < 20 (lower band, oversold) AND BB momentum > 0 (position improving) → LONG. When BB position > 80 (upper band, overbought) AND BB momentum < 0 → SHORT.

**Why not duplicate**: No prior RUN uses Bollinger Band position combined with momentum. All prior BB RUNs use price crossing the bands or bandwidth. BB Momentum Score is unique because it combines *where* price is (position) with *how it's changing* (momentum), creating a more nuanced signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN248: Bollinger Band Momentum Score ────────────────────────────────
// bb_position = (close - lower) / (upper - lower) × 100
// bb_momentum = bb_position - bb_position[prior]
// LONG: bb_position < 20 AND bb_momentum > 0
// SHORT: bb_position > 80 AND bb_momentum < 0

pub const BB_MOM_ENABLED: bool = true;
pub const BB_MOM_PERIOD: usize = 20;         // BB period
pub const BB_MOM_STD: f64 = 2.0;             // BB std dev multiplier
pub const BB_MOM_LONG_POS: f64 = 20.0;       // oversold position threshold
pub const BB_MOM_SHORT_POS: f64 = 80.0;      // overbought position threshold
pub const BB_MOM_SL: f64 = 0.005;
pub const BB_MOM_TP: f64 = 0.004;
pub const BB_MOM_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run248_1_bb_mom_backtest.py)
2. **Walk-forward** (run248_2_bb_mom_wf.py)
3. **Combined** (run248_3_combined.py)

## Out-of-Sample Testing

- BB_PERIOD sweep: 15 / 20 / 30
- BB_STD sweep: 1.5 / 2.0 / 2.5
- LONG_POS sweep: 15 / 20 / 25
- SHORT_POS sweep: 75 / 80 / 85
