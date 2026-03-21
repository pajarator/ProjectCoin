# RUN171 — Rolling Sharpe-Maximizing Indicator Window: Adaptive Lookback Per Coin

## Hypothesis

**Mechanism**: COINCLAW uses fixed lookback windows for all indicators (e.g., SMA20, RSI14). But the optimal window varies by coin and market regime. A rolling Sharpe-maximizing window selector would dynamically choose the best SMA period (e.g., SMA 10 vs 20 vs 50) for each coin based on recent performance, improving signal quality.

**Why not duplicate**: No prior RUN adapts indicator lookback windows dynamically. All prior RUNs use fixed parameters.

## Proposed Config Changes (config.rs)

```rust
// ── RUN171: Rolling Sharpe-Maximizing SMA Window ───────────────────────
// For each coin: track Sharpe ratio for SMA10, SMA20, SMA50 over rolling 60-bar window
// Use whichever SMA produced best Sharpe for entry/exit decisions

pub const SHARPE_WIN_ENABLED: bool = true;
pub const SHARPE_WIN_SIZE: usize = 60;      // rolling window for Sharpe calculation
pub const SHARPE_MIN_TRADES: usize = 10;     // minimum trades before switching
```

Add to `CoinState` in `state.rs`:

```rust
pub sma10_returns: Vec<f64>,
pub sma20_returns: Vec<f64>,
pub sma50_returns: Vec<f64>,
pub active_sma: &'static str,  // "sma10", "sma20", or "sma50"
```

Compute best SMA:

```rust
fn best_sma_window(returns: &[f64]) -> &'static str {
    if returns.len() < SHARPE_MIN_TRADES { return "sma20"; }  // default
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    let std = variance.sqrt();
    let sharpe = if std > 0.0 { mean / std } else { 0.0 };
    // Track sharpe per window and pick best
    // ...
    "sma20"  // placeholder
}
```

Modify `long_entry` to use active SMA:

```rust
// Instead of: if ind.p > ind.sma20
// Use: let ma = match active_sma { "sma10" => ind.sma10, "sma50" => ind.sma50, _ => ind.sma20 };
// if ind.p > ma { return false; }
```

---

## Validation Method

1. **Historical backtest** (run171_1_sharpewin_backtest.py)
2. **Walk-forward** (run171_2_sharpewin_wf.py)
3. **Combined** (run171_3_combined.py)

## Out-of-Sample Testing

- WIN_SIZE sweep: 40 / 60 / 100 bars
- MIN_TRADES sweep: 5 / 10 / 20
