# RUN112 — Money Flow Index Confirmation: Require MFI Extreme at Entry for Institutional Conviction

## Hypothesis

**Named:** `mfi_confirmation`

**Mechanism:** COINCLAW's volume filter (`vol > vol_ma * 1.2`) measures raw volume but not the *direction* or *conviction* of money flow. The Money Flow Index (MFI) is a volume-weighted RSI — it measures the rate at which money flows into or out of an asset, incorporating both price and volume. When MFI is very low (e.g., < 20), it indicates strong selling pressure/distribution — a market bathed in volume but going down, which is exactly the setup for a regime LONG mean-reversion. When MFI is very high (e.g., > 80), it indicates accumulation/institutional buying pressure — a market bathed in volume and going up, which confirms a regime SHORT.

**Money Flow Index Confirmation:**
- Track MFI on 15m: MFI = 100 - (100 / (1 + typical_price_up_flow / typical_price_down_flow))
- For regime LONG entry: require `mfi < MFI_LONG_MAX` (e.g., MFI < 30) — confirms selling pressure/extreme
- For regime SHORT entry: require `mfi > MFI_SHORT_MIN` (e.g., MFI > 70) — confirms buying pressure/extreme
- This adds institutional-grade money flow conviction to entries

**Why this is not a duplicate:**
- RUN80 (volume imbalance) estimated buy/sell volume from candle bodies — MFI uses a different formula based on typical price and cumulative flow
- RUN109 (min volume surge) required raw vol spike — MFI requires extreme money flow direction, not just magnitude
- RUN104 (volume dryup exit) — MFI is an entry filter, not an exit; MFI measures inflow/outflow conviction, not volume absence
- RUN88 (trailing z-exit) — completely different mechanism
- RUN85 (momentum pulse filter) used ROC — MFI uses volume-weighted price momentum
- No prior RUN has used MFI as an entry gate for regime trades

**Mechanistic rationale:** MFI combines price and volume into a single oscillator that measures the "smart money" flow. Institutional traders leave footprints in volume — when price drops on high volume and MFI is extremely low, it means institutional selling is at an extreme, and the subsequent mean-reversion is more likely to succeed because the institutional flow has exhausted itself. MFI acts as a filter that ensures entries fire when there is institutional conviction behind the mean-reversion.

---

## Proposed Config Changes

```rust
// RUN112: Money Flow Index Confirmation
pub const MFI_CONFIRM_ENABLE: bool = true;
pub const MFI_LONG_MAX: f64 = 30.0;    // MFI must be below this for LONG entry (selling pressure)
pub const MFI_SHORT_MIN: f64 = 70.0;   // MFI must be above this for SHORT entry (buying pressure)
pub const MFI_LOOKBACK: usize = 14;     // MFI lookback period (standard is 14)
```

**`indicators.rs` — add MFI computation to Ind15m:**
```rust
// Ind15m already has: mfi: f64, — just needs to be computed
// MFI = 100 - (100 / (1 + money_flow_ratio))
// where money_flow_ratio = sum(positive_typical_price_flow) / sum(negative_typical_price_flow)
// over MFI_LOOKBACK periods
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
/// Compute Money Flow Index from 15m indicator data.
fn mfi_confirm_long(ind: &Ind15m) -> bool {
    if !config::MFI_CONFIRM_ENABLE { return true; }
    if ind.mfi.is_nan() { return true; }  // no MFI data available
    ind.mfi < config::MFI_LONG_MAX
}

fn mfi_confirm_short(ind: &Ind15m) -> bool {
    if !config::MFI_CONFIRM_ENABLE { return true; }
    if ind.mfi.is_nan() { return true; }
    ind.mfi > config::MFI_SHORT_MIN
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN112: MFI Confirmation
    if !mfi_confirm_long(ind) { return false; }

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

    // RUN112: MFI Confirmation
    if !mfi_confirm_short(ind) { return false; }

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

### RUN112.1 — MFI Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no MFI filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MFI_LONG_MAX` | [20, 30, 40] |
| `MFI_SHORT_MIN` | [60, 70, 80] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `mfi_filter_rate`: % of entries filtered by MFI confirmation
- `mfi_at_filtered`: average MFI of filtered entries (should be non-extreme)
- `mfi_at_allowed`: average MFI of allowed entries (should be extreme)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN112.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best MFI_LONG_MAX × MFI_SHORT_MIN per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- MFI-filtered entries (blocked) have lower win rate than MFI-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN112.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no MFI) | MFI Confirmation | Delta |
|--------|----------------------|-----------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| MFI at Blocked LONG | — | X | — |
| MFI at Blocked SHORT | — | X | — |
| MFI at Allowed LONG | — | X | — |
| MFI at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **MFI is noisy on 15m:** MFI is derived from cumulative money flow over 14 periods. On 15m bars, the 14-bar window is ~3.5 hours — meaningful but can still be noisy. Extreme MFI readings may not persist across the 15m bar boundary.
2. **MFI and RSI are similar:** MFI is a volume-weighted RSI. If RSI is already capturing the signal, MFI may be redundant. COINCLAW already uses RSI in several strategies.
3. **Filtering may be too strict:** Adding MFI filter on top of existing volume, z-score, and RSI filters may reduce entry count too much, lowering the opportunity set.

---

## Why It Could Succeed

1. **Institutional money flow conviction:** MFI measures the *direction* of money flow, not just magnitude. Low MFI for LONG means money is flowing out — selling pressure — which confirms the mean-reversion thesis.
2. **Volume-weighted signal:** Unlike RSI (price-only), MFI incorporates volume. A drop in price on high volume AND low MFI is a stronger signal than price drop alone.
3. **Different from volume imbalance:** RUN80 estimated buy/sell volume from candle bodies — a heuristic. MFI uses a standard, well-established formula for money flow.
4. **Confirms without duplicating:** MFI is not used by any strategy in COINCLAW. It adds a new dimension (money flow direction) that existing filters don't capture.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN112 MFI Confirmation |
|--|--|--|
| Entry filter | vol > vol_ma × 1.2 | vol × MFI extreme (<30 or >70) |
| Money flow awareness | None | MFI (volume-weighted RSI) |
| Institutional signal | None | Low MFI = distribution (LONG confirm) |
| LONG confirmation | vol spike | vol spike + MFI < 30 |
| SHORT confirmation | vol spike | vol spike + MFI > 70 |
| Filter strictness | Single condition | Dual condition |
| Signal quality | Volume magnitude | Volume-weighted momentum direction |
