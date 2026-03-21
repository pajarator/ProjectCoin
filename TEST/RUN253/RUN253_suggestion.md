# RUN253 — Market-Wide RSI Breadth: Percentage of Coins in Overbought/Oversold

## Hypothesis

**Mechanism**: Track the percentage of all 18 coins where RSI(14) is overbought (>70) or oversold (<30). When >60% of coins are RSI-overbought → market-wide froth → SHORT mean-reversion. When >60% of coins are RSI-oversold → market-wide capitulation → LONG mean-reversion. The 60% threshold acts as a contrarian indicator.

**Why not duplicate**: No prior RUN uses RSI breadth across all coins. All prior breadth RUNs use SMA above/below (RUN227). RSI breadth is distinct because it measures *momentum extremity* across the market, not just price position.

## Proposed Config Changes (config.rs)

```rust
// ── RUN253: Market-Wide RSI Breadth ───────────────────────────────────────
// rsi_breadth_overbought = coins with RSI > 70 / 18 × 100
// rsi_breadth_oversold = coins with RSI < 30 / 18 × 100
// breadth > 60% → contrarian signal
// breadth > 70% → strong contrarian (higher conviction)

pub const RSI_BREADTH_ENABLED: bool = true;
pub const RSI_BREADTH_THRESH: f64 = 60.0;   // threshold for signal
pub const RSI_BREADTH_STRONG: f64 = 70.0;   // strong signal threshold
pub const RSI_BREADTH_RSI_PERIOD: usize = 14;
```

Modify engine to reduce LONG entries when rsi_breadth_oversold > 60% (already oversold), and reduce SHORT entries when rsi_breadth_overbought > 60%.

---

## Validation Method

1. **Historical backtest** (run253_1_rsi_breadth_backtest.py)
2. **Walk-forward** (run253_2_rsi_breadth_wf.py)
3. **Combined** (run253_3_combined.py)

## Out-of-Sample Testing

- THRESH sweep: 50 / 60 / 70
- STRONG sweep: 60 / 70 / 80
- RSI_PERIOD sweep: 10 / 14 / 21
