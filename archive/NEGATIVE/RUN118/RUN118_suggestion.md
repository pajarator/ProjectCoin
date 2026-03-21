# RUN118 — Hull Suite Trend Filter: Use Alan Hull's Hull MA for Low-Lag Trend Confirmation

## Hypothesis

**Named:** `hull_suite_filter`

**Mechanism:** COINCLAW uses SMA20 as a trend filter and exit signal, but SMA has significant lag. The Hull Moving Average (HMA), developed by Alan Hull, reduces lag dramatically while maintaining smoothness — it achieves this by using weighted moving averages and the square root of the lookback period. The Hull Suite Trend Filter uses HMA as a faster, lower-lag alternative to SMA20 for trend direction filtering and entry confirmation: price must be below HMA for LONG entries (confirming downtrend) and above HMA for SHORT entries (confirming uptrend), with HMA slope providing earlier direction change signals than SMA.

**Hull Suite Trend Filter:**
- Track Hull MA (HMA) on 15m: `HMA = WMA(2 × WMA(n/2) − WMA(n), sqrt(n))`
- For regime LONG entry: require `ind.p < ind.hma` AND `hma_slope < 0` (price below HMA, HMA trending down)
- For regime SHORT entry: require `ind.p > ind.hma` AND `hma_slope > 0` (price above HMA, HMA trending up)
- As exit: exit LONG when price crosses above HMA; exit SHORT when price crosses below HMA
- HMA's lower lag gives earlier entry/exit signals than SMA20

**Why this is not a duplicate:**
- RUN13 (complement signals) used KST Cross as secondary — HMA is a different MA type with lag reduction
- RUN88 (trailing z-exit) — completely different mechanism
- SMA20 is used throughout COINCLAW but never HMA — HMA is a fundamentally different calculation with significantly less lag
- No prior RUN has used Hull MA as a trend filter for entries or as an exit mechanism

**Mechanistic rationale:** SMA20 has ~10 bar lag (half of lookback). HMA reduces this to ~sqrt(20) ≈ 4.5 bars while maintaining smoothness. For regime entries that require trend confirmation, HMA's faster response means entries fire earlier in the mean-reversion cycle, capturing better prices. For exits, HMA's lower lag means exits fire sooner when the trend reverses, capturing more profit and reducing drawdown. The Hull Suite also includes the Hull Oscillator and Directional Movement Index variants, but the core HMA is the primary signal.

---

## Proposed Config Changes

```rust
// RUN118: Hull Suite Trend Filter
pub const HULL_FILTER_ENABLE: bool = true;
pub const HULL_LOOKBACK: usize = 20;       // HMA lookback period (standard is 20)
pub const HULL_SLOPE_CONFIRM: bool = true;  // require HMA slope to confirm direction
pub const HULL_EXIT: bool = true;           // use HMA crossover as exit trigger
```

**`indicators.rs` — add HMA computation:**
```rust
// HMA(n) = WMA(2 * WMA(n/2, price) - WMA(n, price), sqrt(n))
// Hull Suite: HMA + Hull Oscillator (difference between HMA and HMA of HMA)
// Ind15m should have: hma: f64, hma_slope: f64
```

**`strategies.rs` — add Hull Suite helper functions:**
```rust
/// Check if Hull MA confirms LONG entry (price below HMA, HMA trending down).
fn hull_confirm_long(ind: &Ind15m) -> bool {
    if !config::HULL_FILTER_ENABLE { return true; }
    if ind.hma.is_nan() { return true; }
    if ind.p >= ind.hma { return false; }
    if config::HULL_SLOPE_CONFIRM && ind.hma_slope >= 0.0 { return false; }
    true
}

/// Check if Hull MA confirms SHORT entry (price above HMA, HMA trending up).
fn hull_confirm_short(ind: &Ind15m) -> bool {
    if !config::HULL_FILTER_ENABLE { return true; }
    if ind.hma.is_nan() { return true; }
    if ind.p <= ind.hma { return false; }
    if config::HULL_SLOPE_CONFIRM && ind.hma_slope <= 0.0 { return false; }
    true
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN118: Hull Suite Trend Filter
    if !hull_confirm_long(ind) { return false; }

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

    // RUN118: Hull Suite Trend Filter
    if !hull_confirm_short(ind) { return false; }

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

**`engine.rs` — add Hull MA exit:**
```rust
/// In check_exit, add Hull MA crossover exit:
if config::HULL_EXIT && config::HULL_FILTER_ENABLE {
    // LONG exit: price crosses above HMA
    if trade.direction == Direction::Long && !cs.ind_15m.as_ref().map(|i| i.hma).unwrap_or(f64::NAN).is_nan() {
        if cs.ind_15m.as_ref().map(|i| i.p > i.hma).unwrap_or(false) {
            return ExitReason::HullMaCross;
        }
    }
    // SHORT exit: price crosses below HMA
    if trade.direction == Direction::Short && !cs.ind_15m.as_ref().map(|i| i.hma).unwrap_or(f64::NAN).is_nan() {
        if cs.ind_15m.as_ref().map(|i| i.p < i.hma).unwrap_or(false) {
            return ExitReason::HullMaCross;
        }
    }
}
```

---

## Validation Method

### RUN118.1 — Hull Suite Trend Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — SMA20 for trend filter/exit

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `HULL_LOOKBACK` | [14, 20, 28] |
| `HULL_SLOPE_CONFIRM` | [true, false] |
| `HULL_EXIT` | [true, false] |

**Per coin:** 3 × 2 × 2 = 12 configs × 18 coins = 216 backtests

**Key metrics:**
- `hull_filter_rate`: % of entries filtered by Hull MA direction
- `hull_exit_rate`: % of trades exiting via Hull MA crossover
- `hull_exit_wr`: win rate of Hull MA crossover exits
- `avg_entry_timing_delta`: change in entry bar vs SMA20-based entries (should be earlier)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline

### RUN118.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best HULL_LOOKBACK × SLOPE_CONFIRM per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Hull MA exits have higher win rate than SMA20 exits
- Entry filter rate 10–35%

### RUN118.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, SMA20) | Hull Suite Trend Filter | Delta |
|--------|----------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Hull Filter Rate | 0% | X% | — |
| Hull Exit Rate | 0% | X% | — |
| SMA20 Exit Rate | X% | X% | -Y% |
| Hull Exit WR% | — | X% | — |
| Avg Entry Bar vs SMA | 0 | -N | Earlier |

---

## Why This Could Fail

1. **HMA is more reactive but noisier:** The lag reduction comes at a cost — HMA can be noisier than SMA, potentially giving false signals in choppy markets. The faster response also means it can flip direction more quickly, causing premature exits.
2. **Hull Suite is less established:** Unlike SMA, EMA, or even VWAP, HMA is less commonly used in institutional trading. Its behavior in crypto markets may not be well-characterized.
3. **SMA20 is already working:** COINCLAW's SMA20 filter has been battle-tested. Replacing it with HMA is an unproven change that may not add enough benefit to justify the risk.

---

## Why It Could Succeed

1. **Significantly lower lag:** HMA reduces lag from ~10 bars to ~4-5 bars for a 20-period MA. This means entries fire earlier in the mean-reversion cycle, capturing better prices.
2. **Still smooth:** Unlike EMA (which is smooth but still has lag) or WMA (which has low lag but is choppy), HMA achieves both low lag AND smoothness through its unique calculation.
3. **Earlier exits:** When trend reverses, HMA flips before SMA. This means exits fire sooner, capturing more profit and reducing drawdown.
4. **Drop-in SMA replacement:** HMA can be computed from the same data SMA uses. It's a direct upgrade — same conceptual framework, better implementation.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN118 Hull Suite Trend Filter |
|--|--|--|
| Trend MA | SMA20 (lag ≈ 10 bars) | HMA20 (lag ≈ 4-5 bars) |
| Entry filter | SMA20 direction | HMA direction + slope |
| Exit trigger | SMA20 crossback | HMA crossover |
| MA lag | ~10 bars | ~4-5 bars |
| Smoothness | High | High (HMA's design goal) |
| Trend sensitivity | Low (slow) | High (faster response) |
| Entry timing | Later | Earlier |
| Exit timing | Later | Earlier |
