# RUN263 — EMA Ribbon Momentum: Multi-EMA Alignment Trend Confirmation

## Hypothesis

**Mechanism**: An EMA ribbon (5, 10, 20, 50) in ascending order = strong uptrend. Descending order = strong downtrend. When the ribbon "fans" (spreads apart) → trend accelerating. When ribbon converges → trend weakening. Trade when ribbon fans in a direction after alignment.

**Why not duplicate**: No prior RUN uses EMA ribbon fan/condensation. RUN182 uses EMA ribbon squeeze but that's for detecting breakout. EMA Ribbon Momentum is distinct — it uses the *fanning* (spreading) of the ribbon as the momentum signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN263: EMA Ribbon Momentum ──────────────────────────────────────────
// ribbon_spread = EMA(50) / EMA(5) - 1  (measure of fanning)
// ribbon_ascending = EMA(5) > EMA(10) > EMA(20) > EMA(50)
// ribbon_descending = reverse
// LONG: ribbon_fanning = true AND ribbon_ascending = true
// SHORT: ribbon_fanning = true AND ribbon_descending = true

pub const RIBBON_MOM_ENABLED: bool = true;
pub const RIBBON_MOM_EMA1: usize = 5;
pub const RIBBON_MOM_EMA2: usize = 10;
pub const RIBBON_MOM_EMA3: usize = 20;
pub const RIBBON_MOM_EMA4: usize = 50;
pub const RIBBON_MOM_FAN_THRESH: f64 = 0.02; // 2% spread = fanning
pub const RIBBON_MOM_SL: f64 = 0.005;
pub const RIBBON_MOM_TP: f64 = 0.004;
pub const RIBBON_MOM_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run263_1_ribbon_mom_backtest.py)
2. **Walk-forward** (run263_2_ribbon_mom_wf.py)
3. **Combined** (run263_3_combined.py)

## Out-of-Sample Testing

- EMA1 sweep: 3 / 5 / 7
- EMA2 sweep: 8 / 10 / 12
- EMA3 sweep: 15 / 20 / 25
- EMA4 sweep: 40 / 50 / 75
- FAN_THRESH sweep: 0.01 / 0.02 / 0.03
