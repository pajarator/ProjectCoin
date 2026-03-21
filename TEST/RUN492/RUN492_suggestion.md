# RUN492 — CCI with Volume Weighted EMA

## Hypothesis

**Mechanism**: CCI (Commodity Channel Index) measures deviation from average price, catching cyclical turns. Volume Weighted EMA gives more weight to prices with higher volume, making it more responsive to volume-backed price moves. When CCI signals AND price is aligned with the Volume Weighted EMA direction, entries have both deviation-based timing and volume-weighted trend confirmation.

**Why not duplicate**: RUN452 uses CCI with RSI Confluence. This RUN uses Volume Weighted EMA instead — distinct mechanism is VW-EMA as a trend direction filter versus RSI oscillator confirmation. VW-EMA directly incorporates volume into trend direction.

## Proposed Config Changes (config.rs)

```rust
// ── RUN492: CCI with Volume Weighted EMA ─────────────────────────────────
// cci: commodity_channel_index measuring deviation from average
// cci_cross: cci crosses above/below 100 or -100 threshold
// vwema: volume_weighted_ema giving more weight to high_volume prices
// price_above_vwema: price position relative to vwema
// LONG: cci_cross bullish AND price > vwema
// SHORT: cci_cross bearish AND price < vwema

pub const CCI_VWEMA_ENABLED: bool = true;
pub const CCI_VWEMA_CCI_PERIOD: usize = 20;
pub const CCI_VWEMA_CCI_OVERSOLD: f64 = -100.0;
pub const CCI_VWEMA_CCI_OVERBOUGHT: f64 = 100.0;
pub const CCI_VWEMA_VWEMA_PERIOD: usize = 20;
pub const CCI_VWEMA_SL: f64 = 0.005;
pub const CCI_VWEMA_TP: f64 = 0.004;
pub const CCI_VWEMA_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run492_1_cci_vwema_backtest.py)
2. **Walk-forward** (run492_2_cci_vwema_wf.py)
3. **Combined** (run492_3_combined.py)

## Out-of-Sample Testing

- CCI_PERIOD sweep: 14 / 20 / 30
- CCI_OVERSOLD sweep: -120 / -100 / -80
- CCI_OVERBOUGHT sweep: 80 / 100 / 120
- VWEMA_PERIOD sweep: 15 / 20 / 25
