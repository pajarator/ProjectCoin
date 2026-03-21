# RUN74 — Daily Equity Compounding with Per-Coin Reset: Preserving Compound Growth Without Crossover Risk

## Hypothesis

**Named:** `daily_compounding_reset`

**Mechanism:** COINCLAW currently uses fixed $100 initial capital per coin with no daily reset — equity compounds over time but there's no periodic rebalancing to lock in gains. The problem: after a large winning streak, a portfolio's equity is concentrated in a few large positions. A single catastrophic loss can wipe out months of gains.

**Daily compounding with reset**:
- Each coin resets to `INITIAL_CAPITAL = $100` at the start of each trading day (at 00:00 UTC)
- The previous day's P&L is locked in (withdrawn), and trading starts fresh
- This preserves compound growth while preventing catastrophic crossovers

**Alternative — weekly compounding:**
- Reset every 7 days instead of daily
- More compounding benefit than daily, less than monthly

**Why daily reset helps:**
- Locks in daily profits before they can be given back
- Prevents runaway position concentration from large equity swings
- Forces disciplined re-investment at a known size

**Why this is not a duplicate:**
- No prior RUN has implemented equity resets
- No prior RUN has tested daily vs weekly vs monthly reset cycles
- This is fundamentally different from position sizing (RISK/RISK_FRACTION) — it changes the capital base, not the fraction

---

## Proposed Config Changes

```rust
// RUN74: Daily Equity Compounding
pub const COMPOUND_RESET_ENABLE: bool = true;
pub const COMPOUND_RESET_FREQ: u8 = 1;  // 1=daily, 7=weekly, 30=monthly
pub const COMPOUND_INITIAL_CAPITAL: f64 = 100.0;
pub const COMPOUND_CARRY_PROFITS: bool = true;  // if true, profits from reset period are accumulated
```

**`state.rs` — CoinState changes:**
```rust
pub struct CoinPersist {
    pub bal: f64,
    pub pos: Option<Position>,
    pub trades: Vec<TradeRecord>,
    pub candles_held: u32,
    pub cooldown: u32,
    pub consecutive_sl: u32,
    pub peak_balance: f64,
    pub cumulative_dd: f64,
    pub period_profit: f64,      // NEW: profit this reset period
    pub last_reset_day: u32,     // NEW: UTC day of last reset
}

impl CoinPersist {
    pub fn check_compound_reset(&mut self) {
        let current_day = chrono::Utc::now().date().and_hms(0,0,0).timestamp() / 86400;
        let days_since_reset = current_day - self.last_reset_day as i64;
        if days_since_reset >= config::COMPOUND_RESET_FREQ as i64 {
            // Lock in period profit and reset to initial capital
            self.period_profit = 0.0;
            self.bal = config::COMPOUND_INITIAL_CAPITAL;
            self.trades.clear();
            self.pos = None;
            self.last_reset_day = current_day as u32;
        }
    }
}
```

---

## Validation Method

### RUN74.1 — Compounding Reset Backtest (Rust, portfolio-level)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no reset, equity compounds continuously

**Reset frequency grid:**

| Parameter | Values |
|-----------|--------|
| `COMPOUND_RESET_FREQ` | [1=daily, 7=weekly, 30=monthly] |
| `COMPOUND_CARRY_PROFITS` | [true, false] |

**Key metrics:**
- `compounded_total_return`: total return with compounding
- `simple_total_return`: total return without compounding (baseline)
- `compounding_boost`: `compounded_total_return / simple_total_return - 1`
- `max_concentration`: maximum single-position equity concentration
- `drawdown_series`: daily equity curve for Sharpe calculation

### RUN74.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: measure compounding benefit on train
2. Test: evaluate on held-out month

**Pass criteria:**
- Compounded return ≥ simple return in test period
- Max concentration < 300% of initial (no single coin exceeds $300)

### RUN74.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no reset) | Daily Compound | Weekly Compound |
|--------|------------------------|---------------|----------------|
| Total Return | X% | X% | X% |
| Sharpe Ratio | X.XX | X.XX | X.XX |
| Max Drawdown | X% | X% | X% |
| Max Concentration | X% | X% | X% |
| Equity at Year End | $X | $X | $X |
