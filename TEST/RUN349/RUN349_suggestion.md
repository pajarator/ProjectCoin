# RUN349 — CCI Percentile Rank: Adaptive Commodity Channel Index

## Hypothesis

**Mechanism**: CCI (Commodity Channel Index) measures price deviation from its SMA in units of mean deviation. Instead of fixed overbought/oversold thresholds (±100/±200), rank CCI against its own historical distribution. When CCI rank is at bottom 10% → historically oversold → bounce LONG. When CCI rank is at top 10% → historically overbought → reversal SHORT. Percentile ranking makes CCI adaptive to each coin's own historical range.

**Why not duplicate**: RUN197 uses CCI with fixed thresholds. RUN113 uses CCI as a confirmation filter. RUN262 uses RSI percentile rank. RUN267 uses Williams %R percentile. This RUN specifically applies percentile ranking to CCI — the adaptive framing is what distinguishes it from all prior CCI approaches.

## Proposed Config Changes (config.rs)

```rust
// ── RUN349: CCI Percentile Rank ───────────────────────────────────────────────
// cci = (typical_price - SMA(typical_price, period)) / (mean_deviation * CONST)
// cci_rank = percentile_rank(cci, lookback)
// LONG: cci_rank < CCI_OVERSOLD (bottom percentile = oversold)
// SHORT: cci_rank > CCI_OVERBOUGHT (top percentile = overbought)
// Confirmation: require CCI to cross its zero line

pub const CCI_PCT_ENABLED: bool = true;
pub const CCI_PCT_PERIOD: usize = 20;
pub const CCI_PCT_LOOKBACK: usize = 100;
pub const CCI_PCT_OVERSOLD: f64 = 10.0;    // bottom 10th percentile
pub const CCI_PCT_OVERBOUGHT: f64 = 90.0;  // top 10th percentile
pub const CCI_PCT_SL: f64 = 0.005;
pub const CCI_PCT_TP: f64 = 0.004;
pub const CCI_PCT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run349_1_cci_pct_backtest.py)
2. **Walk-forward** (run349_2_cci_pct_wf.py)
3. **Combined** (run349_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- LOOKBACK sweep: 50 / 100 / 200
- OVERSOLD sweep: 5 / 10 / 15
- OVERBOUGHT sweep: 85 / 90 / 95
