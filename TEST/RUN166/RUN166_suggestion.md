# RUN166 — Stochastic Divergence Entry: Hidden and Classical Price-Stochastic Divergence

## Hypothesis

**Mechanism**: Stochastic divergence (classical or hidden) between price and %K/%D is a leading indicator of trend exhaustion. Classical bearish divergence: price makes higher high, stochastic makes lower high → reversal down. Hidden bearish: price makes lower high, stochastic makes higher high → continuation down. Same inversed for bullish. COINCLAW's scalp uses stochastic crossover but not divergence patterns.

**Why not duplicate**: RUN61 (RSI Divergence Confirmation) was proposed but unexecuted. RUN132 (RSI Divergence Confirmation) also proposed. No prior RUN uses stochastic divergence specifically. Divergence is a leading signal vs crossover is a coincident signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN166: Stochastic Divergence Entry ────────────────────────────────
// Classical bearish: price HH, stoch LH → SHORT
// Hidden bearish: price LH, stoch HH → SHORT
// Same inversed for bullish
// Confirmed by RSI or volume

pub const STOCH_DIV_ENABLED: bool = true;
pub const STOCH_DIV_LOOKBACK: usize = 8;    // bars to look for divergence
pub const STOCH_DIV_CONFIRM_RSI: bool = true;  // require RSI alignment
pub const STOCH_DIV_SL: f64 = 0.004;
pub const STOCH_DIV_TP: f64 = 0.003;
pub const STOCH_DIV_MAX_HOLD: u32 = 16;
```

Add in `engine.rs`:

```rust
/// Detect stochastic divergence in last STOCH_DIV_LOOKBACK bars
fn detect_stoch_divergence(ind: &Ind15m) -> Option<(Direction, &'static str)> {
    let stoch = (ind.stoch_k, ind.stoch_d);
    if stoch.0.is_nan() || stoch.1.is_nan() { return None; }
    // Simplified: check if stoch_k and price are moving in opposite directions
    // over the lookback window. Full implementation requires price/Stoch history.
    // Pattern: stoch_k making lower highs while price making higher highs = bearish div
    // stoch_k making higher lows while price making lower lows = bullish div
    let dir = detect_divergence_pattern(ind)?;
    Some((dir, "stoch_div"))
}

fn detect_divergence_pattern(ind: &Ind15m) -> Option<Direction> {
    // Requires rolling arrays of price and stoch_k over lookback window
    // For now, detect 3-bar momentum divergence:
    // Bearish: last 3 bars: price net positive, stoch net negative
    // Bullish: last 3 bars: price net negative, stoch net positive
    unimplemented!("requires rolling history of stoch_k and price")
}
```

Note: This RUN requires extending `Ind15m` with rolling arrays for price and stochastic history.

---

## Validation Method

1. **Historical backtest** (run166_1_stochdiv_backtest.py): 18 coins, identify divergence patterns
2. **Walk-forward** (run166_2_stochdiv_wf.py): 3-window walk-forward
3. **Combined** (run166_3_combined.py): vs baseline

## Out-of-Sample Testing

- LOOKBACK sweep: 4 / 8 / 12 bars
- CONFIRM_RSI: true/false
