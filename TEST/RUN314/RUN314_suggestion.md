# RUN314 — Mass Index Reversal: High ADR Exhaustion Detection

## Hypothesis

**Mechanism**: Mass Index uses two EMA periods (fast and slow) of the high-low range. When the fast EMA diverges significantly from the slow EMA (range expansion) → trend exhaustion → expect reversal. The key signal is the "reversal bulge" — when Mass Index rises above a threshold then drops below a second threshold → reversal in direction. High ADR (Average Daily Range) combined with Mass Index bulge = strongest reversal signals.

**Why not duplicate**: RUN193 uses Mass Index but as a standalone reversal detection. RUN14 added Mass Index to the indicator library. This RUN specifically combines Mass Index reversal bulge with ADR measurement — only triggering on high-volatility days when the reversal is most reliable. The mass index on low-volatility days produces false signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN314: Mass Index Reversal with ADR Confirmation ─────────────────────────
// mass_index = sum(EMA(high-low, fast) / EMA(high-low, slow), period)
// reversal_bulge: mass_index rises above BULGE_LEVEL then drops below BULGE_EXIT
// ADR check: adr_percent must exceed ADR_THRESH for valid signal
// LONG: reversal_bulge_complete AND price > SMA(20)
// SHORT: reversal_bulge_complete AND price < SMA(20)

pub const MASS_ADR_ENABLED: bool = true;
pub const MASS_FAST: usize = 9;
pub const MASS_SLOW: usize = 25;
pub const MASS_PERIOD: usize = 25;           // sum period
pub const MASS_BULGE: f64 = 27.0;           // reversal bulge threshold
pub const MASS_BULGE_EXIT: f64 = 26.5;      // bulge exit (trigger)
pub const MASS_ADR_THRESH: f64 = 0.02;      // min 2% ADR for valid signal
pub const MASS_SL: f64 = 0.005;
pub const MASS_TP: f64 = 0.004;
pub const MASS_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run314_1_mass_adr_backtest.py)
2. **Walk-forward** (run314_2_mass_adr_wf.py)
3. **Combined** (run314_3_combined.py)

## Out-of-Sample Testing

- BULGE sweep: 25 / 27 / 29
- BULGE_EXIT sweep: 24.5 / 26.5 / 28.5
- ADR_THRESH sweep: 0.015 / 0.02 / 0.03
- PERIOD sweep: 20 / 25 / 30
