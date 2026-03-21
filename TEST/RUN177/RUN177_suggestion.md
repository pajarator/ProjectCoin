# RUN177 — Per-Coin RSI Period Optimization: Rolling OOS SharpeMaximizing RSI Period

## Hypothesis

**Mechanism**: COINCLAW uses RSI(14) universally. But different coins have different natural frequencies — SHIB might benefit from RSI(7) while BTC from RSI(20). A rolling OOS Sharpe selector chooses the best RSI period per coin from {7, 14, 21, 28} each week, adapting to market conditions.

**Why not duplicate**: No prior RUN optimizes RSI period per coin. RUN171 optimizes SMA window. This applies the same rolling-Sharpe selection to RSI period.

## Proposed Config Changes (config.rs)

```rust
// ── RUN177: Per-Coin RSI Period Optimization ─────────────────────────────
// Candidates: RSI(7), RSI(14), RSI(21), RSI(28)
// Each week: pick period with best OOS Sharpe from prior 4 weeks
// Update active period per coin weekly

pub const RSI_OPT_ENABLED: bool = true;
pub const RSI_OPT_WINDOW: usize = 240;    // 240 bars (~60h) for Sharpe calc
pub const RSI_OPT_UPDATE_BARS: usize = 96; // update every 96 bars (~24h)
pub const RSI_OPT_CANDIDATES: [u8; 4] = [7, 14, 21, 28];
```

Add to `CoinState` in `state.rs`:

```rust
pub active_rsi_period: u8,          // current active RSI period
pub rsi_period_sharpes: [f64; 4], // Sharpe for each candidate period
```

---

## Validation Method

1. **Historical backtest** (run177_1_rsiopt_backtest.py)
2. **Walk-forward** (run177_2_rsiopt_wf.py)
3. **Combined** (run177_3_combined.py)

## Out-of-Sample Testing

- WINDOW sweep: 120 / 240 / 480 bars
- UPDATE_FREQ sweep: 48 / 96 / 192 bars
