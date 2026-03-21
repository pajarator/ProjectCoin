# RUN298 — KST + RSI Confluence: Dual Momentum-Oscillator Entry Filter

## Hypothesis

**Mechanism**: KST (Know Sure Thing) measures rate-of-change momentum across 4 different ROC windows. RSI measures overbought/oversold. Both are oscillators but measure different things — KST is trend-following momentum, RSI is bounded oscillator. Require BOTH to align: KST crosses signal AND RSI at extreme level → strong confluence signal. Single-indicator signals are filtered out.

**Why not duplicate**: RUN13 uses KST as a standalone complement signal. RUN276 uses RSI-EMA crossover. RUN237 uses Williams %R + EMA filter. No RUN combines KST crossover confirmation with RSI extreme filter. This is dual-oscillator confluence — both oscillators must agree before entry.

## Proposed Config Changes (config.rs)

```rust
// ── RUN298: KST + RSI Confluence ───────────────────────────────────────────
// kst = weighted ROC across 4 windows (8,16,24,32 default)
// kst_signal = 8-bar SMA of KST
// LONG: kst crosses above kst_signal AND RSI > RSI_EXTREME_LONG
// SHORT: kst crosses below kst_signal AND RSI < RSI_EXTREME_SHORT
// Wait for cooldown_bars between signals (prevent over-trading)

pub const KST_RSI_ENABLED: bool = true;
pub const KST_RSI_ROC1: usize = 8;
pub const KST_RSI_ROC2: usize = 16;
pub const KST_RSI_ROC3: usize = 24;
pub const KST_RSI_ROC4: usize = 32;
pub const KST_RSI_SIGNAL: usize = 8;
pub const KST_RSI_RSI_LONG: f64 = 60.0;   // RSI must be here or above for LONG
pub const KST_RSI_RSI_SHORT: f64 = 40.0;  // RSI must be here or below for SHORT
pub const KST_RSI_SL: f64 = 0.005;
pub const KST_RSI_TP: f64 = 0.004;
pub const KST_RSI_MAX_HOLD: u32 = 48;
pub const KST_RSI_COOLDOWN: u32 = 8;
```

---

## Validation Method

1. **Historical backtest** (run298_1_kst_rsi_backtest.py)
2. **Walk-forward** (run298_2_kst_rsi_wf.py)
3. **Combined** (run298_3_combined.py)

## Out-of-Sample Testing

- RSI_LONG sweep: 55 / 60 / 65
- RSI_SHORT sweep: 35 / 40 / 45
- ROC1 sweep: 6 / 8 / 12
- ROC4 sweep: 24 / 32 / 48
