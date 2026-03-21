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

## Validation Method

### RUN38.1 — Event Proxy Discovery (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 5-month (same dataset as prior RUNs)

**Event proxy window definition:**
- `vol_spike`: `vol[i] ≥ rolling_mean(vol, 20) × EVENT_VOL_MULT`
- `atr_spike`: `atr[i] ≥ rolling_mean(atr, 20) × EVENT_ATR_MULT`
- `event_window`: bars where both conditions are true, plus the next `EVENT_HOLD_BARS` bars
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

## Results — NEGATIVE

**RUN38.1 Grid Search (37 configs × 18 coins, 5-month 15m data)**

### Results Summary

| Config | BasePnL | EvtPnL | ΔPnL | BaseWR | EvtWR | Edge | Score |
|--------|---------|--------|------|--------|-------|------|-------|
| **vol0.0_atr0.0_hold0 (baseline)** | **+$4.9** | +$4.9 | +$0.0 | 48.1% | 48.1% | — | — |
| vol2.0_atr1.0_hold8 | +$4.9 | +$3.4 | -$1.4 | 48.1% | 50.2% | +2.2pp | 263.6 |
| vol2.5_atr1.0_hold8 | +$4.9 | -$4.6 | -$9.4 | 48.1% | 49.2% | +1.1pp | 106.6 |
| vol2.0_atr1.0_hold4 | +$4.9 | -$0.0 | -$4.9 | 48.1% | 50.2% | +2.2pp | 244.6 |
| vol3.0_atr2.0_hold8 | +$4.9 | +$1.4 | -$3.5 | 48.1% | 57.1% | +9.1pp | 0 |
| vol2.5_atr2.0_hold8 | +$4.9 | +$1.4 | -$3.5 | 48.1% | 57.1% | +9.1pp | 0 |

**Every single event-filtered config had lower P&L than the no-filter baseline.**

### Per-Coin Breakdown (baseline)

| Coin | Strategy | Trades | WR% | PF | P&L |
|------|----------|--------|-----|----|----|
| NEAR | VWAPRev | 357 | 49.0% | 1.24 | +$4.9 |
| DASH | OuMeanRev | 0 | — | — | $0 |
| UNI | BB_Bounce | 117 | 56.4% | 0.47 | -$4.4 |
| DOT | ADRRev | 87 | 55.2% | 1.29 | +$2.8 |
| SHIB | RSI_Rev | 168 | 45.8% | 1.30 | +$2.6 |
| ADA | ADRRev | 82 | 52.4% | 1.19 | +$1.5 |
| LINK | BB_Bounce | 105 | 50.5% | 1.06 | +$1.3 |
| ALGO | RSI_Rev | 194 | 45.9% | 1.13 | +$1.1 |
| ETH | VWAPRev | 271 | 50.2% | 0.91 | -$1.0 |
| BTC | VWAPRev | 269 | 47.6% | 1.06 | -$0.3 |

### Key Findings

1. **All 36 event configs lose more P&L than baseline.** The event proxy filter reduces trades by ~78% but does not improve quality enough to offset the opportunity cost.

2. **High ATR threshold configs (2.0×) show highest WR% (+9.1pp) but still net negative P&L.** These windows are too rare (very few event bars) to generate enough trades.

3. **Best scoring config (vol2.0_atr1.0_hold8):** +2.2pp WR edge, 50.2% WR, but P&L = +$3.4 vs +$4.9 baseline (−$1.5).

4. **The vol+ATR spike window is anti-correlated with profitable regime trades** — high vol/ATR periods coincide with trending markets where mean-reversion SL=0.3% gets hit.

### Conclusion

**NEGATIVE — No COINCLAW changes.** The event proxy window filter does not improve regime trade performance. The vol+ATR spike proxy for crypto events captures trending/volatility periods where mean-reversion strategies underperform, not periods of reversal opportunity. This aligns with the failure reason identified in the hypothesis: vol+ATR spikes are trend-continuation signals, not reversal signals.

### Files
- `run38_1_results.json` — Full grid search results (37 configs × 18 coins)
- `run38.rs` — Implementation (coinclaw/src/run38.rs)

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN38 Event Proxy |
|--|--|--|
| Entry trigger | z-score, Bollinger, RSI, VWAP | Same + event window gate |
| Stop loss | 0.3% fixed | 0.3% fixed (unchanged) |
| Event data | None | Volume+ATR anomaly proxy |
| Trade frequency | All bars | Only event windows (~22% of bars) |
| Actual WR% | 48.1% | Best: 50.2% (+2.2pp) |
| Actual P&L | +$4.9 | Best: +$3.4 (-$1.5) |
| Verdict | — | NEGATIVE |
