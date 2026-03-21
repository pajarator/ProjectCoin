# RUN306 — Williams %R Multi-Timeframe Confluence: 15m/1h/4h Alignment

## Hypothesis

**Mechanism**: Williams %R measures where close is relative to the N-period high-low range. When Williams %R is oversold (<-80) on 15m AND 1h AND 4h simultaneously → overwhelming bearish exhaustion → strong LONG bounce. When Williams %R is overbought (>-20) on all three timeframes → strong SHORT. All three must agree for a signal — this is a high-conviction mean-reversion setup.

**Why not duplicate**: RUN237 uses Williams %R + EMA filter (single timeframe). RUN199 uses Stochastic RSI. RUN246 uses multi-timeframe RSI. RUN270 uses VW Stochastic. No RUN uses Williams %R across multiple timeframes with mandatory alignment for a high-conviction signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN306: Williams %R Multi-Timeframe Confluence ───────────────────────────
// williams_r(period) = (highest - close) / (highest - lowest) * -100
// LONG: wr_15m < -80 AND wr_1h < -80 AND wr_4h < -80 (all oversold)
// SHORT: wr_15m > -20 AND wr_1h > -20 AND wr_4h > -20 (all overbought)
// Confirmation: at least 2 of 3 timeframes agree (relaxed mode) vs all 3 (strict)
// Hold until any timeframe exits extreme zone

pub const WR_MTF_ENABLED: bool = true;
pub const WR_MTF_PERIOD: usize = 14;
pub const WR_MTF_OVERSOLD: f64 = -80.0;
pub const WR_MTF_OVERBOUGHT: f64 = -20.0;
pub const WR_MTF_REQUIRE_ALL: bool = true;    // true = all 3 TF must agree; false = 2 of 3
pub const WR_MTF_SL: f64 = 0.005;
pub const WR_MTF_TP: f64 = 0.004;
pub const WR_MTF_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run306_1_wr_mtf_backtest.py)
2. **Walk-forward** (run306_2_wr_mtf_wf.py)
3. **Combined** (run306_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- OVERSOLD sweep: -85 / -80 / -75
- OVERBOUGHT sweep: -25 / -20 / -15
- REQUIRE_ALL sweep: true / false
