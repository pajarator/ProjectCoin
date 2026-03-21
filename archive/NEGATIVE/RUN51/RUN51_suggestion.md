# RUN51 — Drawdown-Contingent Stop Loss Widening: Dynamic SL Based on Cumulative P&L

## Hypothesis

**Named:** `drawdown_contingent_sl`

**Mechanism:** COINCLAW currently uses a fixed 0.3% SL for all regime trades, regardless of the coin's cumulative trading history. If a coin has hit 5 consecutive SLs (cumulative drawdown > 1.5%), the market regime for that coin may have changed — it's trending and mean reversion isn't working. A fixed 0.3% SL during a trending period gets hit repeatedly.

The hypothesis is that **when a coin is in cumulative drawdown, the SL should widen** to give trades more room to work during what may be a regime change:

```
cumulative_dd = (peak_balance - current_balance) / peak_balance
if cumulative_dd >= DD_THRESHOLD_1:
    effective_sl = SL × SL_WIDEN_FACTOR_1  (e.g., 1.5× → 0.45%)
if cumulative_dd >= DD_THRESHOLD_2:
    effective_sl = SL × SL_WIDEN_FACTOR_2  (e.g., 2.0× → 0.60%)
```

This is different from RUN34's consecutive-SL escalation because:
- RUN34: tracks consecutive SL hits → cooldown escalation (suppression mechanism)
- RUN51: tracks cumulative drawdown → SL widening (accommodation mechanism)

**Why this is not a duplicate:**
- RUN34 tested SL widening based on consecutive SL count — this tests SL widening based on cumulative P&L level
- No prior RUN changed any parameter dynamically based on cumulative trading performance
- Consecutive SL count and cumulative drawdown are different signals: 3 small wins + 1 big loss can produce high drawdown with 0 consecutive SLs
- This is a dynamic parameter adjustment based on trading history, a fundamentally different mechanism

---

## Proposed Config Changes

```rust
// RUN51: Drawdown-Contingent Stop Loss Widening
pub const DD_SL_ENABLE: bool = true;
pub const DD_THRESHOLD_1: f64 = 0.05;    // 5% cumulative drawdown → first widening
pub const DD_THRESHOLD_2: f64 = 0.10;    // 10% cumulative drawdown → second widening
pub const DD_SL_WIDEN_FACTOR_1: f64 = 1.5;  // SL × 1.5 at 5% DD (0.3% → 0.45%)
pub const DD_SL_WIDEN_FACTOR_2: f64 = 2.0;  // SL × 2.0 at 10% DD (0.3% → 0.60%)
pub const DD_SL_RESET_ON_PROFIT: bool = true;  // reset DD tracking after a winning trade
```

**`state.rs` — add drawdown tracking to CoinPersist:**
```rust
pub struct CoinPersist {
    pub bal: f64,
    pub pos: Option<Position>,
    pub trades: Vec<TradeRecord>,
    pub candles_held: u32,
    pub cooldown: u32,
    pub consecutive_sl: u32,
    pub peak_balance: f64,      // NEW: highest balance achieved
    pub cumulative_dd: f64,     // NEW: current cumulative drawdown (0.0 to 1.0)
}
```

**`state.rs` — update after each trade:**
```rust
impl CoinPersist {
    pub fn update_drawdown(&mut self) {
        if self.bal > self.peak_balance {
            self.peak_balance = self.bal;
        }
        if self.peak_balance > 0.0 {
            self.cumulative_dd = (self.peak_balance - self.bal) / self.peak_balance;
        }
    }
}
```

**`engine.rs` — effective SL in check_exit:**
```rust
fn effective_sl(cs: &CoinState) -> f64 {
    if !config::DD_SL_ENABLE { return config::STOP_LOSS; }
    let dd = cs.cumulative_dd;
    if dd >= config::DD_THRESHOLD_2 {
        return config::STOP_LOSS * config::DD_SL_WIDEN_FACTOR_2;
    } else if dd >= config::DD_THRESHOLD_1 {
        return config::STOP_LOSS * config::DD_SL_WIDEN_FACTOR_1;
    }
    config::STOP_LOSS
}
```

In `check_exit`, replace `pnl <= -config::STOP_LOSS` with `pnl <= -effective_sl(cs)`.

---

## Validation Method

### RUN51.1 — Drawdown-Contingent SL Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed 0.3% SL

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `DD_THRESHOLD_1` | [0.03, 0.05, 0.07, 0.10] |
| `DD_THRESHOLD_2` | [0.08, 0.10, 0.15, 0.20] |
| `DD_SL_WIDEN_FACTOR_1` | [1.25, 1.5, 2.0] |
| `DD_SL_WIDEN_FACTOR_2` | [1.5, 2.0, 2.5] |
| `DD_SL_RESET_ON_PROFIT` | [true, false] |

**Per coin:** 4 × 4 × 3 × 3 × 2 = 288 configs × 18 coins = 5,184 backtests

**Key metrics:**
- `avg_effective_sl`: average SL% used at entry (shows how often widening activates)
- `max_dd_delta`: reduction in max drawdown vs baseline
- `PF_delta`: profit factor change vs baseline
- `trade_count_delta`: change in number of trades (wider SL → fewer SL hits → more trades held to signal exit)
- `avg_held_bars_delta`: wider SL → longer holds → more bars

### RUN51.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best DD thresholds and widen factors per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Portfolio OOS max_DD < baseline portfolio max_DD (primary)
- Portfolio OOS P&L ≥ 90% of baseline (don't sacrifice too much)

### RUN51.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | DD-Contingent SL | Delta |
|--------|---------------|-----------------|-------|
| Total P&L | $X | $X | +$X |
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Max DD | X% | X% | −Ypp |
| Avg Effective SL | 0.30% | X% | +Ypp |
| Avg Held Bars | X | X | +N |
| Trades at DD1 | — | N | — |
| Trades at DD2 | — | N | — |

---

## Why This Could Fail

1. **Wider SL means bigger losses when wrong:** Every trade that hits the wider SL loses more $. The benefit must outweigh this cost.
2. **Drawdown is backward-looking:** By the time DD > 5% has accumulated, the bad regime may already be ending. Widening SL as a response to past drawdown doesn't address the current trade's quality.
3. **Doesn't fix the underlying problem:** If the issue is that the strategy doesn't work in trending markets, widening the SL just delays the inevitable SL hit. The money is lost either way.

---

## Why It Could Succeed

1. **Trended markets and mean reversion are incompatible:** During a sustained uptrend, every mean-reversion entry gets hit by the widening SL. A 0.45% SL in a trending market gets hit; a 0.30% SL gets hit faster. Widening to 0.6% may allow the trade to survive the trend until it mean-reverts.
2. **Fewer SL hits → more signal exits:** Wider SL means the trade isn't stopped out by noise. More trades reach the SMA/Z0 exit, which may produce better exits.
3. **Complements cooldown escalation:** If RUN34's cooldown doesn't fire (because trades exit via signal before consecutive SLs accumulate), the drawdown-contingent SL can provide a safety net.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN51 DD-Contingent SL |
|--|--|--|
| Stop loss | Fixed 0.30% | 0.30%–0.60% based on DD |
| DD tracking | None | Peak balance tracking |
| Dynamic parameters | None | Cumulative P&L responsive |
| Max SL | 0.30% | 0.60% (at 10%+ DD) |
| Expected Max DD | X% | −20–40% |
| Expected Avg Loss | $X | +$X (bigger losses but fewer) |
| Expected WR% | X% | +1–3pp |
