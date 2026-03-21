# RUN397 — CCI with Volume Divergence Filter

## Hypothesis

**Mechanism**: The Commodity Channel Index (CCI) measures how far price deviates from its statistical mean. Extremely high/low CCI values (above +100 or below -100) indicate overbought/oversold conditions. However, not all extreme CCI readings reverse — sometimes price continues trending. Add volume divergence as a filter: when CCI reaches extreme AND volume shows a divergence (price makes a new extreme but volume doesn't confirm), the signal has a higher probability of reversal. Volume divergence confirms the move lacks institutional backing.

**Why not duplicate**: RUN349 uses CCI Percentile Rank standalone. RUN324 uses Stochastic RSI Divergence with Volume. RUN377 uses Momentum Exhaustion with Volume Divergence. This RUN specifically uses CCI extreme readings with volume divergence confirmation — the distinct mechanism is using CCI (a mean-reversion indicator) with volume divergence as the confirming signal, rather than CCI percentile ranking.

## Proposed Config Changes (config.rs)

```rust
// ── RUN397: CCI with Volume Divergence Filter ───────────────────────────────────────
// cci = (typical_price - SMA(typical_price, period)) / (0.015 * mean_deviation)
// cci_extreme: cci > +100 (overbought) or cci < -100 (oversold)
// volume_divergence: price makes new high/low but volume doesn't confirm
// LONG: cci < -100 AND volume_divergence bullish (price low, volume not confirming)
// SHORT: cci > +100 AND volume_divergence bearish (price high, volume not confirming)

pub const CCI_VOL_DIV_ENABLED: bool = true;
pub const CCI_VOL_DIV_CCI_PERIOD: usize = 14;
pub const CCI_VOL_DIV_CCI_OVERSOLD: f64 = -100.0;
pub const CCI_VOL_DIV_CCI_OVERBOUGHT: f64 = 100.0;
pub const CCI_VOL_DIV_VOL_PERIOD: usize = 20;   // volume SMA lookback
pub const CCI_VOL_DIV_SL: f64 = 0.005;
pub const CCI_VOL_DIV_TP: f64 = 0.004;
pub const CCI_VOL_DIV_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run397_1_cci_vol_div_backtest.py)
2. **Walk-forward** (run397_2_cci_vol_div_wf.py)
3. **Combined** (run397_3_combined.py)

## Out-of-Sample Testing

- CCI_PERIOD sweep: 10 / 14 / 21
- CCI_OVERSOLD sweep: -80 / -100 / -120
- CCI_OVERBOUGHT sweep: 80 / 100 / 120
- VOL_PERIOD sweep: 15 / 20 / 30
