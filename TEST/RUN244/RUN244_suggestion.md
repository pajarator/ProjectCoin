# RUN244 — Fibonacci Retracement Confluence: Key Level Reversal

## Hypothesis

**Mechanism**: Identify the most recent swing high and swing low. Calculate Fibonacci retracement levels (23.6%, 38.2%, 50%, 61.8%, 78.6%). When price retraces to one of these levels AND shows a reversal indicator (RSI < 40 for LONG, RSI > 60 for SHORT) → high-probability entry. The 38.2% and 61.8% levels are historically the most significant.

**Why not duplicate**: No prior RUN uses Fibonacci retracement levels. All prior support/resistance RUNs use pivot points or VWAP. Fibonacci levels are a distinct price geometry approach based on the natural ratios found in markets.

## Proposed Config Changes (config.rs)

```rust
// ── RUN244: Fibonacci Retracement Confluence ────────────────────────────
// Find swing high and swing low over lookback period
// Retracement levels: 23.6%, 38.2%, 50%, 61.8%, 78.6%
// LONG: price touches Fib level AND RSI < 40
// SHORT: price touches Fib level AND RSI > 60

pub const FIB_ENABLED: bool = true;
pub const FIB_LOOKBACK: usize = 50;          // swing detection lookback
pub const FIB_LEVEL: f64 = 0.382;           // primary level (38.2%)
pub const FIB_RSI_PERIOD: usize = 14;
pub const FIB_RSI_LONG: f64 = 40.0;        // RSI for LONG confirmation
pub const FIB_RSI_SHORT: f64 = 60.0;        // RSI for SHORT confirmation
pub const FIB_SL: f64 = 0.005;
pub const FIB_TP: f64 = 0.004;
pub const FIB_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run244_1_fib_backtest.py)
2. **Walk-forward** (run244_2_fib_wf.py)
3. **Combined** (run244_3_combined.py)

## Out-of-Sample Testing

- LOOKBACK sweep: 30 / 50 / 100
- LEVEL sweep: 0.236 / 0.382 / 0.500 / 0.618
- RSI_LONG sweep: 30 / 40 / 50
- RSI_SHORT sweep: 50 / 60 / 70
