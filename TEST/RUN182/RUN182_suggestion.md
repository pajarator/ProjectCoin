# RUN182 — EMA Ribbon Squeeze: Multiple EMA Convergence as Momentum Buildup Signal

## Hypothesis

**Mechanism**: When multiple EMAs (9, 21, 50, 200) converge tightly (within 0.2% of each other), it signals a coiled market — a directional explosion is imminent. After the squeeze, price typically breaks in the direction of the prevailing trend. COINCLAW has no EMA ribbon indicator.

**Why not duplicate**: No prior RUN uses EMA ribbon squeeze. All are based on Bollinger Bands or ATR compression.

## Proposed Config Changes (config.rs)

```rust
// ── RUN182: EMA Ribbon Squeeze ─────────────────────────────────────────
// ribbon_spread = (max(EMA9,EMA21,EMA50,EMA200) / min(EMA9,EMA21,EMA50,EMA200)) - 1.0
// spread < 0.002 = squeeze → breakout imminent
// After squeeze: price breaks above max EMA → LONG momentum
// After squeeze: price breaks below min EMA → SHORT momentum

pub const RIBBON_ENABLED: bool = true;
pub const RIBBON_SPREAD_THRESH: f64 = 0.002;   // 0.2% = tight squeeze
pub const RIBBON_CONFIRM_BARS: u32 = 2;          // breakout must persist 2 bars
pub const RIBBON_SL: f64 = 0.005;
pub const RIBBON_TP: f64 = 0.004;
pub const RIBBON_MAX_HOLD: u32 = 24;
```

---

## Validation Method

1. **Historical backtest** (run182_1_ribbon_backtest.py)
2. **Walk-forward** (run182_2_ribbon_wf.py)
3. **Combined** (run182_3_combined.py)

## Out-of-Sample Testing

- SPREAD_THRESH sweep: 0.001 / 0.002 / 0.003
