# RUN370 — Price Momentum Rotation with Breadth Confirmation

## Hypothesis

**Mechanism**: Portfolio breadth (% of coins in uptrend) rotates over time. When breadth reaches extreme readings (e.g., >80% coins above SMA), the market is likely exhausting its buying pressure — expect mean-reversion. When breadth drops to extreme lows (<20%), selling pressure is exhausted — expect bounce. Use breadth as a timing filter: only go SHORT when breadth is extreme high; only go LONG when breadth is extreme low.

**Why not duplicate**: No prior RUN uses breadth as an entry filter for individual coin trades. RUN238 uses momentum rotation. RUN268 uses momentum rotation long/short. This RUN specifically uses portfolio breadth to time entries — the distinct mechanism is cross-coin breadth as a timing signal for mean-reversion entries.

## Proposed Config Changes (config.rs)

```rust
// ── RUN370: Price Momentum Rotation with Breadth Confirmation ────────────────────────────
// breadth = % of coins where close > SMA(close, period)
// extreme_high_breadth = breadth > BREADTH_HIGH (exhausted upside)
// extreme_low_breadth = breadth < BREADTH_LOW (exhausted downside)
// LONG: breadth < BREADTH_LOW AND coin RSI < RSI_OVERSOLD
// SHORT: breadth > BREADTH_HIGH AND coin RSI > RSI_OVERBOUGHT

pub const MOM_ROT_BREADTH_ENABLED: bool = true;
pub const MOM_ROT_BREADTH_SMA_PERIOD: usize = 20;
pub const MOM_ROT_BREADTH_HIGH: f64 = 80.0;   // above this = market wide bullish exhaustion
pub const MOM_ROT_BREADTH_LOW: f64 = 20.0;    // below this = market wide selling exhaustion
pub const MOM_ROT_BREADTH_RSI_PERIOD: usize = 14;
pub const MOM_ROT_BREADTH_RSI_OVERSOLD: f64 = 35.0;
pub const MOM_ROT_BREADTH_RSI_OVERBOUGHT: f64 = 65.0;
pub const MOM_ROT_BREADTH_SL: f64 = 0.005;
pub const MOM_ROT_BREADTH_TP: f64 = 0.004;
pub const MOM_ROT_BREADTH_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run370_1_mom_rot_breadth_backtest.py)
2. **Walk-forward** (run370_2_mom_rot_breadth_wf.py)
3. **Combined** (run370_3_combined.py)

## Out-of-Sample Testing

- SMA_PERIOD sweep: 15 / 20 / 30
- BREADTH_HIGH sweep: 70 / 80 / 90
- BREADTH_LOW sweep: 10 / 20 / 30
