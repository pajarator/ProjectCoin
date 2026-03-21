# RUN259 — Dollar Volume Imbalance: Abnormal Buying/Selling Pressure

## Hypothesis

**Mechanism**: Dollar Volume = close × volume. It measures the actual dollar value traded, not just the number of shares/coins. When dollar volume on a bullish bar is 3× the average dollar volume → abnormal buying pressure. When dollar volume on a bearish bar is 3× average → abnormal selling pressure. Imbalances predict mean reversion.

**Why not duplicate**: No prior RUN uses Dollar Volume. All prior volume RUNs use raw coin volume. Dollar volume is distinct because it accounts for price level — a trade of 1 BTC at $60k is $60k not 1 BTC.

## Proposed Config Changes (config.rs)

```rust
// ── RUN259: Dollar Volume Imbalance ──────────────────────────────────────
// dollar_volume = close × volume
// dv_ma = SMA(dollar_volume, period)
// bullish_bar = close > open
// bearish_bar = close < open
// LONG: bullish_bar AND dollar_volume > dv_ma × 2.0
// SHORT: bearish_bar AND dollar_volume > dv_ma × 2.0

pub const DV_IMBALANCE_ENABLED: bool = true;
pub const DV_IMBALANCE_PERIOD: usize = 20;   // MA lookback
pub const DV_IMBALANCE_MULT: f64 = 2.0;     // imbalance threshold
pub const DV_IMBALANCE_SL: f64 = 0.005;
pub const DV_IMBALANCE_TP: f64 = 0.004;
pub const DV_IMBALANCE_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn dollar_volume_imbalance(closes: &[f64], volumes: &[f64], period: usize) -> (bool, bool) {
    let n = closes.len().min(volumes.len());
    if n < period {
        return (false, false);
    }

    let mut dv_sum = 0.0;
    for i in (n-period)..n {
        dv_sum += closes[i] * volumes[i];
    }
    let dv_ma = dv_sum / (period as f64);

    let current_dv = closes[n-1] * volumes[n-1];
    let bullish = closes[n-1] > closes[n-2];
    let bearish = closes[n-1] < closes[n-2];

    let bullish_imbalance = bullish && current_dv > dv_ma * 2.0;
    let bearish_imbalance = bearish && current_dv > dv_ma * 2.0;

    (bullish_imbalance, bearish_imbalance)
}
```

---

## Validation Method

1. **Historical backtest** (run259_1_dv_imb_backtest.py)
2. **Walk-forward** (run259_2_dv_imb_wf.py)
3. **Combined** (run259_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- MULT sweep: 1.5 / 2.0 / 3.0
