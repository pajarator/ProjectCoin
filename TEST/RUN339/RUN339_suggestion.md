# RUN339 — Dual RSI Confluence: Fast-Slow Oscillator Momentum

## Hypothesis

**Mechanism**: Use two RSI periods simultaneously — a fast RSI (e.g., 7) for early detection and a slow RSI (e.g., 21) for confirmation. Both must align: fast RSI oversold AND slow RSI oversold → strong mean-reversion LONG. Both overbought → strong mean-reversion SHORT. Single-timeframe RSI gives false signals; requiring both to agree increases conviction. When fast RSI diverges from slow RSI → weakening momentum.

**Why not duplicate**: RUN199 uses Stochastic RSI. RUN262 uses RSI percentile rank. RUN246 uses multi-timeframe RSI. This RUN uses two RSI periods simultaneously with a confluence requirement — the dual-period approach is distinct because it captures both early momentum (fast) and confirmed exhaustion (slow).

## Proposed Config Changes (config.rs)

```rust
// ── RUN339: Dual RSI Confluence ───────────────────────────────────────────────
// rsi_fast = RSI(close, FAST_PERIOD)
// rsi_slow = RSI(close, SLOW_PERIOD)
// LONG: rsi_fast < RSI_OVERSOLD AND rsi_slow < RSI_OVERSOLD
// SHORT: rsi_fast > RSI_OVERBOUGHT AND rsi_slow > RSI_OVERBOUGHT
// Divergence: fast RSI making higher low while slow RSI making lower low = weakening bearish

pub const DUAL_RSI_ENABLED: bool = true;
pub const DUAL_RSI_FAST: usize = 7;
pub const DUAL_RSI_SLOW: usize = 21;
pub const DUAL_RSI_OVERSOLD: f64 = 35.0;
pub const DUAL_RSI_OVERBOUGHT: f64 = 65.0;
pub const DUAL_RSI_SL: f64 = 0.005;
pub const DUAL_RSI_TP: f64 = 0.004;
pub const DUAL_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run339_1_dual_rsi_backtest.py)
2. **Walk-forward** (run339_2_dual_rsi_wf.py)
3. **Combined** (run339_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 5 / 7 / 10
- SLOW sweep: 14 / 21 / 28
- OVERSOLD sweep: 30 / 35 / 40
- OVERBOUGHT sweep: 60 / 65 / 70
