# RUN98 — Intraday Max Drawdown Clip: Exit Positions When Unrealized Loss Exceeds Daily Threshold

## Hypothesis

**Named:** `intraday_dd_clip`

**Mechanism:** COINCLAW currently uses a fixed 0.30% SL as the maximum loss per trade. But a coin can move 2-3% against us intraday without hitting the SL — the SL is based on the entry price, but if the market gaps down overnight or has a sudden volatility spike, the position can accumulate large unrealized losses before recovering. The Intraday Max Drawdown Clip adds a daily loss ceiling: if any position's unrealized loss exceeds `INTRADAY_DD_CAP` (e.g., 1.0%) within a single UTC day, exit immediately regardless of how far we are from the entry price.

**Intraday Max Drawdown Clip:**
- Track per-coin `daily_high` and `daily_low` (resets at UTC midnight)
- For each open position, compute `intraday_drawdown = (daily_high - current_price) / daily_high` for longs
- When `intraday_drawdown >= INTRADAY_DD_CAP` (e.g., 1.0%):
  - Exit immediately with reason `DD_CLIP`
  - This is stricter than SL — it catches the total intraday drawdown, not just the drawdown from entry
- Reset at UTC midnight: `daily_high = entry_price`, `daily_low = entry_price`

**Why this is not a duplicate:**
- RUN51 (DD SL widening) sized positions by per-coin drawdown — this exits positions based on INTRADAY drawdown from the daily high
- RUN81 (equity circuit breaker) halts entries during portfolio drawdown — this clips individual position drawdowns within a day
- No prior RUN has implemented a daily drawdown ceiling that forces exit regardless of entry price

**Mechanistic rationale:** A position entered at $100 might have a daily high of $101, then suddenly drop to $97 due to a market-wide selloff. The SL at $99.70 hasn't been hit, but the position has already lost 3% from its daily high. The Intraday DD Clip exits at $97, preventing further losses from a market that has clearly broken. This is particularly valuable for overnight moves or sudden volatility spikes where the fixed SL is too slow to react.

---

## Proposed Config Changes

```rust
// RUN98: Intraday Max Drawdown Clip
pub const INTRADAY_DD_CLIP_ENABLE: bool = true;
pub const INTRADAY_DD_CAP: f64 = 0.010;  // 1.0% intraday drawdown from daily high triggers exit
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub daily_high: f64,      // highest price since UTC midnight
    pub daily_low: f64,       // lowest price since UTC midnight
    pub daily_reset_bar: u32, // bar of last UTC midnight reset
}

// Position struct — track daily high at entry:
pub struct Position {
    // ... existing fields ...
    pub entry_daily_high: f64,  // daily high at time of entry (for DD measurement)
}
```

**`engine.rs` — update_daily_highs and check_dd_clip:**
```rust
/// Reset daily high/low at UTC midnight.
fn check_daily_reset(state: &mut SharedState, current_bar: u32) {
    use chrono::Timelike;
    let utc_hour = chrono::Utc::now().hour();

    for cs in &mut state.coins {
        // Reset at UTC midnight (first bar of new day)
        if utc_hour == 0 && cs.daily_reset_bar != current_bar {
            if let Some(ind) = &cs.ind_15m {
                cs.daily_high = ind.p;
                cs.daily_low = ind.p;
                cs.daily_reset_bar = current_bar;
            }
        }
    }
}

/// Update daily high/low for coins with open positions.
fn update_daily_highs(state: &mut SharedState) {
    for cs in &mut state.coins {
        if let Some(ref ind) = cs.ind_15m {
            if ind.p > cs.daily_high {
                cs.daily_high = ind.p;
            }
            if ind.p < cs.daily_low {
                cs.daily_low = ind.p;
            }
        }
    }
}

/// Check if any position exceeds the intraday drawdown cap.
fn check_dd_clip(state: &mut SharedState, ci: usize) -> bool {
    if !config::INTRADAY_DD_CLIP_ENABLE { return false; }

    let cs = &state.coins[ci];
    let pos = match &cs.pos {
        Some(p) => p,
        None => return false,
    };

    let current_price = match &cs.ind_15m {
        Some(ind) => ind.p,
        None => return false,
    };

    // Compute intraday drawdown from daily high
    let intraday_dd = if pos.dir == "long" {
        (cs.daily_high - current_price) / cs.daily_high
    } else {
        (current_price - cs.daily_low) / cs.daily_low
    };

    if intraday_dd >= config::INTRADAY_DD_CAP {
        return true;
    }

    false
}

// In the main tick — update daily highs and check clip:
fn tick(state: &mut SharedState, current_bar: u32) {
    check_daily_reset(state, current_bar);
    update_daily_highs(state);

    // ... existing checks ...

    // Check DD clip before checking entry
    for ci in 0..state.coins.len() {
        if check_dd_clip(state, ci) {
            let price = state.coins[ci].ind_15m.as_ref().map(|i| i.p).unwrap_or(0.0);
            let trade_type = state.coins[ci].pos.as_ref()
                .and_then(|p| p.trade_type)
                .unwrap_or(TradeType::Regime);
            close_position(state, ci, price, "DD_CLIP", trade_type);
        }
    }
}
```

---

## Validation Method

### RUN98.1 — Intraday DD Clip Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed SL only, no intraday DD clip

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `INTRADAY_DD_CAP` | [0.008, 0.010, 0.015, 0.020] |

**Per coin:** 4 configs × 18 coins = 72 backtests

**Key metrics:**
- `dd_clip_rate`: % of regime trades exited by DD_CLIP
- `avg_dd_at_clip`: average drawdown at time of DD_CLIP exit
- `dd_clip_vs_sl_comparison`: how many DD_CLIP exits vs SL exits, and their relative PnL
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline

### RUN98.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best INTRADAY_DD_CAP per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- DD clip activates at least 3 times per quarter (meaningful use)
- Max drawdown reduced vs baseline

### RUN98.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, SL only) | Intraday DD Clip | Delta |
|--------|------------------------|-----------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| DD Clip Exits | 0 | X | — |
| Avg DD at Clip | — | X% | — |
| SL Exits | X | X | -N |
| Worst Intraday Loss (no clip) | — | -X% | — |

---

## Why This Could Fail

1. **Normal volatility triggers clips:** During high-volatility days, coins can swing 1-2% intraday without it being a real breakdown. The clip may exit positions that would have recovered by end of day.
2. **Resets at UTC midnight may not align with market:** If a position is opened at 23:00 UTC and the market drops at 23:30, the daily high is only 30 minutes old — the 1% clip fires immediately, cutting a trade that had little chance to recover.
3. **Overriding SL with clip:** The DD clip is tighter than SL (1.0% vs 0.30%). This effectively replaces the SL as the primary exit for most losing trades, changing the system's risk profile significantly.

---

## Why It Could Succeed

1. **Prevents blowups from overnight moves:** Crypto can gap significantly between daily closes and opens. A position that was fine at close could open 3% lower. The DD clip catches this immediately.
2. **Limits intraday worst case:** Rather than waiting for the 0.30% SL (which requires a sustained move), the DD clip limits the maximum intraday drawdown to the cap. This is a more direct measure of risk.
3. **Simple and interpretable:** One threshold, one check per bar. Clear logic that limits worst-case outcomes.
4. **Institutional practice:** Daily VaR-style limits are standard — this is a simplified version of that concept.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN98 Intraday DD Clip |
|--|--|--|
| Maximum intraday loss | 0.30% SL (from entry) | 1.0% from daily high |
| Overnight gap handling | SL only | DD clip catches gaps |
| Exit reason | SL, SMA, Z0 | SL, SMA, Z0, DD_CLIP |
| Worst intraday scenario | -0.30% from entry | -1.0% from daily high |
| Risk control | Fixed SL | Daily drawdown ceiling |
