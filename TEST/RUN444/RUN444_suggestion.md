# RUN444 — Bollinger Band Width with Stochastic Momentum Divergence

## Hypothesis

**Mechanism**: Bollinger Band Width (BBW) measures the narrowing or widening of the bands — when narrow, volatility is compressed; when wide, volatility is expanded. Stochastic Momentum Divergence identifies when price and Stochastic disagree — price making new highs but Stochastic failing to confirm. When BBW is at extreme narrowness (squeeze) AND Stochastic shows divergence, the squeeze is building toward a directional move that Stochastic divergence confirms.

**Why not duplicate**: RUN398 uses Stochastic with BB Squeeze (uses squeeze as regime, Stochastic crossover as signal). RUN347 uses BB Width with KST Confluence. This RUN specifically uses Stochastic Momentum Divergence (price-Stochastic disagreement) to confirm the squeeze, not Stochastic crossover or KST.

## Proposed Config Changes (config.rs)

```rust
// ── RUN444: Bollinger Band Width with Stochastic Momentum Divergence ─────────────────────────────────────
// bb_width = (bb_upper - bb_lower) / bb_middle
// extreme_narrow: bb_width < BB_WIDTH_THRESH (squeeze state)
// stoch_momentum = stochastic_oscillator(close, period)
// stoch_divergence: price makes new high but stoch doesn't confirm (bearish div)
// LONG: extreme_narrow AND stoch_divergence bullish (price low, stoch higher low)
// SHORT: extreme_narrow AND stoch_divergence bearish (price high, stoch lower high)

pub const BBW_STOCH_DIV_ENABLED: bool = true;
pub const BBW_STOCH_DIV_BB_PERIOD: usize = 20;
pub const BBW_STOCH_DIV_BB_STD: f64 = 2.0;
pub const BBW_STOCH_DIV_WIDTH_THRESH: f64 = 0.05;
pub const BBW_STOCH_DIV_STOCH_PERIOD: usize = 14;
pub const BBW_STOCH_DIV_STOCH_SMOOTH: usize = 3;
pub const BBW_STOCH_DIV_SL: f64 = 0.005;
pub const BBW_STOCH_DIV_TP: f64 = 0.004;
pub const BBW_STOCH_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run444_1_bbw_stoch_div_backtest.py)
2. **Walk-forward** (run444_2_bbw_stoch_div_wf.py)
3. **Combined** (run444_3_combined.py)

## Out-of-Sample Testing

- BB_PERIOD sweep: 15 / 20 / 30
- BB_STD sweep: 1.5 / 2.0 / 2.5
- WIDTH_THRESH sweep: 0.04 / 0.05 / 0.06
- STOCH_PERIOD sweep: 10 / 14 / 21
