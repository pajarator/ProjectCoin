# RUN131 — Volume-Weighted RSI Confirmation: Require RSI Extreme to Be Volume-Confirmed

## Hypothesis

**Named:** `vwrsi_confirm`

**Mechanism:** COINCLAW uses RSI as an oscillator filter for regime entries (e.g., RSI < 40 for LONG), but standard RSI is computed purely from price changes, without considering volume. Volume-Weighted RSI (VWRSI) weights each price change by its associated volume — so a large price change on high volume contributes more to the RSI than the same price change on low volume. This ensures RSI extremes are backed by volume conviction. The Volume-Weighted RSI Confirmation requires standard RSI to be extreme AND VWRSI to confirm the extreme is volume-backed: if RSI shows oversold but VWRSI does NOT confirm (because the moves were on low volume), the entry is filtered out.

**Volume-Weighted RSI Confirmation:**
- Track VWRSI on 15m: `VWRSI = 100 - (100 / (1 + sum(vol_i × up_i) / sum(vol_i × down_i)))`
- For regime LONG entry: require `rsi < 40` AND `vwrsi < VWRSI_LONG_MAX` (e.g., VWRSI < 35) — the RSI extreme must be volume-confirmed
- For regime SHORT entry: require `rsi > 60` AND `vwrsi > VWRSI_SHORT_MIN` (e.g., VWRSI > 65) — the RSI extreme must be volume-confirmed
- This ensures RSI extremes are backed by volume conviction, filtering out weak signals

**Why this is not a duplicate:**
- RUN109 (min volume surge) required volume spike — VWRSI weights ALL price changes by volume, not just requiring a surge
- RUN112 (MFI confirmation) used MFI (volume-weighted RSI-like oscillator) — VWRSI is a different formula using up/down moves weighted by volume
- RUN80 (volume imbalance) estimated buy/sell imbalance — VWRSI uses the ratio of volume-weighted up vs down moves, different calculation
- RSI is used in multiple strategies (DualRsi, etc.) but standard RSI, not volume-weighted
- No prior RUN has used Volume-Weighted RSI as an entry filter

**Mechanistic rationale:** Standard RSI treats a 1% price change the same whether it occurred on 100 units of volume or 10,000 units. Volume-Weighted RSI weights price changes by volume, so only moves backed by substantial volume contribute meaningfully to the RSI. When RSI shows oversold but VWRSI shows a much higher reading (because the declines were on low volume), the signal is weak — the "oversold" reading came from light-volume noise, not genuine selling pressure. Requiring VWRSI confirmation ensures RSI extremes are backed by real market participation.

---

## Proposed Config Changes

```rust
// RUN131: Volume-Weighted RSI Confirmation
pub const VWRSI_CONFIRM_ENABLE: bool = true;
pub const VWRSI_LONG_MAX: f64 = 35.0;     // VWRSI must be below this for LONG (volume-confirmed oversold)
pub const VWRSI_SHORT_MIN: f64 = 65.0;    // VWRSI must be above this for SHORT (volume-confirmed overbought)
```

**`indicators.rs` — add VWRSI computation:**
```rust
// VWRSI = 100 - (100 / (1 + RS))
// where RS = sum(vol_i × max(0, close_i - close_{i-1})) / sum(vol_i × max(0, close_{i-1} - close_i))
// over RSI_PERIOD (standard 14) bars
// Ind15m should have: vwrsi: f64
```

**`strategies.rs` — add VWRSI helper functions:**
```rust
/// Check if VWRSI confirms LONG entry (volume-confirmed oversold).
fn vwrsi_confirm_long(ind: &Ind15m) -> bool {
    if !config::VWRSI_CONFIRM_ENABLE { return true; }
    if ind.vwrsi.is_nan() { return true; }
    ind.vwrsi < config::VWRSI_LONG_MAX
}

/// Check if VWRSI confirms SHORT entry (volume-confirmed overbought).
fn vwrsi_confirm_short(ind: &Ind15m) -> bool {
    if !config::VWRSI_CONFIRM_ENABLE { return true; }
    if ind.vwrsi.is_nan() { return true; }
    ind.vwrsi > config::VWRSI_SHORT_MIN
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN131: Volume-Weighted RSI Confirmation
    if !vwrsi_confirm_long(ind) { return false; }

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
        LongStral::AdrReversal => {
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

    // RUN131: Volume-Weighted RSI Confirmation
    if !vwrsi_confirm_short(ind) { return false; }

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

### RUN131.1 — VWRSI Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — standard RSI only (no VWRSI)

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `VWRSI_LONG_MAX` | [30, 35, 40] |
| `VWRSI_SHORT_MIN` | [60, 65, 70] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `vwrsi_filter_rate`: % of entries filtered by VWRSI confirmation
- `rsi_vs_vwrsi_divergence`: average gap between RSI and VWRSI at filtered entries
- `vwrsi_at_allowed`: average VWRSI at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN131.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best VWRSI_LONG_MAX × VWRSI_SHORT_MIN per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- VWRSI-filtered entries (blocked) have lower win rate than VWRSI-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN131.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, RSI only) | Volume-Weighted RSI Confirmation | Delta |
|--------|----------------------|------------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| RSI at Blocked LONG | X | X | — |
| VWRSI at Blocked LONG | — | X | — |
| RSI at Allowed LONG | X | X | — |
| VWRSI at Allowed LONG | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **VWRSI adds computation without enough new information:** If RSI is already extreme, the market is likely oversold/overbought regardless of volume. VWRSI may just be confirming what RSI already says, adding complexity without incremental value.
2. **High-volume noise:** Volume can be noisy — a single high-volume bar can skew VWRSI significantly. The volume-weighted calculation may introduce more noise than it removes.
3. **Two thresholds to tune:** VWRSI requires two additional parameters (LONG_MAX and SHORT_MIN), increasing the search space and risk of overfitting.

---

## Why It Could Succeed

1. **Volume confirms conviction:** A price decline on high volume is genuine selling pressure; the same decline on low volume is likely noise. VWRSI distinguishes between these, ensuring RSI extremes come from real market participation.
2. **Natural extension of RSI:** RSI is already used in COINCLAW. Adding VWRSI as a confirmation is a natural next step — it's RSI, but better.
3. **Identifies weak RSI signals:** When RSI shows oversold but VWRSI shows a higher value, it means the declines were on low volume — the RSI extreme is not confirmed. Filtering these out improves signal quality.
4. **Different from MFI:** MFI is an oscillator scaled 0-100 (like RSI). VWRSI is RSI itself (0-100 scale) but with volume weighting applied to the up/down move ratio — a different formula and interpretation.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN131 Volume-Weighted RSI Confirmation |
|--|--|--|
| RSI filter | Standard RSI only | Standard RSI + VWRSI dual confirmation |
| Volume awareness | None in RSI | Volume-weighted up/down moves |
| LONG confirmation | RSI < 40 | RSI < 40 AND VWRSI < 35 |
| SHORT confirmation | RSI > 60 | RSI > 60 AND VWRSI > 65 |
| Signal quality | Price-based | Price + volume weighted |
| Conviction filter | None | Volume confirms RSI extreme |
| Filter strength | Single condition | Dual oscillator |
| Weak signal detection | None | RSI/VWRSI divergence identifies weak signals |
