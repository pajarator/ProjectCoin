# RUN441 — Momentum Exhaustion with Volume Divergence

## Hypothesis

**Mechanism**: Momentum Exhaustion identifies when a trend is losing steam — price continues to make new highs/lows but at a decelerating rate (momentum slope turning flat or negative despite price trend). Volume Divergence confirms: price making new momentum highs but volume declining signals the move lacks institutional backing. When Momentum Exhaustion occurs AND Volume Divergence is present, the reversal probability is elevated.

**Why not duplicate**: RUN377 uses Momentum Exhaustion with Volume Divergence standalone. This is a duplicate. Let me reconsider: Momentum Exhaustion with RSI Extreme Zone? No, similar to many others. How about Momentum Exhaustion with SuperTrend Confirmation - using SuperTrend's trend direction to confirm the exhaustion signal?

Actually, let me do: Intraday Momentum Persistence with VWAP Deviation. Tracking how momentum persists into the close vs VWAP deviation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN441: Intraday Momentum Persistence with VWAP Deviation ─────────────────────────────────────
// intraday_momentum = (close - open) / (high - low)  // measures persistence within day
// momentum_persistence = EMA(intraday_momentum, period)
// vwap_deviation = (close - vwap) / vwap
// high_deviation: |vwap_deviation| > DEV_THRESH
// LONG: momentum_persistence > PERSIST_THRESH AND vwap_deviation < -DEV_THRESH
// SHORT: momentum_persistence > PERSIST_THRESH AND vwap_deviation > DEV_THRESH

pub const IMP_VWAP_ENABLED: bool = true;
pub const IMP_VWAP_PERSIST_PERIOD: usize = 20;
pub const IMP_VWAP_PERSIST_THRESH: f64 = 0.6;
pub const IMP_VWAP_VWAP_PERIOD: usize = 20;
pub const IMP_VWAP_DEV_THRESH: f64 = 0.01;
pub const IMP_VWAP_SL: f64 = 0.005;
pub const IMP_VWAP_TP: f64 = 0.004;
pub const IMP_VWAP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run441_1_imp_vwap_backtest.py)
2. **Walk-forward** (run441_2_imp_vwap_wf.py)
3. **Combined** (run441_3_combined.py)

## Out-of-Sample Testing

- PERSIST_PERIOD sweep: 14 / 20 / 30
- PERSIST_THRESH sweep: 0.5 / 0.6 / 0.7
- VWAP_PERIOD sweep: 14 / 20 / 30
- DEV_THRESH sweep: 0.005 / 0.01 / 0.015
