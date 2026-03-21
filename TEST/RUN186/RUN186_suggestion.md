# RUN186 — VWAP Volume Profile: High-Volume Node Detection as Support/Resistance

## Hypothesis

**Mechanism**: Standard VWAP is a single line. VWAP Volume Profile tracks *where* volume was concentrated — high-volume nodes (HVNs) act as sticky support/resistance zones; low-volume nodes (LVNs) act as fast rejection zones. When price retraces to an HVN from above → LONG entry (bulls defended the zone). When price gets rejected at an LVN → mean reversion back through the zone.

**Why not duplicate**: No prior RUN uses volume-weighted price levels. All VWAP RUNs use the single VWAP line, not the profile distribution. Volume profile is a fundamentally different data product.

## Proposed Config Changes (config.rs)

```rust
// ── RUN186: VWAP Volume Profile ────────────────────────────────────────
// hvn_threshold = volume at node > 2× average node volume
// lv_threshold = volume at node < 0.5× average node volume
// Profile computed over lookback window (e.g., 100 bars)
// HVN = zone of contention (consolidation), LVN = fast money (trending)

// When price crosses above LVN from below → bullish, ride to next HVN
// When price crosses below LVN from above → bearish, ride to next HVN below
// When price rejects at HVN → mean-revert through the HVN

pub const VOLPROF_ENABLED: bool = true;
pub const VOLPROF_WINDOW: usize = 100;      // bars to compute profile over
pub const VOLPROF_HVN_MULT: f64 = 2.0;       // HVN = node_vol > avg × this
pub const VOLPROF_LVN_MULT: f64 = 0.5;       // LVN = node_vol < avg × this
pub const VOLPROF_BUCKETS: usize = 50;        // price buckets for volume distribution
pub const VOLPROF_SL: f64 = 0.005;
pub const VOLPROF_TP: f64 = 0.004;
pub const VOLPROF_MAX_HOLD: u32 = 36;
```

Add to `CoinState` in `state.rs`:

```rust
pub vol_profile_hvns: Vec<f64>,  // high-volume node price levels
pub vol_profile_lvns: Vec<f64>,  // low-volume node price levels
pub vol_profile_poc: f64,          // point of control (max volume price)
```

Add in `indicators.rs`:

```rust
pub fn volume_profile(closes: &[f64], volumes: &[f64], buckets: usize) -> (Vec<f64>, Vec<f64>, f64) {
    if closes.is_empty() || volumes.is_empty() {
        return (vec![], vec![], 0.0);
    }

    let min_p = closes.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_p = closes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_p - min_p;
    if range <= 0.0 {
        return (vec![], vec![], min_p);
    }

    let bucket_size = range / (buckets as f64);
    let mut bucket_vols = vec![0.0; buckets];
    let mut bucket_prices = vec![0.0; buckets];

    for i in 0..closes.len() {
        let idx = ((closes[i] - min_p) / bucket_size).floor() as usize;
        let idx = idx.min(buckets - 1);
        bucket_vols[idx] += volumes[i];
        bucket_prices[idx] = closes[i];
    }

    let avg_vol = bucket_vols.iter().sum::<f64>() / (buckets as f64);

    let mut hvns = Vec::new();
    let mut lvns = Vec::new();

    for i in 0..buckets {
        if bucket_vols[i] > avg_vol * 2.0 {
            hvns.push(bucket_prices[i]);
        } else if bucket_vols[i] < avg_vol * 0.5 && bucket_vols[i] > 0.0 {
            lvns.push(bucket_prices[i]);
        }
    }

    let poc_idx = bucket_vols.iter().enumerate().fold(0, |acc, (i, &v)|
        if v > bucket_vols[acc] { i } else { acc });
    let poc = bucket_prices[poc_idx];

    (hvns, lvns, poc)
}
```

---

## Validation Method

1. **Historical backtest** (run186_1_volprof_backtest.py)
2. **Walk-forward** (run186_2_volprof_wf.py)
3. **Combined** (run186_3_combined.py)

## Out-of-Sample Testing

- WINDOW sweep: 50 / 100 / 200 bars
- HVN_MULT sweep: 1.5 / 2.0 / 3.0
- LVN_MULT sweep: 0.3 / 0.5 / 0.7
- BUCKETS sweep: 25 / 50 / 100
