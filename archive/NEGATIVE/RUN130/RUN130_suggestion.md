# RUN130 — Negative Day Revenue Filter: Use Rolling N-Day Loss Frequency as Market Stress Signal

## Hypothesis

**Named:** `negday_revenue_filter`

**Mechanism:** COINCLAW uses portfolio-level drawdown tracking and circuit breakers, but these measure current P&L. The Negative Day Revenue Filter measures *how often* the portfolio has been losing money over a rolling window — a rolling loss frequency that signals market stress. When 3 out of the last 5 days closed negative, the market is in a stressed state — more choppy, more likely to whipsaw mean-reversion trades. The filter scales down position sizing and increases entry thresholds during high-loss-frequency periods, and suppresses entries entirely when all 5 of the last 5 days closed negative.

**Negative Day Revenue Filter:**
- Track daily realized P&L for the portfolio: each UTC day has a net P&L
- Compute `loss_freq = count of negative P&L days in last NEGDAY_WINDOW (e.g., 5) / NEGDAY_WINDOW`
- At `loss_freq >= NEGDAY_SUPPRESS` (e.g., 0.80 = 4/5 negative days): suppress all regime entries
- At `loss_freq >= NEGDAY_TAPER` (e.g., 0.60 = 3/5 negative days): reduce RISK × NEGDAY_RISK_SCALE, tighten z-threshold
- This measures *market stress* via recent loss frequency, different from current drawdown

**Why this is not a duplicate:**
- RUN81 (equity circuit breaker) used portfolio drawdown ≥ 15% for 10 bars — this uses rolling loss frequency over N days, measuring *how often* losing, not *how much*
- RUN87 (drawdown recovery mode) used portfolio drawdown level — this uses loss frequency (binary win/loss per day), not cumulative P&L
- RUN98 (intraday DD clip) used intraday drawdown — this uses end-of-day P&L, not intraday
- No prior RUN has used a rolling count of *negative days* as a market stress filter

**Mechanistic rationale:** When the market has produced multiple consecutive losing days, it is in a stressed state — the trend is choppy, mean-reversion signals are less reliable, and the probability of continued losses is elevated. Loss frequency is a cleaner measure of market stress than cumulative drawdown because it is binary: did the market make money today or not? A market that goes up 5% then down 5% has 0% drawdown but 2 negative days. Loss frequency captures this choppy, stressful environment that erodes mean-reversion edge even without large drawdowns.

---

## Proposed Config Changes

```rust
// RUN130: Negative Day Revenue Filter
pub const NEGDAY_FILTER_ENABLE: bool = true;
pub const NEGDAY_WINDOW: usize = 5;           // rolling window of days to track
pub const NEGDAY_SUPPRESS: f64 = 1.0;      // suppress all entries if loss_freq >= this (1.0 = all negative)
pub const NEGDAY_TAPER: f64 = 0.60;        // taper RISK if loss_freq >= this
pub const NEGDAY_RISK_SCALE: f64 = 0.70;   // multiply RISK by this during high loss-freq
pub const NEGDAY_Z_TIGHTEN: f64 = 0.20;   // tighten z-threshold by this much during stress
```

**`state.rs` — SharedState additions:**
```rust
pub struct SharedState {
    // ... existing fields ...
    pub daily_pnl_history: Vec<f64>,        // rolling N-day realized P&L per day
    pub current_day_pnl: f64,              // current UTC day's realized P&L
}
```

**`strategies.rs` — add negative day helpers:**
```rust
/// Compute rolling loss frequency: % of days with negative P&L in last window.
fn loss_frequency(state: &SharedState) -> f64 {
    let history = &state.daily_pnl_history;
    if history.len() == 0 {
        return 0.0;
    }
    let negatives = history.iter().filter(|&&pnl| pnl < 0.0).count();
    negatives as f64 / history.len() as f64
}

/// Compute effective RISK during high loss-frequency periods.
fn negday_effective_risk(state: &SharedState) -> f64 {
    if !config::NEGDAY_FILTER_ENABLE { return config::RISK; }
    let freq = loss_frequency(state);
    if freq >= config::NEGDAY_SUPPRESS {
        return 0.0;  // suppress all entries
    }
    if freq >= config::NEGDAY_TAPER {
        return config::RISK * config::NEGDAY_RISK_SCALE;
    }
    config::RISK
}

/// Compute effective z-threshold (tightened during high loss-freq).
fn negday_effective_z_thresh(base: f64, state: &SharedState) -> f64 {
    if !config::NEGDAY_FILTER_ENABLE { return base; }
    let freq = loss_frequency(state);
    if freq >= config::NEGDAY_TAPER {
        return base - config::NEGDAY_Z_TIGHTEN;  // require more extreme z
    }
    base
}

/// Check if entries should be suppressed.
fn negday_entry_suppressed(state: &SharedState) -> bool {
    if !config::NEGDAY_FILTER_ENABLE { return false; }
    loss_frequency(state) >= config::NEGDAY_SUPPRESS
}
```

**`engine.rs` — modify check_entry to apply filter:**
```rust
// In check_entry, before opening position:
if config::NEGDAY_FILTER_ENABLE && negday_entry_suppressed(&state) {
    return;  // suppress entry during extreme loss-frequency stress
}

// Apply risk scaling:
let effective_risk = negday_effective_risk(&state);

// Apply z-threshold tightening:
let z_thresh = negday_effective_z_thresh(base_z_thresh, &state);
```

---

## Validation Method

### RUN130.1 — Negative Day Revenue Filter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV + daily P&L for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no negative day filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `NEGDAY_WINDOW` | [3, 5, 7] |
| `NEGDAY_SUPPRESS` | [0.80, 1.0] |
| `NEGDAY_TAPER` | [0.50, 0.60] |
| `NEGDAY_RISK_SCALE` | [0.50, 0.70] |

**Per coin:** 3 × 2 × 2 × 2 = 24 configs × 18 coins = 432 backtests

**Key metrics:**
- `negday_suppress_rate`: % of bars with entries suppressed due to high loss-freq
- `negday_taper_rate`: % of bars with tapered risk
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_risk_delta`: change in average deployed risk
- `total_PnL_delta`: P&L change vs baseline

### RUN130.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best NEGDAY_TAPER × RISK_SCALE per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Suppression events correlate with elevated market stress
- Risk tapering reduces drawdown more than it reduces wins

### RUN130.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no negday) | Negative Day Revenue Filter | Delta |
|--------|--------------------------|--------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Suppress Events | 0% | X% | — |
| Taper Events | 0% | X% | — |
| Avg Deployed Risk | 10% | X% | -Y% |
| Stress Period Entries | X | X | -N |

---

## Why This Could Fail

1. **Day-level P&L tracking adds complexity:** COINCLAW tracks positions and trades, but daily P&L aggregation requires additional bookkeeping. This is a significant implementation change.
2. **Loss frequency doesn't predict future:** Past negative days don't cause future negative days. The market doesn't have memory in the way this filter assumes.
3. **May suppress valid entries:** During a sustained downtrend (all 5 days negative), the market may be about to bounce. Suppressing entries at the bottom misses the best opportunities.

---

## Why It Could Succeed

1. **Captures market stress:** Multiple consecutive negative days is a genuine signal of market stress — the market is choppy, mean-reversion signals are degraded, and patience is warranted.
2. **Prevents overtrading in bad regimes:** When the market has produced 4 negative days out of 5, it's telling us the environment is hostile to mean-reversion. Reducing size or suppressing entries avoids burning capital in bad regimes.
3. **Clean market-wide stress signal:** Loss frequency is a simple, interpretable number: "how bad has the last week been?" It captures stress that drawdown alone doesn't — a market that went up 5% then down 5% has 0 drawdown but 2 negative days.
4. **Easy to understand:** Unlike complex multi-factor filters, loss frequency is immediately intuitive: "if most days this week were losses, trade less."

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN130 Negative Day Revenue Filter |
|--|--|--|
| Market stress signal | None | Rolling N-day loss frequency |
| Stress measurement | Drawdown (cumulative $) | Loss frequency (binary days) |
| Entry suppression | None | All entries suppressed if 5/5 negative days |
| Risk scaling | Fixed RISK | Scaled by loss frequency |
| Z-threshold | Fixed | Tightened during stress |
| Time horizon | Bar-level | Daily P&L level |
| Market memory | None | Rolling window of days |
| Stress interpretation | Drawdown amount | Negative day count |
