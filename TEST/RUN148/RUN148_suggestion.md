# RUN148 — Per-Coin Minimum Hold Optimization: Rolling-Window Adaptive Hold

## Hypothesis

**Mechanism**: COINCLAW uses a global `MIN_HOLD_CANDLES = 2` for all coins and strategies. But optimal minimum hold time is coin-specific and regime-dependent. Coins with faster mean-reversion cycles (XRP, DOGE) benefit from shorter holds; coins with slower cycles (BTC, ETH) need longer holds. A rolling 60-bar window tracks which MIN_HOLD value (1/2/3/4 bars) produces the highest Sharpe for each coin, and adapts dynamically.

**Why this is not a duplicate**: RUN47 (Per-Strategy Optimal MIN_HOLD) was proposed but unexecuted. It focused on per-strategy (not per-coin) hold times. This RUN improves on it by: (1) per-coin adaptation rather than per-strategy, (2) rolling window rather than fixed train/test split, and (3) using Sharpe ratio (not just WR) as the selection metric.

**Why it could work**: Different coins have different mean-reversion speeds due to market cap, liquidity, and vol characteristics. A micro-cap like SHIB reverts faster than BTC. A rolling adaptive hold captures this heterogeneity. If the best MIN_HOLD differs by >1 bar across coins, this is a direct improvement over the global default.

---

## Proposed Config Changes (config.rs)

```rust
// ── RUN8: Per-Coin Adaptive Minimum Hold ──────────────────────────────
// Each coin maintains a rolling record of the last 60 bars' trade outcomes
// by MIN_HOLD value. The effective MIN_HOLD per coin = argmax Sharpe over {1,2,3,4}.
// Updated every BAR, not every trade (smoother adaptation).

pub const ADAPTIVE_HOLD_ENABLED: bool = true;
pub const ADAPTIVE_HOLD_WINDOW: usize = 60;    // rolling window in bars
pub const ADAPTIVE_HOLD_CANDLES: [u32; 4] = [1, 2, 3, 4];  // candidates
pub const ADAPTIVE_HOLD_MIN_TRADES: usize = 10;  // minimum trades before switching
```

Add to `CoinState` in state.rs:
```rust
// Per-MIN_HOLD trade outcome tracking for adaptive selection
pub hold1_returns: Vec<f64>,   // PnL % of trades exited at bar 1
pub hold2_returns: Vec<f64>,   // PnL % of trades exited at bar 2
pub hold3_returns: Vec<f64>,   // PnL % of trades exited at bar 3
pub hold4_returns: Vec<f64>,   // PnL % of trades exited at bar 4
```

Add helper in engine.rs:
```rust
/// Returns the optimal MIN_HOLD for a coin given recent trade performance.
/// Uses Sharpe ratio as selection metric; falls back to config default if insufficient trades.
fn adaptive_min_hold(returns: &[f64]) -> u32 {
    if returns.len() < config::ADAPTIVE_HOLD_MIN_TRADES {
        return config::MIN_HOLD_CANDLES;
    }
    let mut best_hold = config::MIN_HOLD_CANDLES;
    let mut best_sharpe = f64::NEG_INFINITY;
    for (i, h) in config::ADAPTIVE_HOLD_CANDLES.iter().enumerate() {
        let rets = match i {
            0 => &state.coins[ci].hold1_returns,
            1 => &state.coins[ci].hold2_returns,
            2 => &state.coins[ci].hold3_returns,
            3 => &state.coins[ci].hold4_returns,
            _ => continue,
        };
        if rets.len() < config::ADAPTIVE_HOLD_MIN_TRADES { continue; }
        let mean = rets.iter().sum::<f64>() / rets.len() as f64;
        let variance = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rets.len() as f64;
        let std = variance.sqrt();
        let sharpe = if std > 0.0 { mean / std } else { 0.0 };
        if sharpe > best_sharpe {
            best_sharpe = sharpe;
            best_hold = *h;
        }
    }
    best_hold
}
```

Modify `check_exit` in engine.rs to record outcomes by hold time and use adaptive hold:
```rust
// Instead of: if held >= config::MIN_HOLD_CANDLES
// Use: if held >= adaptive_min_hold(state, ci)
```

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 with global MIN_HOLD_CANDLES=2
- **Comparison**: same trades with per-coin adaptive MIN_HOLD

**Metrics to measure**:
- Per-coin optimal MIN_HOLD distribution (do they differ meaningfully?)
- Portfolio Sharpe improvement
- Hold-time distribution change (avg holds longer/shorter per coin)
- Frequency of hold-time switches (if too frequent → add hysteresis)

**Hypothesis**: Per-coin adaptive MIN_HOLD should improve portfolio Sharpe >10% vs global default. Coins like BTC/ETH should shift to longer holds (3-4), micro-caps to shorter holds (1-2).

---

## Validation Method

1. **Historical backtest** (run8_1_adaptivehold_backtest.py):
   - 18 coins, 1-year 15m data
   - Simulate COINCLAW v16 with global MIN_HOLD=2
   - Re-simulate with rolling per-coin adaptive hold
   - Record: per-coin optimal hold, Sharpe by hold value, switching frequency
   - Output: per-coin comparison, portfolio-level improvement

2. **Walk-forward** (run8_2_adaptivehold_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep ADAPTIVE_HOLD_WINDOW: 40 / 60 / 100 bars
   - Sweep ADAPTIVE_HOLD_MIN_TRADES: 5 / 10 / 20

3. **Combined comparison** (run8_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + adaptive_min_hold
   - Portfolio stats, per-coin hold distribution, Sharpe comparison

---

## Out-of-Sample Testing

- Window sweep: 40 / 60 / 100 bars
- MIN_TRADES sweep: 5 / 10 / 20
- Hysteresis: require >3pp Sharpe improvement before switching hold value
- OOS: final 4 months held out from all parameter selection
