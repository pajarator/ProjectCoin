# RUN116 — KST Confirmation Filter: Use Know Sure Thing Momentum Oscillator for Entry Timing

## Hypothesis

**Named:** `kst_confirm_filter`

**Mechanism:** COINCLAW uses z-score and RSI for entry signals, but these are single-lookback oscillators. The Know Sure Thing (KST) is a momentum oscillator developed by Martin Pring that combines multiple rate-of-change (ROC) measurements across different smoothing periods into a single oscillator. KST captures the "market's known sure thing" — the underlying momentum across multiple timeframes. When KST crosses above its signal line (for LONG) or below (for SHORT), it confirms that momentum from multiple timeframes is aligned in the direction of the trade. This provides a multi-timeframe momentum confirmation that single oscillators miss.

**KST Confirmation Filter:**
- Track KST on 15m: KST = weighted sum of RCMA1 through RCMA4 (multiple ROC smoothed moving averages)
- Track KST signal line: 9-period SMA of KST
- For regime LONG entry: require `kst > signal` AND `kst >= KST_LONG_MIN` (momentum turning up from extreme)
- For regime SHORT entry: require `kst < signal` AND `kst <= KST_SHORT_MAX` (momentum turning down from extreme)
- This ensures entries fire when multi-timeframe momentum confirms the mean-reversion direction

**Why this is not a duplicate:**
- RUN85 (momentum pulse filter) used single ROC (roc_3) — KST combines 4 different ROC periods for multi-timeframe confirmation
- RUN95 (scalp momentum alignment) used 15m z < -0.5 — KST uses smoothed multi-period ROC, fundamentally different
- RUN99 (z-momentum divergence) used z-score momentum divergence — KST is a different oscillator using ROC-based momentum
- RUN13 (complement signals) used KST Cross as secondary entry — RUN116 uses KST as an entry GATE, not just a complement
- RUN60 (z_momentum threshold) used z-score direction at entry — KST is a different momentum calculation
- No prior RUN has used KST as a primary entry filter gate for regime trades

**Mechanistic rationale:** KST synthesizes momentum from 4 different timeframes (8, 12, 16, 20-ROC smoothed) into one oscillator. A regime LONG entry with KST > signal and KST at a high reading means momentum from short, medium, and longer-term timeframes are all confirming upward pressure — the mean-reversion has multi-timeframe backing. Without KST confirmation, a regime entry relies only on the single-timeframe z-score signal.

---

## Proposed Config Changes

```rust
// RUN116: KST Confirmation Filter
pub const KST_CONFIRM_ENABLE: bool = true;
pub const KST_LONG_MIN: f64 = 0.0;       // KST must be >= this for LONG (momentum positive territory)
pub const KST_SHORT_MAX: f64 = 0.0;      // KST must be <= this for SHORT (momentum negative territory)
pub const KST_SIGNAL_CROSS: bool = true;  // require KST to cross signal line (above for LONG, below for SHORT)
```

**`indicators.rs` — add KST computation:**
```rust
// KST = (RCMA1 * 1) + (RCMA2 * 2) + (RCMA3 * 3) + (RCMA4 * 4) / 10
// where RCMA1 = 10-period SMA of 8-period ROC
//       RCMA2 = 10-period SMA of 12-period ROC
//       RCMA3 = 10-period SMA of 16-period ROC
//       RCMA4 = 10-period SMA of 20-period ROC
// Signal = 9-period SMA of KST
// Ind15m already has kst: f64 and kst_signal: f64 fields
```

**`strategies.rs` — add KST helper functions:**
```rust
/// Check if KST confirms LONG entry (momentum turning up).
fn kst_confirm_long(ind: &Ind15m) -> bool {
    if !config::KST_CONFIRM_ENABLE { return true; }
    if ind.kst.is_nan() || ind.kst_signal.is_nan() { return true; }
    if ind.kst < config::KST_LONG_MIN { return false; }
    if config::KST_SIGNAL_CROSS && ind.kst <= ind.kst_signal { return false; }
    true
}

/// Check if KST confirms SHORT entry (momentum turning down).
fn kst_confirm_short(ind: &Ind15m) -> bool {
    if !config::KST_CONFIRM_ENABLE { return true; }
    if ind.kst.is_nan() || ind.kst_signal.is_nan() { return true; }
    if ind.kst > config::KST_SHORT_MAX { return false; }
    if config::KST_SIGNAL_CROSS && ind.kst >= ind.kst_signal { return false; }
    true
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN116: KST Confirmation Filter
    if !kst_confirm_long(ind) { return false; }

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

    // RUN116: KST Confirmation Filter
    if !kst_confirm_short(ind) { return false; }

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

### RUN116.1 — KST Confirmation Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no KST filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `KST_LONG_MIN` | [-50, 0, 50] |
| `KST_SHORT_MAX` | [-50, 0, 50] |
| `KST_SIGNAL_CROSS` | [true, false] |

**Per coin:** 3 × 3 × 2 = 18 configs × 18 coins = 324 backtests

**Key metrics:**
- `kst_filter_rate`: % of entries filtered by KST confirmation
- `kst_at_filtered`: average KST at filtered entries
- `kst_at_allowed`: average KST at allowed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN116.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best KST_LONG_MIN × KST_SHORT_MAX × SIGNAL_CROSS per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- KST-filtered entries (blocked) have lower win rate than KST-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN116.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no KST) | KST Confirmation Filter | Delta |
|--------|----------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| KST at Blocked LONG | — | X | — |
| KST at Blocked SHORT | — | X | — |
| KST at Allowed LONG | — | X | — |
| KST at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **KST is a trend-following oscillator:** KST crossing its signal line is a momentum confirmation signal. Using it to filter mean-reversion entries may block valid counter-trend setups where momentum hasn't yet turned.
2. **Multiple ROC periods may lag:** Combining 4 ROC periods (8, 12, 16, 20) means KST is a slow-moving average of momentum. It may not respond quickly enough to sudden mean-reversion opportunities.
3. **KST signal is noisy:** The KST oscillator can oscillate around zero frequently. Requiring a signal line cross may create choppy entry/exit behavior.

---

## Why It Could Succeed

1. **Multi-timeframe momentum synthesis:** KST combines 4 different ROC periods into one oscillator. This captures momentum from short, medium, and longer-term perspectives simultaneously — more information than any single oscillator.
2. **KST as entry gate catches momentum alignment:** When KST crosses above its signal line, multiple timeframes of momentum are aligning. This gives mean-reversion entries more conviction because the broader momentum backdrop is supportive.
3. **Smoothed momentum avoids noise:** The multiple smoothing stages (SMA of ROC × multiple periods) filter out noise that single-lookback momentum indicators miss.
4. **Institutional-grade tool:** KST is a respected momentum oscillator in technical analysis, used by practitioners for identifying trend changes across multiple timeframes.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN116 KST Confirmation Filter |
|--|--|--|
| Entry filter | z-score + RSI + vol | z-score + RSI + vol + KST |
| Momentum source | Single-lookback (z, RSI) | Multi-ROC (4-period KST) |
| LONG momentum confirm | None | KST > signal + KST >= 0 |
| SHORT momentum confirm | None | KST < signal + KST <= 0 |
| Timeframe synthesis | 15m only | 8/12/16/20 ROC smoothed |
| Entry timing | Single oscillator | Multi-timeframe confirmation |
| Filter strength | Double (z + RSI) | Triple (z + RSI + KST) |
