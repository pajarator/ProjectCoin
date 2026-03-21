# RUN108 — Momentum Hour Filter: Suppress Regime Entries When 1m Momentum Strongly Opposes Trade Direction

## Hypothesis

**Named:** `momentum_hour_filter`

**Mechanism:** COINCLAW uses the 15m timeframe for regime entry signals. But the 1m timeframe contains short-term momentum information that can be used to filter entries: if a regime LONG signal fires but the 1m momentum is strongly negative (price is in a sharp short-term downtrend), the entry is fighting a strong headwind. The Momentum Hour Filter suppresses regime entries when the 1m momentum opposes the trade direction by a threshold.

**Momentum Hour Filter:**
- Measure 1m momentum: `roc_1m = (close_1m - close_N_1m_bars_ago) / close_N_1m_bars_ago`
- For regime LONG entry: require `roc_1m >= MOMENTUM_HOUR_LONG_MIN` (e.g., -0.002) — price must not be in a sharp short-term downtrend
- For regime SHORT entry: require `roc_1m <= MOMENTUM_HOUR_SHORT_MAX` (e.g., +0.002) — price must not be in a sharp short-term uptrend
- F6 blocks counter-momentum entries (momentum was against us over last 3 candles). This goes further: it blocks when momentum is strongly against us (larger threshold) even if not blocked by F6.

**Why this is not a duplicate:**
- F6 (RUN10) uses `dir_roc_3 < -0.195` as a negative filter (blocks counter-momentum) — this uses a positive threshold on 1m momentum as an entry gate
- RUN40 (BTC_DOM_SCALE) uses btc_z − avg_z for scalp — this uses 1m ROC for regime entries
- RUN60 (z_momentum threshold) uses z-score direction at entry — this uses 1m price momentum
- RUN85 (momentum pulse) uses 3-bar ROC for regime LONG — this uses 1m momentum with a different threshold and timeframe
- No prior RUN has used 1m momentum as a gate for 15m regime entries

**Mechanistic rationale:** The 1m timeframe captures the immediate short-term flow. A regime LONG at z = -2.0 on the 15m while the 1m is in a sharp downtrend (roc_1m = -0.5%) is fighting strong headwind — the short-term momentum is working against the mean-reversion thesis. Suppressing this entry avoids entries that are fighting both the mean-reversion headwind and the short-term momentum.

---

## Proposed Config Changes

```rust
// RUN108: Momentum Hour Filter
pub const MOMENTUM_HOUR_ENABLE: bool = true;
pub const MOMENTUM_HOUR_LOOKBACK: usize = 6;   // lookback in 1m bars (e.g., 6 × 1m = 6 minutes)
pub const MOMENTUM_HOUR_LONG_MIN: f64 = -0.002;  // 1m ROC must be >= -0.2% for LONG entry
pub const MOMENTUM_HOUR_SHORT_MAX: f64 = 0.002;  // 1m ROC must be <= +0.2% for SHORT entry
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
/// Compute 1m momentum (ROC over MOMENTUM_HOUR_LOOKBACK bars).
fn get_1m_momentum(state: &SharedState, ci: usize) -> f64 {
    let cs = &state.coins[ci];
    let candles = &cs.candles_1m;
    if candles.len() < config::MOMENTUM_HOUR_LOOKBACK + 1 {
        return 0.0;
    }
    let lookback = config::MOMENTUM_HOUR_LOOKBACK;
    let recent = candles[candles.len() - 1].c;
    let past = candles[candles.len() - 1 - lookback].c;
    (recent - past) / past
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat, state: &SharedState, ci: usize) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // Base entry check (existing logic)
    let base_ok = match strat {
        LongStrat::VwapReversion => ind.z < -1.5 && ind.p < ind.vwap && ind.vol > ind.vol_ma * 1.2,
        LongStrat::BbBounce => ind.p <= ind.bb_lo * 1.02 && ind.vol > ind.vol_ma * 1.3,
        LongStrat::DualRsi => ind.rsi < 40.0 && ind.rsi7 < 30.0 && ind.sma9 > ind.sma20,
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
    };
    if !base_ok { return false; }

    // RUN108: Momentum Hour Filter
    if config::MOMENTUM_HOUR_ENABLE {
        let roc_1m = get_1m_momentum(state, ci);
        if roc_1m < config::MOMENTUM_HOUR_LONG_MIN {
            return false;  // 1m momentum too negative for LONG
        }
    }

    true
}

pub fn short_entry(ind: &Ind15m, strat: ShortStrat, state: &SharedState, ci: usize) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p < ind.sma20 || ind.z < -0.5 { return false; }

    // Base entry check (existing logic)
    let base_ok = match strat {
        ShortStrat::ShortVwapRev => ind.z > 1.5 && ind.p > ind.vwap && ind.vol > ind.vol_ma * 1.2,
        ShortStrat::ShortBbBounce => ind.p >= ind.bb_hi * 0.98 && ind.vol > ind.vol_ma * 1.3,
        ShortStrat::ShortMeanRev => ind.z > 1.5,
        ShortStrat::ShortAdrRev => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_hi.is_nan() && range > 0.0
                && ind.p >= ind.adr_hi - range * 0.25
                && ind.vol > ind.vol_ma * 1.1
        }
    };
    if !base_ok { return false; }

    // RUN108: Momentum Hour Filter
    if config::MOMENTUM_HOUR_ENABLE {
        let roc_1m = get_1m_momentum(state, ci);
        if roc_1m > config::MOMENTUM_HOUR_SHORT_MAX {
            return false;  // 1m momentum too positive for SHORT
        }
    }

    true
}
```

---

## Validation Method

### RUN108.1 — Momentum Hour Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m + 1m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no 1m momentum filter on regime entries

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `MOMENTUM_HOUR_LOOKBACK` | [3, 6, 12] |
| `MOMENTUM_HOUR_LONG_MIN` | [-0.001, -0.002, -0.003] |
| `MOMENTUM_HOUR_SHORT_MAX` | [0.001, 0.002, 0.003] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `momentum_filter_rate`: % of regime entries blocked by momentum hour filter
- `roc_1m_at_blocked`: average roc_1m of blocked entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN108.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best threshold pair per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Filter rate 10–35% (meaningful filtering)
- Blocked entries have lower win rate than allowed entries (confirming filter is working)

### RUN108.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no 1m filter) | Momentum Hour Filter | Delta |
|--------|-----------------------------|-------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Filtered | 0% | X% | — |
| Avg 1m ROC at Blocked LONG | — | X% | — |
| Avg 1m ROC at Blocked SHORT | — | X% | — |
| Allowed Entry WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **1m momentum is noisy:** The 1m timeframe is very noisy. A single bad candle can create a negative roc_1m that doesn't reflect the true short-term trend. The filter may block valid entries based on noise.
2. **Regime trades are 15m setups:** The 1m momentum is a different timeframe than the regime signal. Mixing timeframes may create conflicting signals that reduce edge rather than add it.
3. **F6 already handles this:** F6 already blocks counter-momentum entries using the 1m roc_3. Adding a separate 1m momentum filter may be redundant.

---

## Why It Could Succeed

1. **Captures the immediate flow:** The 1m timeframe captures the most recent short-term momentum. A regime LONG while the 1m is sharply down is fighting two opposing forces — the 15m mean-reversion signal and the 1m momentum. Filtering these out reduces friction.
2. **Complements F6:** F6 blocks very strong counter-momentum (roc_3 < -0.195). This filter blocks moderately negative 1m momentum that might not trigger F6 but still creates headwind.
3. **Multi-timeframe confirmation:** Adding 1m momentum as a confirmation for 15m regime entries is a natural multi-timeframe approach used in technical analysis.
4. **Simple and fast:** One comparison per entry check. Minimal computation.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN108 Momentum Hour Filter |
|--|--|--|
| 1m momentum filter | None | Blocks LONG if roc_1m < -0.2% |
| Short filter | None | Blocks SHORT if roc_1m > +0.2% |
| Timeframe | 15m only | 15m + 1m confirmation |
| Counter-momentum handling | F6 (strong counter only) | F6 + moderate 1m filter |
| Entry quality | Single-timeframe | Multi-timeframe |
