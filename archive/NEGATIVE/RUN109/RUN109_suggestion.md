# RUN109 — Minimum Volume Surge Confirmation: Require Volume Spike at Entry, Not Just Above-Average

## Hypothesis

**Named:** `min_vol_surge`

**Mechanism:** COINCLAW's current volume filter requires `vol > vol_ma * 1.2` for regime entries. But this is a weak filter — it only requires volume to be slightly above average. Many valid mean-reversion setups fire with only marginal volume. The Minimum Volume Surge Confirmation requires a genuine volume spike: `vol > vol_ma * VOL_SURGE_MIN` (e.g., 2.0×) — the volume must be at least 2× the average, indicating a real spike of market participation, not just slightly elevated volume.

**Minimum Volume Surge Confirmation:**
- Replace the weak `vol > vol_ma * 1.2` with a stronger `vol > vol_ma * VOL_SURGE_MIN`
- For regime LONG entries: require `vol >= vol_ma * VOL_SURGE_LONG`
- For regime SHORT entries: require `vol >= vol_ma * VOL_SURGE_SHORT`
- This is applied on top of the existing strategy-specific volume requirements

**Why this is not a duplicate:**
- RUN10 (F6 filter) used dir_roc_3 and avg_body_3 — this is purely a volume magnitude filter
- RUN54 (vol entry threshold) used ATR percentile rank — this uses absolute volume surge
- RUN80 (volume imbalance) used buy/sell imbalance — this uses volume surge magnitude
- No prior RUN has required a genuine volume spike (2× average) as an entry filter

**Mechanistic rationale:** A volume spike at entry is a proxy for market participation conviction. When volume surges, it means real money is moving — the mean-reversion opportunity is being recognized by the market. Without volume, a z-score extreme is just a statistical quirk. Volume surge confirms the opportunity is real and traded.

---

## Proposed Config Changes

```rust
// RUN109: Minimum Volume Surge Confirmation
pub const MIN_VOL_SURGE_ENABLE: bool = true;
pub const VOL_SURGE_LONG: f64 = 2.0;    // vol must be ≥ 2× vol_ma for LONG entry
pub const VOL_SURGE_SHORT: f64 = 2.0;   // vol must be ≥ 2× vol_ma for SHORT entry
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN109: Minimum Volume Surge Confirmation
    if config::MIN_VOL_SURGE_ENABLE {
        if ind.vol_ma > 0.0 && ind.vol < ind.vol_ma * config::VOL_SURGE_LONG {
            return false;  // not enough volume surge
        }
    }

    match strat {
        LongStrat::VwapReversion => {
            ind.z < -1.5 && ind.p < ind.vwap && ind.vol > ind.vol_ma * 1.2
        }
        LongStrat::BbBounce => {
            ind.p <= ind.bb_lo * 1.02 && ind.vol > ind.vol_ma * 1.3
        }
        LongStrat::DualRsi => {
            ind.rsi < 40.0 && ind.rsi7 < 30.0 && ind.sma9 > ind.sma20
        }
        LongStrat::AdrReversal => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_lo.is_nan() && range > 0.0
                && ind.p <= ind.adr_lo + range * 0.25
                && ind.vol > ind.vol_ma * 1.1
        }
        LongStrat::MeanReversion => ind.z < -1.5,
        LongStrat::OuMeanRev => {
            !ind.ou_halflife.is_nan() && !ind.ou_deviation.is_nan()
                && ind.std20 > 0.0
                && ind.ou_halflife >= config::OU_MIN_HALFLIFE
                && ind.ou_halflife <= config::OU_MAX_HALFLIFE
                && (ind.ou_deviation / ind.std20) < -config::OU_DEV_THRESHOLD
        }
    }
}

pub fn short_entry(ind: &Ind15m, strat: ShortStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p < ind.sma20 || ind.z < -0.5 { return false; }

    // RUN109: Minimum Volume Surge Confirmation
    if config::MIN_VOL_SURGE_ENABLE {
        if ind.vol_ma > 0.0 && ind.vol < ind.vol_ma * config::VOL_SURGE_SHORT {
            return false;  // not enough volume surge
        }
    }

    match strat {
        ShortStrat::ShortVwapRev => {
            ind.z > 1.5 && ind.p > ind.vwap && ind.vol > ind.vol_ma * 1.2
        }
        ShortStrat::ShortBbBounce => {
            ind.p >= ind.bb_hi * 0.98 && ind.vol > ind.vol_ma * 1.3
        }
        ShortStrat::ShortMeanRev => ind.z > 1.5,
        ShortStrat::ShortAdrRev => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_hi.is_nan() && range > 0.0
                && ind.p >= ind.adr_hi - range * 0.25
                && ind.vol > ind.vol_ma * 1.1
        }
    }
}
```

---

## Validation Method

### RUN109.1 — Volume Surge Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — `vol > vol_ma * 1.2`

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `VOL_SURGE_LONG` | [1.5, 2.0, 2.5, 3.0] |
| `VOL_SURGE_SHORT` | [1.5, 2.0, 2.5, 3.0] |

**Per coin:** 4 × 4 = 16 configs × 18 coins = 288 backtests

**Key metrics:**
- `vol_surge_filter_rate`: % of entries filtered by volume surge requirement
- `vol_ratio_at_filtered`: vol/vol_ma of filtered entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN109.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best VOL_SURGE_LONG × VOL_SURGE_SHORT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Filter rate 15–40% (meaningful filtering without over-suppressing)
- Filtered-out entries have lower win rate than allowed entries

### RUN109.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, vol > 1.2×) | Min Volume Surge | Delta |
|--------|--------------------------|-----------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Avg Vol Ratio (filtered) | 1.2× | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Volume is noisy:** A single-bar volume spike may be a random occurrence, not a genuine signal. The volume surge requirement may filter out valid entries based on bar-to-bar volume noise.
2. **Mean reversion doesn't always need volume:** Price can mean-revert on low volume if the prior move was itself low-volume. Requiring a surge may filter out valid low-volume mean-reversion setups.
3. **May over-filter:** Increasing from 1.2× to 2.0× could reduce entries by 40%+, reducing the opportunity set significantly.

---

## Why It Could Succeed

1. **Genuine conviction signal:** A volume surge at entry means real market participants recognize the opportunity. Without volume, the z-score extreme may be a statistical artifact.
2. **Institutional practice:** Volume confirmation is standard in technical analysis. Entries on above-average volume are more reliable than entries on below-average volume.
3. **Reduces false signals:** Many bad entries fire on flat or declining volume. Requiring a surge filters these out, leaving only high-conviction setups.
4. **Simple and interpretable:** One threshold per direction. Clear meaning.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN109 Min Volume Surge |
|--|--|--|
| Volume filter (LONG) | vol > vol_ma × 1.2 | vol > vol_ma × 2.0 |
| Volume filter (SHORT) | vol > vol_ma × 1.2 | vol > vol_ma × 2.0 |
| Filter strength | Weak | Strong |
| Entry conviction | Marginal volume | Volume spike confirmed |
| Entries filtered | 0% | X% |
| Volume noise immunity | Low | High |
