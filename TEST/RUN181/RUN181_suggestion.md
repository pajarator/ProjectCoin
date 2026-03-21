# RUN181 — Per-Coin ATR Percentile Stop Loss: Dynamic SL Based on Natural Volatility Distribution

## Hypothesis

**Mechanism**: COINCLAW uses a fixed SL of 0.3% for all coins. But a coin like SHIB moves 10x more (in %) than BTC on average. A fixed 0.3% SL for SHIB is either too tight (always gets stopped out) or too loose (takes huge losses when it does trigger). A percentile-based SL: set SL = ATR(14) × Nth percentile of the ATR distribution over past 60 bars.

**Why not duplicate**: No prior RUN adapts SL based on ATR percentile per coin. All prior stop-loss RUNs use fixed percentages.

## Proposed Config Changes (config.rs)

```rust
// ── RUN181: Per-Coin ATR Percentile Stop Loss ──────────────────────────
// SL = ATR(14) × ATR_PCT_MULT × atr_percentile
// atr_percentile: e.g., 50th pct of last 60 ATR readings

pub const ATR_SL_ENABLED: bool = true;
pub const ATR_SL_WINDOW: usize = 60;       // rolling window for ATR distribution
pub const ATR_SL_PCT_MULT: f64 = 1.0;    // SL = ATR × this × atr_pct
pub const ATR_SL_PCT: f64 = 0.60;       // percentile: 0.60 = 60th pct of ATR dist
```

Add to `CoinState` in `state.rs`:

```rust
pub atr_history: Vec<f64>,  // rolling ATR(14) history for percentile calculation
```

Add in `indicators.rs`:

```rust
pub fn atr_percentile(history: &[f64], pct: f64) -> f64 {
    if history.is_empty() { return 1.0; }
    let mut sorted = history.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = ((sorted.len() as f64 - 1.0) * pct).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}
```

Modify `open_position` in `engine.rs`:

```rust
fn dynamic_sl(ind: &Ind15m, cs: &CoinState) -> f64 {
    let pct_sl = indicators::atr_percentile(&cs.atr_history, config::ATR_SL_PCT);
    pct_sl * config::ATR_SL_PCT_MULT
}
```

---

## Validation Method

1. **Historical backtest** (run181_1_atrsl_backtest.py)
2. **Walk-forward** (run181_2_atrsl_wf.py)
3. **Combined** (run181_3_combined.py)

## Out-of-Sample Testing

- PCT_MULT sweep: 0.8 / 1.0 / 1.2
- PCT sweep: 0.50 / 0.60 / 0.70
- WINDOW sweep: 30 / 60 / 120 bars
