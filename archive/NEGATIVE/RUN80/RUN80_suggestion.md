# RUN80 — Volume Imbalance Confirmation: On-Balance Volume Direction as Entry Filter

## Hypothesis

**Named:** `vol_imbalance_confirm`

**Mechanism:** COINCLAW currently uses volume only as a simple threshold (`vol > vol_ma * 1.2`) to confirm entries. But volume direction matters as much as volume magnitude: if price is falling but volume is dominated by selling (high sellVolume), that's a different signal than price falling on low volume (no conviction). Similarly, price rising on high buy volume is more constructive than price rising on mixed volume.

**Volume Imbalance Confirmation:**
- Track per-coin volume imbalance: `imb = (buyVol - sellVol) / totalVol`, normalized to [-1, +1]
- Estimate imbalance from candle characteristics: up candles = partial buy volume, down candles = partial sell volume
- For regime LONG entries: require `imb >= MIN_IMB_LONG` (e.g., +0.15) — market is absorbing buying pressure
- For regime SHORT entries: require `imb <= MAX_IMB_SHORT` (e.g., -0.15) — market is absorbing selling pressure
- Additionally: require `relative_vol >= VOL_IMB_CONFIRM_MIN` (both volume AND direction must confirm)
- Scalp trades are exempt (they already use 1m vol spikes)

**Why this is not a duplicate:**
- RUN10 (F6 filter) uses `dir_roc_3` and `avg_body_3` as pre-entry filters — these are price-based, not volume-based
- RUN54 (vol entry threshold) uses ATR percentile rank — not volume imbalance direction
- No prior RUN has used estimated buy/sell volume imbalance as an entry confirmation filter

**Mechanistic rationale:** Volume is the one indicator that cannot be faked over sustained periods. If price is falling but volume is dominated by selling, the move has conviction. If price is falling on declining volume, the move is likely to reverse. Volume imbalance confirmation adds a dimension of market depth that price-based signals miss.

---

## Proposed Config Changes

```rust
// RUN80: Volume Imbalance Confirmation
pub const VOL_IMB_CONFIRM_ENABLE: bool = true;
pub const VOL_IMB_WINDOW: usize = 20;         // lookback window for volume imbalance
pub const VOL_IMB_LONG_MIN: f64 = 0.15;        // minimum imb for LONG entries (0 to +1)
pub const VOL_IMB_SHORT_MAX: f64 = -0.15;      // maximum imb for SHORT entries (-1 to 0)
pub const VOL_IMB_REL_VOL_MIN: f64 = 1.20;    // relative volume minimum (same as existing vol filter)
```

**`indicators.rs` — Ind15m additions:**
```rust
pub struct Ind15m {
    // ... existing fields ...
    pub vol_imb: f64,       // volume imbalance: (buyVol - sellVol) / totalVol, range [-1, +1]
    pub rel_vol: f64,        // current vol / vol_ma (already partially available)
}
```

**`indicators.rs` — volume imbalance computation:**
```rust
/// Estimate volume imbalance from OHLCV candles.
/// For each candle: if close > open → up candle → portion of volume is buy
///                  if close < open → down candle → portion of volume is sell
/// Use body ratio as proxy for buy/sell split within the candle.
/// Returns (buy_vol, sell_vol, imbalance) where imb = (buy-sell)/(buy+sell)
fn estimate_vol_imbalance(candles: &[Candle], window: usize) -> f64 {
    let window = window.min(candles.len());
    if window == 0 { return 0.0; }

    let mut buy_vol = 0.0;
    let mut sell_vol = 0.0;

    for c in candles.iter().skip(candles.len() - window) {
        let body = (c.c - c.o).abs();
        let range = c.h - c.l;
        if range == 0.0 { continue; }

        // Body ratio: what fraction of the candle range was the body?
        // Larger body = more directional = more of volume is "informed"
        let body_ratio = body / range;

        if c.c > c.o {
            // Up candle: body_ratio portion is buy-side, rest is neutral/sell
            buy_vol += c.v * body_ratio;
            sell_vol += c.v * (1.0 - body_ratio);
        } else {
            // Down candle: body_ratio portion is sell-side
            sell_vol += c.v * body_ratio;
            buy_vol += c.v * (1.0 - body_ratio);
        }
    }

    let total = buy_vol + sell_vol;
    if total == 0.0 { return 0.0; }
    (buy_vol - sell_vol) / total  // range [-1, +1]
}
```

**`strategies.rs` — long_entry / short_entry add vol_imb check:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }
    // RUN80: Volume imbalance confirmation
    if config::VOL_IMB_CONFIRM_ENABLE {
        if ind.vol_imb < config::VOL_IMB_LONG_MIN { return false; }
        if ind.rel_vol < config::VOL_IMB_REL_VOL_MIN { return false; }
    }
    match strat {
        // ... rest unchanged ...
    }
}

pub fn short_entry(ind: &Ind15m, strat: ShortStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p < ind.sma20 || ind.z < -0.5 { return false; }
    // RUN80: Volume imbalance confirmation
    if config::VOL_IMB_CONFIRM_ENABLE {
        if ind.vol_imb > config::VOL_IMB_SHORT_MAX { return false; }
        if ind.rel_vol < config::VOL_IMB_REL_VOL_MIN { return false; }
    }
    match strat {
        // ... rest unchanged ...
    }
}
```

---

## Validation Method

### RUN80.1 — Volume Imbalance Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — volume threshold only (no imbalance check)

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `VOL_IMB_WINDOW` | [10, 20, 30] |
| `VOL_IMB_LONG_MIN` | [0.10, 0.15, 0.20] |
| `VOL_IMB_SHORT_MAX` | [-0.10, -0.15, -0.20] |
| `VOL_IMB_REL_VOL_MIN` | [1.0, 1.2, 1.5] |

**Per coin:** 3 × 3 × 3 × 3 = 81 configs × 18 coins = 1,458 backtests

**Key metrics:**
- `imb_filter_rate`: % of entries blocked by volume imbalance filter
- `imb_at_entries`: average volume imbalance at filtered-in vs filtered-out entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN80.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best imbalance thresholds per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Imbalance filter rate 10–35% (meaningful filtering without over-suppressing)
- Filtered-out trades have lower WR than filtered-in trades (filter is working)

### RUN80.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, vol only) | Vol Imbalance Confirm | Delta |
|--------|------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Avg Imb at LONG Entry | X | X | +N |
| Avg Imb at SHORT Entry | X | X | +N |
| Avg RelVol at Entry | X | X | +N |

---

## Why This Could Fail

1. **OHLCV doesn't include real buy/sell volume:** The imbalance is estimated from candle bodies, not actual trade direction. A large up candle with equal buy and sell volume would still be classified as buy-dominated. This estimation is a proxy with inherent noise.
2. **Volume imbalance is a coincident indicator:** By the time volume imbalance confirms a move, the price may have already moved. This could add confirmation without adding predictive power.
3. **May suppress valid low-volume entries:** Quiet mean reversion setups (low volume, pure z-score driven) would be filtered out. These may still be valid trades.

---

## Why It Could Succeed

1. **Adds directional conviction to volume threshold:** The current vol filter (`vol > vol_ma * 1.2`) confirms there is unusual activity but not whether that activity is buying or selling. Imbalance tells us direction of the unusual volume.
2. **Filters false breakouts:** A price spike on declining volume is a false breakout. Volume imbalance would filter this out — the spike has no volume conviction behind it.
3. **Institutional practice:** Volume imbalance (On-Balance Volume, VWAP volume profile) is a standard institutional trading tool for confirming directional moves.
4. **Complementary to existing filters:** Works alongside F6 (price-based counter-momentum), not instead of it.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN80 Vol Imbalance Confirm |
|--|--|--|
| Volume filter | `vol > vol_ma × 1.2` | vol threshold + imb direction |
| LONG entry | z-score + vol threshold | z-score + vol threshold + imb ≥ +0.15 |
| SHORT entry | z-score + vol threshold | z-score + vol threshold + imb ≤ -0.15 |
| Imbalance tracked | No | Yes (estimated from candle bodies) |
| Entry conviction signal | None | imb direction |
| Implementation complexity | None | Low (add imb field + filter check) |
