# RUN315 — Coppock Curve: Long-Cycle War Cycle Momentum

## Hypothesis

**Mechanism**: Coppock = weighted moving average of the sum of two ROC periods. Originally designed for market timing on monthly data (10-month and 14-month ROC). This is a long-cycle contrarian indicator — it identifies deep oversold bounces. The signal is when Coppock crosses above its signal line from below → LONG. This is not a short-term indicator — it captures major cyclical turns.

**Why not duplicate**: No prior RUN uses Coppock Curve. This is distinct from short-cycle momentum (all prior ROC RUNs use 3-32 bar windows). The long-period ROC windows (40+ bars) make this a macro-cycle reversal detector. It's the longest-cycle indicator tested so far.

## Proposed Config Changes (config.rs)

```rust
// ── RUN315: Coppock Curve ─────────────────────────────────────────────────────
// coppock = WMA(sum(roc_long + roc_short), period)
// roc_long = rate_of_change(close, LONG_PERIOD)
// roc_short = rate_of_change(close, SHORT_PERIOD)
// signal = SMA(coppock, signal_period)
// LONG: coppock crosses above signal from below
// SHORT: coppock crosses below signal from above

pub const COPPOCK_ENABLED: bool = true;
pub const COPPOCK_LONG_PERIOD: usize = 40;    // ~10 days at 15m bars
pub const COPPOCK_SHORT_PERIOD: usize = 20;   // ~5 days at 15m bars
pub const COPPOCK_WMA_PERIOD: usize = 10;
pub const COPPOCK_SIGNAL: usize = 5;
pub const COPPOCK_SL: f64 = 0.005;
pub const COPPOCK_TP: f64 = 0.004;
pub const COPPOCK_MAX_HOLD: u32 = 96;        // long-cycle = longer hold
```

---

## Validation Method

1. **Historical backtest** (run315_1_coppock_backtest.py)
2. **Walk-forward** (run315_2_coppock_wf.py)
3. **Combined** (run315_3_combined.py)

## Out-of-Sample Testing

- LONG_PERIOD sweep: 30 / 40 / 60
- SHORT_PERIOD sweep: 14 / 20 / 28
- WMA_PERIOD sweep: 8 / 10 / 14
- SIGNAL sweep: 3 / 5 / 8
