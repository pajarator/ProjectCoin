# RUN44 — Multi-Timeframe ISO Short Confirmation: 15m/1h/4h Overbought Alignment

## Hypothesis

**Named:** `multi_timeframe_iso_confirmation`

**Mechanism:** COINCLAW's ISO short entries (`IsoShortStrat`) are triggered purely by 15m indicators. A coin can show overbought conditions on 15m due to temporary noise, but if the 1h and 4h timeframes are neutral or bullish, the 15m "overbought" condition is likely to reverse quickly.

The hypothesis is that ISO short entries should require **multi-timeframe confirmation**:
- **1h RSI** must also be overbought (e.g., RSI > 65) for the ISO short to fire
- **4h RSI** must also be overbought (e.g., RSI > 70) for additional confirmation
- When all three timeframes align (15m + 1h + 4h), the ISO short has higher win rate because the move is driven by genuine, broad-based overbought conditions rather than coin-specific noise

This is analogous to how institutional traders check multiple timeframes before committing to a position. If BTC is overbought on 15m but neutral on 4h, the 15m overbought is likely temporary. If BTC is overbought on all three, the reversal signal is much stronger.

**Why this is not a duplicate:**
- No prior RUN tested multi-timeframe confirmation
- RUN6 (ISO short discovery) established the baseline single-timeframe ISO short logic
- RUN34 (ISO drawdown mitigation) tested SL/cooldown changes, not entry signal improvement
- RUN28 (momentum persistence) classified coins by their post-breakout behavior — different direction
- Multi-timeframe analysis is a structural change to signal generation, not a parameter tune

---

## Proposed Config Changes

```rust
// RUN44: Multi-timeframe confirmation for ISO shorts
pub const ISO_MTF_CONFIRM_1H: bool = true;   // require 1h RSI confirmation
pub const ISO_MTF_CONFIRM_4H: bool = true;   // require 4h RSI confirmation
pub const ISO_1H_RSI_THRESHOLD: f64 = 65.0;  // 1h RSI must exceed this
pub const ISO_4H_RSI_THRESHOLD: f64 = 70.0;  // 4h RSI must exceed this
```

**New indicator types to add to `indicators.rs`:**
```rust
pub struct Ind1h {
    pub rsi: f64,
    pub rsi_valid: bool,
}

pub struct Ind4h {
    pub rsi: f64,
    pub rsi_valid: bool,
}
```

**`fetcher.rs` changes:** Add `fetch_1h_indicators()` and `fetch_4h_indicators()` functions that compute RSI(14) on 1h and 4h candles. Fetch 100 1h candles (~4 days) and 100 4h candles (~16 days) per coin per cycle.

**`state.rs` changes:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub ind_1h: Option<Ind1h>,
    pub ind_4h: Option<Ind4h>,
    pub last_1h_fetch: Option<std::time::Instant>,
    pub last_4h_fetch: Option<std::time::Instant>,
}
```

**`strategies.rs` change — `iso_short_entry` extended:**
```rust
pub fn iso_short_entry(
    ind: &Ind15m,
    strat: IsoShortStrat,
    ctx: &MarketCtx,
    ind_1h: Option<&Ind1h>,
    ind_4h: Option<&Ind4h>,
) -> bool {
    // Multi-timeframe confirmation gate
    if config::ISO_MTF_CONFIRM_1H {
        if let Some(h1) = ind_1h {
            if !h1.rsi_valid || h1.rsi < config::ISO_1H_RSI_THRESHOLD {
                return false;
            }
        } else {
            return false;  // no 1h data yet, suppress signal
        }
    }
    if config::ISO_MTF_CONFIRM_4H {
        if let Some(h4) = ind_4h {
            if !h4.rsi_valid || h4.rsi < config::ISO_4H_RSI_THRESHOLD {
                return false;
            }
        } else {
            return false;  // no 4h data yet, suppress signal
        }
    }
    // ... rest of existing iso_short_entry logic ...
}
```

---

## Validation Method

### RUN44.1 — Multi-Timeframe Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m, 1h, and 4h OHLCV for all 18 coins, 1-year dataset (same 1-year period)

**Implementation:** The backtester must fetch and compute 1h and 4h RSI alongside 15m indicators. This is the most data-intensive RUN so far — 3 timeframes × 18 coins.

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `ISO_MTF_CONFIRM_1H` | [false, true] |
| `ISO_MTF_CONFIRM_4H` | [false, true] |
| `ISO_1H_RSI_THRESHOLD` | [60.0, 65.0, 70.0] |
| `ISO_4H_RSI_THRESHOLD` | [65.0, 70.0, 75.0] |

**Per coin:** 2 × 2 × 3 × 3 = 36 configs × 18 coins = 648 backtests

**Also test:** Is the effect specific to certain ISO short strategies (IsoRsiExtreme vs IsoDivergence vs IsoRelativeZ)? Some strategies may benefit more from MTF confirmation.

**Key metrics:**
- `WR_delta = filtered_ISO_WR% − baseline_ISO_WR%`
- `PF_delta = filtered_ISO_PF − baseline_ISO_PF`
- `trade_count_reduction = % of ISO shorts blocked by the filter`
- `false_block_rate = % of blocked ISO shorts that would have been winners`

### RUN44.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best MTF confirmation params per coin
2. Test: evaluate on held-out month with those params

**Pass criteria:**
- ≥ 10/18 coins show positive OOS WR% delta vs baseline ISO shorts
- ISO short P&L per coin in test half ≥ baseline (despite fewer trades)
- False block rate < 30% (filter doesn't block too many good trades)

### RUN44.3 — Combined Comparison

Side-by-side ISO short performance:

| Metric | Baseline ISO Shorts (v16) | MTF-Confirmed ISO Shorts | Delta |
|--------|--------------------------|-------------------------|-------|
| ISO WR% | X% | X% | +Ypp |
| ISO PF | X.XX | X.XX | +0.XX |
| ISO P&L | $X | $X | +$X |
| ISO Max DD | X% | X% | -Ypp |
| ISO Trade Count | N | M | −K (−X%) |
| Trades Blocked | — | N | — |
| False Block Rate | — | X% | — |
| 1h RSI at Entry (avg) | X% | X% | +Ypp |
| 4h RSI at Entry (avg) | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Higher timeframe data is stale in live trading:** 1h RSI updates every hour; 4h RSI updates every 4 hours. The MTF signal may indicate an opportunity that has already resolved by the time it's detected.
2. **Multi-timeframe alignment is rare:** If overbought on all three timeframes is uncommon, the filter will block nearly all ISO shorts, leaving ISO short mode with almost no trades.
3. **Implementation complexity:** Fetching and managing 1h and 4h data adds significant complexity to the data pipeline. If the effect size is small, this cost may not be worth it.
4. **Wrong direction:** ISO shorts work because they catch coin-specific overbought — requiring broader confirmation may filter out exactly the trades that work best.

---

## Why It Could Succeed

1. **Institutional-standard technique:** Multi-timeframe confirmation is a cornerstone of technical analysis. Applying it to ISO shorts leverages established trading wisdom.
2. **Highest-impact subset of ISO shorts:** The coins most likely to benefit (ATOM, SHIB, LTC — identified as strong mean-reverters in RUN28) may also show the cleanest MTF alignment before reverting.
3. **RSI thresholds are generous:** RSI > 65 on 1h and RSI > 70 on 4h are not extreme — most genuine overbought events should clear these thresholds without much filtering.

---

## Comparison to Baseline

| | Current ISO Shorts (v16) | RUN44 MTF-Confirmed ISO Shorts |
|--|--|--|
| Timeframe | 15m only | 15m + 1h + 4h RSI |
| Signal quality | Single-point | Confirmed across 3 timeframes |
| ISO trade count | N | N × (1 − block_rate) |
| Expected WR% | ~35% | +3–8pp |
| Data required | 15m only | 15m + 1h + 4h |
| Fetch complexity | Current | +2 new fetch streams |
| Implementation | strategies.rs only | strategies.rs + fetcher.rs + state.rs |
