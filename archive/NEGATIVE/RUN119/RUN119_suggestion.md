# RUN119 — Vortex Indicator Confirmation: Use VI+ / VI- Crossover for Trend Direction Confirmation

## Hypothesis

**Named:** `vortex_confirm`

**Mechanism:** COINCLAW uses ADX and SMA20 for trend/regime detection, but these have limitations. ADX measures trend strength but not direction. SMA20 is a simple price-vs-MA filter. The Vortex Indicator (VI), developed by Etienne Botes and Douglas Siep, consists of two oscillators — VI+ (positive trend movement) and VI- (negative trend movement) — that capture the strength of upward and downward price movements. When VI+ crosses above VI-, it identifies the start of an uptrend; when VI- crosses above VI+, it identifies the start of a downtrend. The Vortex Indicator Confirmation uses VI+ / VI- crossover as an entry direction filter: for regime LONG entries, require VI- > VI+ (confirming the market is in a downward vortex, not upward), and vice versa for SHORT.

**Vortex Indicator Confirmation:**
- Track VI+ and VI- on 15m over VI_LOOKBACK (e.g., 14 bars)
- `VI+ = EMA of +VM / ATR` where +VM = max(high - low(prev), high - close(prev)) strength
- `VI- = EMA of -VM / ATR` where -VM = max(low - high(prev), low - close(prev)) strength
- For regime LONG entry: require `vi_minus > vi_plus` (confirming negative vortex/downtrend)
- For regime SHORT entry: require `vi_plus > vi_minus` (confirming positive vortex/uptrend)
- This ensures entries align with the correct vortex direction, avoiding entries that fight an emerging trend

**Why this is not a duplicate:**
- RUN86 (correlation cluster suppress) — completely different mechanism
- RUN25 (BTC 4-regime framework) used volatility regimes — VI uses directional movement, not volatility
- RUN12 (scalp market mode filter) used breadth — VI is a single-coin directional indicator
- RUN89 (market-wide ADX confirmation) used ADX — ADX measures trend strength (directional independent), VI distinguishes direction AND strength
- RUN114 (Aroon regime confirm) used Aroon time-since-high/low — VI uses directional movement + ATR normalization, different math and concept
- RUN13 (complement signals) used KST Cross, Kalman Filter — VI is a different directional movement oscillator
- No prior RUN has used Vortex Indicator as an entry direction filter

**Mechanistic rationale:** The Vortex Indicator was specifically designed to identify trend reversal points by measuring the strength of directional movement. Unlike ADX (which rises in both uptrends and downtrends), VI+ and VI- oscillate and cross to show direction. When VI- > VI+, the market is in a "negative vortex" — downward movement is stronger than upward movement. For a regime LONG entry, this is the ideal micro-environment: the market is mid-downtrend, so when price mean-reverts upward, there's room for the mean-reversion to work WITH the emerging uptrend rather than against it.

---

## Proposed Config Changes

```rust
// RUN119: Vortex Indicator Confirmation
pub const VORTEX_CONFIRM_ENABLE: bool = true;
pub const VI_LOOKBACK: usize = 14;       // VI lookback period (standard is 14)
pub const VI_CROSS_STRICT: bool = false;   // require clear crossover gap (not just VI- > VI+)
```

**`indicators.rs` — add Vortex Indicator computation:**
```rust
// VI+ = 14-period EMA of +VM / 14-period ATR
// VI- = 14-period EMA of -VM / 14-period ATR
// +VM = max(H - L(prev), H - C(prev))
// -VM = max(L - H(prev), L - C(prev))
// Ind15m should have: vi_plus: f64, vi_minus: f64
```

**`strategies.rs` — add Vortex Indicator helper functions:**
```rust
/// Check if Vortex Indicator confirms LONG entry (VI- > VI+, negative vortex).
fn vortex_confirm_long(ind: &Ind15m) -> bool {
    if !config::VORTEX_CONFIRM_ENABLE { return true; }
    if ind.vi_plus.is_nan() || ind.vi_minus.is_nan() { return true; }
    if config::VI_CROSS_STRICT {
        // Require VI- to be above VI+ by a margin
        ind.vi_minus > ind.vi_plus
    } else {
        ind.vi_minus > ind.vi_plus
    }
}

/// Check if Vortex Indicator confirms SHORT entry (VI+ > VI-, positive vortex).
fn vortex_confirm_short(ind: &Ind15m) -> bool {
    if !config::VORTEX_CONFIRM_ENABLE { return true; }
    if ind.vi_plus.is_nan() || ind.vi_minus.is_nan() { return true; }
    ind.vi_plus > ind.vi_minus
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // RUN119: Vortex Indicator Confirmation
    if !vortex_confirm_long(ind) { return false; }

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

    // RUN119: Vortex Indicator Confirmation
    if !vortex_confirm_short(ind) { return false; }

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

### RUN119.1 — Vortex Indicator Confirmation Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no Vortex Indicator filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `VI_LOOKBACK` | [14, 21, 28] |
| `VI_CROSS_STRICT` | [true, false] |

**Per coin:** 3 × 2 = 6 configs × 18 coins = 108 backtests

**Key metrics:**
- `vortex_filter_rate`: % of entries filtered by VI direction requirement
- `vi_ratio_at_filtered`: VI- / VI+ ratio at filtered entries (should be opposite of required)
- `vi_ratio_at_allowed`: VI- / VI+ ratio at allowed entries (should confirm direction)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN119.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best VI_LOOKBACK × CROSS_STRICT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Vortex-filtered entries (blocked) have lower win rate than Vortex-confirmed entries
- Filter rate 10–35% (meaningful filtering without over-suppressing)

### RUN119.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no VI) | Vortex Indicator Confirmation | Delta |
|--------|---------------------|---------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| VI- / VI+ at Blocked LONG | — | X | — |
| VI+ / VI- at Blocked SHORT | — | X | — |
| VI- / VI+ at Allowed LONG | — | X | — |
| VI+ / VI- at Allowed SHORT | — | X | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **VI has not been tested in crypto:** Vortex Indicator is relatively obscure compared to RSI or MACD. Its effectiveness in crypto markets (which have different microstructure than the futures markets it was designed for) is unknown.
2. **VI crossover can lag:** VI uses EMA of directional movement / ATR. During choppy markets, VI+ and VI- can oscillate and cross frequently, creating false signals.
3. **Mean-reversion doesn't need trend confirmation:** A core premise of mean-reversion is that price diverged from the mean and must return. Requiring VI to confirm the "right" vortex direction may filter out valid counter-trend entries that fire at the start of a new trend.

---

## Why It Could Succeed

1. **ATR-normalized direction:** VI+ and VI- are normalized by ATR, making them comparable across different volatility regimes. VI- > VI+ means downward movement is stronger relative to the current range — a clean directional signal.
2. **Distinct from ADX and Aroon:** ADX measures trend strength without direction. Aroon measures time-since-high/low. VI measures directional movement strength normalized by range — a unique combination.
3. **Identifies trend transitions:** VI crossing is specifically designed to identify when a trend is about to start. For regime LONG, requiring VI- > VI+ means the downward vortex is dominant — the perfect setup for a mean-reversion entry that catches the bounce.
4. **Complementary to existing filters:** VI is a different calculation (directional movement, not price vs MA, not time-since-high) and adds a new dimension to the confirmation stack.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN119 Vortex Indicator Confirmation |
|--|--|--|
| Directional filter | SMA20 price vs MA | VI+ / VI- crossover |
| Trend strength | ADX | VI strength + direction |
| LONG filter | price < SMA20 | VI- > VI+ |
| SHORT filter | price > SMA20 | VI+ > VI- |
| ATR usage | None | VI normalized by ATR |
| Indicator type | Price vs MA | Directional movement oscillator |
| Regime awareness | SMA20 direction | VI vortex direction |
| Filter source | Single price | High/Low/Close movement |
