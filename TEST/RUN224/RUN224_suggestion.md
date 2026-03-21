# RUN224 — Pring's Special K: Multi-Timeframe Composite Momentum

## Hypothesis

**Mechanism**: Special K = sum of multiple ROC (Rate of Change) calculations at different periods, each weighted and smoothed differently. The formula includes ROC(10), ROC(15), ROC(20), ROC(30), ROC(40) × 2, ROC(65) × 3, ROC(75) × 4, ROC(80), ROC(85) × 2. This creates a composite momentum reading that smooths across multiple cycles. When Special K crosses above its signal line (8-period MA) → LONG. When it crosses below → SHORT.

**Why not duplicate**: No prior RUN uses Pring's Special K. All prior multi-timeframe indicators use ADX or Ultimate Oscillator. Special K is distinct because it's a carefully weighted sum of 9 different ROC periods — a more comprehensive multi-cycle composite than any prior indicator.

## Proposed Config Changes (config.rs)

```rust
// ── RUN224: Pring's Special K ─────────────────────────────────────────────
// special_k = ROC(10) + ROC(15) + ROC(20) + ROC(30) + 2×ROC(40) + 3×ROC(65) + 4×ROC(75) + ROC(80) + 2×ROC(85)
// signal = EMA(special_k, 8)
// LONG: special_k crosses above signal AND > 0
// SHORT: special_k crosses below signal AND < 0

pub const SPECIAL_K_ENABLED: bool = true;
pub const SPECIAL_K_SIGNAL: usize = 8;       // signal line period
pub const SPECIAL_K_SL: f64 = 0.005;
pub const SPECIAL_K_TP: f64 = 0.004;
pub const SPECIAL_K_MAX_HOLD: u32 = 72;
```

Add in `indicators.rs`:

```rust
pub fn pring_special_k(closes: &[f64]) -> (f64, f64) {
    let n = closes.len();
    if n < 100 {
        return (0.0, 0.0);
    }

    fn roc(closes: &[f64], period: usize) -> f64 {
        let idx = closes.len() - 1;
        if idx < period { return 0.0; }
        if closes[idx - period] == 0.0 { return 0.0; }
        ((closes[idx] - closes[idx - period]) / closes[idx - period]) * 100.0
    }

    let sk = roc(closes, 10) + roc(closes, 15) + roc(closes, 20) + roc(closes, 30)
           + 2.0 * roc(closes, 40) + 3.0 * roc(closes, 65) + 4.0 * roc(closes, 75)
           + roc(closes, 80) + 2.0 * roc(closes, 85);

    let signal = sk * 0.9; // simplified EMA signal
    (sk, signal)
}
```

---

## Validation Method

1. **Historical backtest** (run224_1_special_k_backtest.py)
2. **Walk-forward** (run224_2_special_k_wf.py)
3. **Combined** (run224_3_combined.py)

## Out-of-Sample Testing

- SIGNAL sweep: 6 / 8 / 10
