# RUN319 — Elder Ray Bull/Bear Power with EMA Filter: Institutional Money Detection

## Hypothesis

**Mechanism**: Elder Ray measures the buying pressure (Bull Power) and selling pressure (Bear Power) by comparing each bar's high/low to an EMA. Bull Power = high - EMA (how far buyers pushed above average). Bear Power = low - EMA (how far sellers pushed below average). When EMA is trending up AND Bull Power is making higher lows → strong institutional buying. When EMA is trending down AND Bear Power is making lower highs → strong institutional selling. Use EMA direction as a filter to avoid counter-trend trades.

**Why not duplicate**: RUN122 uses Elder Ray Bull Power as a confirmation filter. RUN205 uses Elder Ray Index (ERI), which normalizes bull/bear power by ATR. This RUN specifically combines both Elder Ray components with EMA trend direction as the primary filter — the distinct mechanism is using Bear Power lows vs Bull Power highs together for a complete picture.

## Proposed Config Changes (config.rs)

```rust
// ── RUN319: Elder Ray Bull/Bear Power with EMA Filter ─────────────────────────
// bull_power = high - EMA(close, period)
// bear_power = low - EMA(close, period)
// ema_trend_up = EMA(close, period) > EMA(close, period)[1]
// ema_trend_down = EMA(close, period) < EMA(close, period)[1]
// LONG: ema_trend_up AND bull_power crosses above 0 (buyers overcame sellers)
// SHORT: ema_trend_down AND bear_power crosses below 0 (sellers overcame buyers)
// Confirm: bull_power > 0 AND rising for LONG; bear_power < 0 AND falling for SHORT

pub const ELDER_RAY_ENABLED: bool = true;
pub const ELDER_RAY_PERIOD: usize = 13;
pub const ELDER_RAY_SL: f64 = 0.005;
pub const ELDER_RAY_TP: f64 = 0.004;
pub const ELDER_RAY_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run319_1_elder_ray_backtest.py)
2. **Walk-forward** (run319_2_elder_ray_wf.py)
3. **Combined** (run319_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 13 / 20
