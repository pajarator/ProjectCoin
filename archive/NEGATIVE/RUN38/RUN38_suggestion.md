# RUN38 — Volume-Volatility Event Proxy: Targeted Mean Reversion in High-Energy Windows

## Hypothesis

**Named:** `event_proxy_reversion`

**Mechanism:** Cryptocurrency events (token unlocks, exchange listings, protocol upgrades) create concentrated periods of unusual volume and volatility. These "event proxy windows" — detected purely from OHLCV data as simultaneous volume and volatility spikes — represent periods where mean reversion is more likely to succeed because:
1. The price deviation from fair value is larger (more oversold/overbought)
2. Volume confirms the move is real, not noise
3. The subsequent reversion to the mean is sharper and more reliable

The hypothesis is that COINCLAW's existing mean reversion strategies (vwap_rev, bb_bounce, adr_rev, etc.) have a *higher win rate and profit factor during event proxy windows* than during normal conditions. If confirmed, COINCLAW can selectively engage only during these windows — or weight position size higher — without needing external event data.

**Why this is not a duplicate:**
- No prior RUN tested event-conditional strategy performance
- No prior RUN used volume+volatility anomaly as a trade filter
- RUN20 (momentum crowding) used 3-day momentum — this uses 15m intraday volume+vol spikes as a proxy for *known* crypto events
- RUN27/28 (momentum persistence) tested breakout continuation — this tests the opposite: reversion *during* event volatility
- RUN12 (scalp market mode) gated scalp direction by regime — this gates regime-trade *engagement* by event energy

---

## Proposed Config Changes

No changes to `config.rs` are required for the discovery run. This RUN tests a new signal-gating mechanism.

**New constants to add (if hypothesis confirms):**
```rust
// RUN38: event proxy parameters
pub const EVENT_VOL_MULT: f64 = 2.0;      // vol must be ≥ 2× rolling avg
pub const EVENT_ATR_MULT: f64 = 1.5;      // ATR must be ≥ 1.5× rolling ATR
pub const EVENT_HOLD_BARS: u32 = 8;       // window extends 8 bars after trigger
pub const EVENT_MIN_BARS_APART: u32 = 20;  // minimum bars between event windows
```

**New strategy variant to add to `strategies.rs`:**
```rust
// event_proxy_reversion: mean reversion gated by simultaneous vol+vol spike
pub fn event_proxy_long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    let vol_spike = ind.vol > ind.vol_ma * config::EVENT_VOL_MULT;
    let atr_spike = ind.atr > ind.atr_ma * config::EVENT_ATR_MULT;
    if !(vol_spike && atr_spike) { return false; }
    long_entry(ind, strat)  // delegate to existing entry logic
}
```

---

## Validation Method

### RUN38.1 — Event Proxy Discovery (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year (same dataset as prior RUNs)

**Event proxy window definition:**
- `vol_spike`: `vol[i] ≥ rolling_mean(vol, 20) × 2.0`
- `atr_spike`: `atr[i] ≥ rolling_mean(atr, 20) × 1.5`
- `event_window`: bars where both conditions are true, plus the next 8 bars
- `event_gap`: windows must be ≥ 20 bars apart (merge nearby spikes)

**Grid search:**
| Parameter | Values |
|-----------|--------|
| `EVENT_VOL_MULT` | [1.5, 2.0, 2.5, 3.0] |
| `EVENT_ATR_MULT` | [1.0, 1.5, 2.0] |
| `EVENT_HOLD_BARS` | [4, 8, 12] |

**Per coin:**
1. Label all bars as `in_event_window` or `normal`
2. Run COINCLAW v13 strategy per coin on all bars (baseline)
3. Run COINCLAW v13 strategy per coin *only during event windows* (test)
4. Compare: WR%, PF, total P&L, max DD, trade count
5. Also measure: if event windows cover ~20% of bars but contain ~40%+ of profitable trades, that's a strong signal

**Scoring:**
- `edge = test_WR% − baseline_WR%`
- `pct_profitable_trades_in_event = event_trades / total_trades`
- Winner = config with highest `edge × sqrt(event_trades)` (rewards both high edge and sufficient sample)

**Expected outcome:**
- Hypothesis: event window trades have +5–10pp higher WR% than baseline
- Rationale: large vol+ATR spikes typically accompany post-event reversions (unlock cliff selloff → bounce, listing pump → dump)
- If WR% during event windows is ≥ 44% (breakeven for current system), this confirms the hypothesis

### RUN38.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo) — same structure as prior RUNs

For each window:
1. On train half: find best `EVENT_VOL_MULT × EVENT_ATR_MULT × EVENT_HOLD_BARS` per coin
2. Evaluate on test half with those params
3. Compare: event-filtered vs baseline (OOS)

**Pass criteria:**
- ≥ 10/18 coins show positive OOS WR% delta vs baseline
- Portfolio event-filtered P&L ≥ baseline Portfolio P&L on test half

### RUN38.3 — Combined Comparison

Side-by-side:
- Current COINCLAW v16 (all bars, no event filter)
- Proposed: COINCLAW v16 + event proxy gate (only trade during event windows)

**Per-coin metrics:** WR%, PF, total_PnL, max_DD, trade_count, event_coverage_pct
**Portfolio metrics:** aggregate P&L, aggregate WR%, Sharpe ratio, max DD

**Comparison table:**

| Metric | Baseline (v16) | Event-Filtered | Delta |
|--------|---------------|----------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K (−X%) |
| Event Coverage | 100% | Z% | — |

---

## Implementation Notes

### Event Proxy Signal Logic
```rust
// Pseudocode for event_proxy_long_entry (strategies.rs)
fn is_event_bar(ind: &Ind15m, vol_mult: f64, atr_mult: f64) -> bool {
    !ind.vol_ma.is_nan() && !ind.atr_ma.is_nan()
        && ind.vol >= ind.vol_ma * vol_mult
        && ind.atr >= ind.atr_ma * atr_mult
}

pub fn event_proxy_long_entry(ind: &Ind15m, strat: LongStrat,
                              vol_mult: f64, atr_mult: f64, hold_bars: u32) -> bool {
    if !is_event_bar(ind, vol_mult, atr_mult) { return false; }
    // Entry only fires during event window AND holds for hold_bars minimum
    long_entry(ind, strat)
}
```

### Integration with COINCLAW
If RUN38.2 passes:
- Add `event_proxy` mode to `config.rs` as a boolean flag `ENABLE_EVENT_PROXY`
- When enabled, regime and scalp entries are only taken during event proxy windows
- Scalp entries (1m) use 1m vol+ATR anomaly instead of 15m
- `EVENT_*` constants become new top-level config parameters

---

## Why This Could Fail

1. **Vol+ATR spikes are trend-continuation signals, not reversal signals** — in a true breakout, high vol and high volume accompany directional moves, meaning the event proxy might filter IN trend-following trades that get stopped out by mean-reversion SL=0.3%
2. **Sample size** — event windows may be rare enough that even with 1 year of data, trade counts are too low for statistical significance
3. **Crypto events don't reliably produce intrabar vol spikes at 15m resolution** — some events are known in advance and price moves gradually before the event

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN38 Event Proxy |
|--|--|--|
| Entry trigger | z-score, Bollinger, RSI, VWAP | Same + event window gate |
| Stop loss | 0.3% fixed | 0.3% fixed (unchanged) |
| Event data | None | Volume+ATR anomaly proxy |
| Trade frequency | All bars | Only event windows |
| Expected WR% | ~35–40% | +5–10pp during events |
| Expected PF | ~0.85–0.95 | Higher during events |
