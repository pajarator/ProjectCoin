# RUN58 — Post-Event Gap Fill Strategy: Exploiting CME Gap Dynamics in Crypto

## Hypothesis

**Named:** `cme_gap_fill`

**Mechanism:** Traditional markets (including crypto via CME futures) exhibit "gap fill" behavior — price tends to revert to fill the gap created by overnight or weekend moves. In crypto, CME Bitcoin futures create a weekly "institutional gap" around the Friday 5pm ET close to Monday 6pm ET open. While crypto itself trades 24/7, these institutional gaps create temporary dislocations that tend to fill.

**For altcoins:** Since most altcoins don't have their own futures, they often exhibit correlated gap behavior with BTC. When BTC gaps up/down at the open of the CME week, altcoins tend to follow, creating tradable mean-reversion opportunities back toward the pre-gap level.

**The trade:**
1. At Monday open, measure the CME BTC gap: `(BTC_open_Monday − BTC_close_Friday) / BTC_close_Friday`
2. If gap > GAP_THRESHOLD (e.g., > 1.0%): expect a fill — fade the gap by SHORTING with a mean-reversion target at the Friday close price
3. If gap < -GAP_THRESHOLD: expect a fill — fade the gap by BUYING with a mean-reversion target at Friday close price
4. Stop loss: entry ± 0.5% (gaps rarely overshoot by more than this)

**Why this is not a duplicate:**
- No prior RUN has tested gap-fill dynamics
- No prior RUN has used CME futures calendar structure as a trade signal
- No prior RUN has specifically targeted Monday open mean-reversion
- This is a pure market microstructure play, not an indicator-based strategy

---

## Proposed Config Changes

```rust
// RUN58: CME Gap Fill Strategy
pub const CME_GAP_ENABLE: bool = true;
pub const CME_GAP_THRESHOLD: f64 = 0.010;   // 1.0% gap threshold
pub const CME_GAP_SL: f64 = 0.005;         // 0.5% stop loss on gap fade
pub const CME_GAP_TP: f64 = 0.015;         // 1.5% take profit (fill target)
pub const CME_GAP_MAX_HOLD: u32 = 48;       // ~12 hours at 15m bars (should fill within day)
```

**New strategy module:**
```rust
// strategies.rs — CME gap fill entry
pub fn cme_gap_fill_entry(
    btc_candle_15m: &Candle,     // current BTC candle (for gap measurement)
    btc_prev_candle: &Candle,    // Friday's last candle
) -> Option<(Direction, &'static str)> {
    if !config::CME_GAP_ENABLE { return None; }

    let gap_pct = (btc_candle_15m.o - btc_prev_candle.c) / btc_prev_candle.c;

    if gap_pct.abs() < config::CME_GAP_THRESHOLD {
        return None;  // no significant gap
    }

    if gap_pct > 0.0 {
        // Gap up — expect fill → SHORT with TP = Friday close
        Some((Direction::Short, "cme_gap_fill"))
    } else {
        // Gap down — expect fill → LONG with TP = Friday close
        Some((Direction::Long, "cme_gap_fill"))
    }
}
```

---

## Validation Method

### RUN58.1 — CME Gap Fill Backtest (Rust, focused on BTC + major alts)

**Data:** 15m OHLCV for BTC, ETH, and top 5 alts by volume, 1-year dataset. Identify Friday close and Monday open bars.

**Gap identification:**
- Friday 23:45 UTC 15m candle close = "pre-gap" reference
- Monday 00:00 UTC candle open = "post-gap open"
- Gap% = (Monday_open − Friday_close) / Friday_close

**Per coin:** BTC + 5 alts (ETH, SOL, XRP, ADA, LINK)

**Baseline:** Current COINCLAW v16 with CME gap layer added

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CME_GAP_THRESHOLD` | [0.005, 0.008, 0.010, 0.015, 0.020] |
| `CME_GAP_SL` | [0.003, 0.005, 0.007] |
| `CME_GAP_TP` | [0.010, 0.015, 0.020, 0.025] |
| `CME_GAP_MAX_HOLD` | [24, 48, 72, 96] |

**Per coin:** 5 × 3 × 4 × 4 = 240 configs × 6 coins = 1,440 backtests

**Also test:** Does adding CME gap fill as an *overlay* on top of COINCLAW (i.e., these are additional trades, not replacements) improve portfolio P&L?

**Key metrics:**
- `gap_fill_rate`: % of gaps that fully fill (price returns to Friday close)
- `avg_fill_depth`: how far beyond the Friday close does the fill go (overshoot %)?
- `gap_PnL`: total P&L from gap fill trades
- `gap_WR`: win rate on gap fill trades
- `gap_PF`: profit factor on gap fill trades

### RUN58.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `CME_GAP_THRESHOLD × CME_GAP_TP × CME_GAP_SL` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 4/6 coins (gap-fillable coins) show positive OOS P&L from gap fills
- Gap fill WR% ≥ 55% (higher bar for this strategy type)
- Portfolio gap P&L ≥ baseline

### RUN58.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | With CME Gap Layer | Delta |
|--------|---------------|------------------|-------|
| Total P&L | $X | $X | +$X |
| BTC P&L | $X | $X | +$X |
| ETH P&L | $X | $X | +$X |
| Gap Fill WR% | — | X% | — |
| Gap Fill PF | — | X.XX | — |
| Gap Fill Rate | — | X% | — |
| Avg Gap Size | X% | X% | — |
| Trades/Week | — | N | — |

---

## Why This Could Fail

1. **Crypto doesn't always respect CME gaps:** Unlike equity index futures (SPX), crypto trades 24/7 and may not experience the same institutional-driven gaps. CME Bitcoin gaps may already be "filled" by the time Monday trading begins.
2. **Gaps are rare:** With ~52 Fridays/year, there's only ~52 gap opportunities per year. With a 1% threshold, maybe 10-15 qualify. Not enough data for statistical significance.
3. **Altcoin correlation to BTC gaps is weak:** Altcoins don't always follow BTC's CME gap direction. The correlation may not be strong enough to trade reliably.

---

## Why It Could Succeed

1. **Gap fill is one of the most well-documented phenomena in all markets:** It works in equities, futures, and FX. There's no reason crypto CME would be different.
2. **Institutional crypto participation is growing:** More institutional money enters through CME futures, bringing traditional market dynamics (gap fill) into crypto.
3. **Clear entry/exit:** Gap is measured objectively; TP and SL are fixed. No subjectivity.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN58 CME Gap Fill Layer |
|--|--|--|
| Trade frequency | Continuous | ~10-15/week |
| Entry signal | Indicators | CME gap detection |
| Hold duration | Variable | ~2-12 hours |
| Target | Variable | Friday close price |
| Stop loss | 0.3% | 0.3–0.7% |
| Market hours | All | Monday open focus |
