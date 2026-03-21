# RUN102 — TWAP Entry Execution: Accumulate Positions Over Time to Reduce Entry Timing Risk

## Hypothesis

**Named:** `twap_entry`

**Mechanism:** COINCLAW currently enters positions immediately at the current price when a signal fires. But the signal fires at a specific moment — if there's momentary volatility or a price spike at that instant, we enter at a bad price. The TWAP Entry Execution spreads the entry over multiple bars: when a signal fires, accumulate the full position over the next N bars at the volume-weighted average price of each bar, reducing the risk of entering at an extreme single-price moment.

**TWAP Entry Execution:**
- When a regime entry signal fires:
  - Set `twap_target_size` = full position size
  - Set `twap_bars_remaining` = TWAP_BARS (e.g., 3 bars)
  - Set `twap_start_price` = current price
  - For each of the next N bars, accumulate 1/N of the total size at that bar's VWAP price
  - The position is "twap-building" during this period
  - If during TWAP accumulation the price moves significantly against us (>TWAP_SL_TRIGGER, e.g., 0.5%), cancel the remaining TWAP and skip the entry
- TWAP applies to regime entries only; scalp entries are immediate (they require fast execution)

**Why this is not a duplicate:**
- No prior RUN has addressed entry execution methodology — all prior RUNs assume immediate entry at current price
- TWAP is a standard institutional execution algorithm used to minimize market impact and timing risk
- This is fundamentally different from all prior RUNs which modify entry conditions or position sizing, not execution methodology

**Mechanistic rationale:** Entering at the exact moment a signal fires can mean entering at a price spike caused by momentary order flow imbalance. TWAP smooths the entry price over time, reducing the variance of the entry price. For mean-reversion strategies that rely on precise entry timing, TWAP reduces the risk of systematically entering at bad prices due to short-term volatility.

---

## Proposed Config Changes

```rust
// RUN102: TWAP Entry Execution
pub const TWAP_ENTRY_ENABLE: bool = true;
pub const TWAP_BARS: u32 = 3;                // accumulate over 3 bars
pub const TWAP_SL_TRIGGER: f64 = 0.005;       // cancel TWAP if price moves 0.5% against during accumulation
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub twap_active: bool,                // true = currently accumulating via TWAP
    pub twap_bars_remaining: u32,         // bars left in TWAP accumulation
    pub twap_total_size: f64,           // total size to accumulate (in units)
    pub twap_accumulated: f64,          // size accumulated so far
    pub twap_start_price: f64,          // price at TWAP start (for SL trigger)
    pub twap_entry_prices: Vec<f64>,   // prices at which each partial fill occurred
}
```

**`engine.rs` — TWAP accumulation and execution:**
```rust
/// Check if TWAP entry should be initiated for a coin with a valid signal.
fn initiate_twap(state: &mut SharedState, ci: usize) {
    let cs = &mut state.coins[ci];
    if !config::TWAP_ENTRY_ENABLE { return; }
    if cs.pos.is_some() { return; }  // already have a position

    let cfg = &COINS[cs.config_idx];
    let ind = match &cs.ind_15m {
        Some(i) => i.clone(),
        None => return,
    };

    // Check if there's a valid regime signal (long or short)
    let has_signal = check_regime_signal(&ind, cfg, &state.market_mode);
    if !has_signal { return; }

    // Initiate TWAP
    let price = ind.p;
    let risk = config::RISK;
    let trade_amt = cs.bal * risk;
    let total_size = (trade_amt * config::LEVERAGE) / price;

    cs.twap_active = true;
    cs.twap_bars_remaining = config::TWAP_BARS;
    cs.twap_total_size = total_size;
    cs.twap_accumulated = 0.0;
    cs.twap_start_price = price;
    cs.twap_entry_prices.clear();

    cs.active_strat = Some(cfg.long_strat.to_string());
}

/// Process TWAP accumulation for a coin.
fn process_twap(state: &mut SharedState, ci: usize) {
    let cs = &mut state.coins[ci];
    if !cs.twap_active { return; }

    let current_price = match &cs.ind_15m {
        Some(ind) => ind.p,
        None => return,
    };

    // Check TWAP SL trigger: if price moved against us by threshold, cancel TWAP
    let price_move = if cs.twap_start_price > 0.0 {
        (current_price - cs.twap_start_price) / cs.twap_start_price
    } else {
        0.0
    };
    // For long TWAP: if price dropped more than TWAP_SL_TRIGGER, cancel
    // For short: if price rose more than TWAP_SL_TRIGGER, cancel
    if price_move < -config::TWAP_SL_TRIGGER || price_move > config::TWAP_SL_TRIGGER {
        // Cancel TWAP — price moved against us significantly during accumulation
        cancel_twap(state, ci);
        return;
    }

    // Accumulate one bar's worth of size
    let bar_fraction = cs.twap_total_size / config::TWAP_BARS as f64;
    cs.twap_accumulated += bar_fraction;
    cs.twap_entry_prices.push(current_price);
    cs.twap_bars_remaining -= 1;

    if cs.twap_bars_remaining == 0 {
        // TWAP complete — finalize position at VWAP
        finalize_twap(state, ci);
    }
}

/// Cancel an active TWAP entry.
fn cancel_twap(state: &mut SharedState, ci: usize) {
    let cs = &mut state.coins[ci];
    cs.twap_active = false;
    cs.twap_bars_remaining = 0;
    cs.twap_accumulated = 0.0;
    cs.twap_entry_prices.clear();
    cs.active_strat = None;
}

/// Finalize TWAP accumulation into a single position.
fn finalize_twap(state: &mut SharedState, ci: usize) {
    let cs = &mut state.coins[ci];
    if !cs.twap_active { return; }

    // Compute VWAP from all entry prices
    let total_cost: f64 = cs.twap_entry_prices.iter().sum();
    let vwap = if !cs.twap_entry_prices.is_empty() {
        total_cost / cs.twap_entry_prices.len() as f64
    } else {
        cs.twap_start_price
    };

    // Open the full position at VWAP
    open_position(state, ci, vwap, "TWAP", Direction::Long, TradeType::Regime);

    cs.twap_active = false;
    cs.twap_bars_remaining = 0;
    cs.twap_accumulated = 0.0;
    cs.twap_entry_prices.clear();
}

// In the main tick — process TWAP before checking new entries:
fn tick(state: &mut SharedState) {
    // ... existing logic ...

    // Process TWAP accumulation first
    for ci in 0..state.coins.len() {
        if state.coins[ci].twap_active {
            process_twap(state, ci);
        }
    }

    // Then check new entries (only if not in TWAP)
    for ci in 0..state.coins.len() {
        if !state.coins[ci].twap_active {
            // ... existing check_entry logic ...
        }
    }
}
```

---

## Validation Method

### RUN102.1 — TWAP Entry Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — immediate entry at signal price

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `TWAP_BARS` | [2, 3, 4] |
| `TWAP_SL_TRIGGER` | [0.003, 0.005, 0.008] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `twap_vs_immediate_price_diff`: average difference between TWAP entry price and immediate entry price (bps)
- `twap_cancel_rate`: % of TWAP entries cancelled due to price moving against trigger
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_entry_price_delta`: change in average entry price quality

### RUN102.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best TWAP_BARS × TRIGGER pair per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Average entry price is better (lower for longs, higher for shorts) than baseline

### RUN102.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, immediate entry) | TWAP Entry | Delta |
|--------|-------------------------------|------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Avg Entry Price (longs) | $X | $X | -N bps |
| TWAP Cancel Rate | 0% | X% | — |
| Slippage Saved | 0 bps | X bps | — |

---

## Why This Could Fail

1. **Market moves during accumulation:** During the 3-bar TWAP window, the price can move significantly. The TWAP SL trigger cancels the entry, but by then we may have missed the opportunity.
2. **Holding cash during TWAP:** While accumulating, the reserved capital is not deployed. Opportunity cost of idle capital during the TWAP window.
3. **Implementation complexity:** TWAP requires significant changes to the position tracking architecture — multiple partial fills, VWAP computation, TWAP cancellation logic.

---

## Why It Could Succeed

1. **Reduces entry price variance:** TWAP smooths the entry price, reducing the variance of entry prices across all trades. Lower variance in entry prices means more consistent outcomes.
2. **Avoids entry at spikes:** Signals can fire at momentary price spikes caused by order flow. TWAP avoids these by spreading the entry over time.
3. **Institutional standard:** TWAP is one of the most widely used execution algorithms in institutional trading precisely because it reduces market impact and timing risk.
4. **Slippage savings:** Even small improvements in entry price (1-2 bps) compound over thousands of trades into meaningful P&L improvement.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN102 TWAP Entry |
|--|--|--|
| Entry timing | Immediate at signal price | Accumulated over 3 bars at VWAP |
| Entry price | Single price point | Volume-weighted average |
| Timing risk | Full | Reduced by spreading |
| Cancel option | N/A | Cancel if price moves 0.5% against |
| Capital utilization | Immediate | Delayed during accumulation |
| Execution quality | Variable | More consistent |
