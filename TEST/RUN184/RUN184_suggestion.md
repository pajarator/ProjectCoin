# RUN184 — Rolling Correlation Exit: BTC Correlation Breakdown as Regime Change Signal

## Hypothesis

**Mechanism**: Altcoins normally correlate with BTC. When an altcoin's correlation with BTC drops sharply (from 0.8+ to <0.5), it signals the coin is decoupling — it may be about to move independently. For LONG positions, a correlation drop means the BTC tailwind is fading — exit. For SHORT positions, correlation drop could mean bottom.

**Why not duplicate**: No prior RUN uses rolling correlation as an exit signal. All prior correlation RUNs use it as an entry filter.

## Proposed Config Changes (config.rs)

```rust
// ── RUN184: Rolling Correlation Exit ───────────────────────────────────
// corr_window = rolling 20-bar correlation of altcoin returns with BTC returns
// LONG exit: correlation drops from >0.6 to <0.4 while in profit
// SHORT exit: correlation drops below 0.3

pub const CORR_EXIT_ENABLED: bool = true;
pub const CORR_WINDOW: usize = 20;        // rolling correlation window
pub const CORR_LONG_EXIT: f64 = 0.40;     // exit LONG when corr drops below this
pub const CORR_SHORT_EXIT: f64 = 0.30;   // exit SHORT when corr drops below this
```

---

## Validation Method

1. **Historical backtest** (run184_1_correxit_backtest.py)
2. **Walk-forward** (run184_2_correxit_wf.py)
3. **Combined** (run184_3_combined.py)

## Out-of-Sample Testing

- WINDOW sweep: 10 / 20 / 40 bars
- LONG_EXIT sweep: 0.30 / 0.40 / 0.50
