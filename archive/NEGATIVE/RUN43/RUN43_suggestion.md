# RUN43 — Breadth Momentum (Velocity) Filter: Anticipating Regime Transitions

## Hypothesis

**Named:** `breadth_momentum_filter`

**Mechanism:** COINCLAW's market regime system uses absolute breadth thresholds:
- LONG: breadth ≤ 20%
- ISO_SHORT: 20% < breadth < 50%
- SHORT: breadth ≥ 50%

This is a **level** sensor. It only triggers when breadth has already crossed the threshold. By the time the system switches from LONG to ISO_SHORT, the move is already underway — the best entry point has passed.

The hypothesis is that **breadth momentum** (the rate of change of breadth) contains predictive information:
- **Rising breadth → bearish pressure building** — ISO_SHORT and SHORT entries should be *prepared* early, before threshold is reached
- **Falling breadth → bullish pressure building** — LONG entries should be *prepared* early
- **Rapid breadth change** (d|Σ/dt|) signals a regime shift in progress — suppress entries until the shift stabilizes

**Concrete mechanism:**
- `breadth_velocity = breadth[i] − breadth[i-N]` (e.g., N=4 bars = 1 hour)
- `breadth_accel = breadth_velocity[i] − breadth_velocity[i-N]`
- When `breadth_velocity > THRESHOLD` AND `breadth > 15%` (early warning): suppress LONG entries, prepare SHORT
- When `breadth_velocity < −THRESHOLD` AND `breadth < 25%` (early warning): suppress SHORT entries, prepare LONG
- When `|breadth_accel|` is large: suppress all entries (regime in transition, high noise)

**Why this is not a duplicate:**
- No prior RUN measured breadth velocity or acceleration — only static breadth levels
- RUN12 (scalp market mode) used the 3-mode regime, not breadth momentum
- All prior regime threshold tests (RUN5, RUN6, RUN12) used crossing, not anticipation
- Breadth momentum is a derivative indicator, fundamentally different from level-based thresholds

---

## Proposed Config Changes

```rust
// RUN43: Breadth momentum parameters
pub const BREADTH_VELOCITY_WINDOW: u32 = 4;    // 4 bars (~1h) for velocity computation
pub const BREADTH_VEL_THRESHOLD: f64 = 0.08;   // breadth must be changing at ≥8%/hour to trigger
pub const BREADTH_ACCEL_THRESHOLD: f64 = 0.05;  // suppress entries when accel exceeds this
pub const BREADTH_LEAD_SUPPRESS: bool = true;  // suppress entries during momentum transition
```

**`coordinator.rs` change — `compute_breadth_and_context` extended:**
```rust
pub struct MarketCtx {
    // ... existing fields ...
    pub breadth_velocity: f64,   // NEW: breadth change over BREADTH_VELOCITY_WINDOW
    pub breadth_accel: f64,      // NEW: second derivative of breadth
    pub breadth_lead_suppress: bool,  // NEW: true when in rapid transition
}

pub fn compute_breadth_and_context(state: &SharedState) -> (f64, usize, usize, MarketMode, MarketCtx) {
    // ... existing breadth computation ...

    // Breadth momentum computation
    let breadth_history = &state.coins[ci].candles_15m;  // need rolling history
    // Simpler: track breadth[i] - breadth[i-N] using stored history
    // For backtest: maintain a rolling buffer of breadth values
    let breadth_velocity = /* breadth[i] - breadth[i - BREADTH_VELOCITY_WINDOW] */;
    let breadth_accel = /* breadth_velocity[i] - breadth_velocity[i - BREADTH_VELOCITY_WINDOW] */;
    let breadth_lead_suppress = breadth_accel.abs() > config::BREADTH_ACCEL_THRESHOLD;

    let ctx = MarketCtx {
        // ... existing fields ...
        breadth_velocity,
        breadth_accel,
        breadth_lead_suppress,
    };
}
```

**`strategies.rs` change — `long_entry` and `short_entry` use breadth momentum:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, ctx: &MarketCtx) -> bool {
    // Suppress LONG during rapid bearish breadth transition
    if config::BREADTH_LEAD_SUPPRESS && ctx.breadth_velocity > config::BREADTH_VEL_THRESHOLD {
        return false;
    }
    // ... rest of existing long_entry logic ...
}
```

---

## Validation Method

### RUN43.1 — Breadth Momentum Discovery (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Implementation note:** Breadth is computed from all 18 coins simultaneously. For breadth momentum, we need a rolling history of breadth values per bar. The backtester should maintain a rolling buffer of the last N breadth values.

**Step 1 — Profile breadth momentum at current regime transitions:**

For each bar where regime changes (LONG→ISO_SHORT or ISO_SHORT→LONG):
1. Record `breadth_velocity` and `breadth_accel` at T-1, T-2, T-3 bars before transition
2. Compute: what % of transitions are preceded by strong momentum signal 1-3 bars earlier?

**Step 2 — Grid search:**

| Parameter | Values |
|-----------|--------|
| `BREADTH_VELOCITY_WINDOW` | [2, 4, 6] bars |
| `BREADTH_VEL_THRESHOLD` | [0.05, 0.08, 0.12, 0.15] |
| `BREADTH_ACCEL_THRESHOLD` | [0.03, 0.05, 0.08] |
| `BREADTH_LEAD_SUPPRESS` | [true, false] |

**Per coin:** 3 × 4 × 3 × 2 = 72 configs × 18 coins = 1,296 backtests

**Also test:** Does the *opposite* of lead suppression work better? i.e., **early entry** — enter SHORT 1-2 bars before breadth crosses 50% when momentum is strongly positive?

**Key metric:** `avg_bars_before_regime =` on average, how many bars does the momentum signal fire before the regime transition? Earlier entry = more profit.

### RUN43.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best breadth momentum params per coin (or universal best)
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Average early-entry advantage ≥ 1 bar (momentum signal fires before level threshold)
- Portfolio OOS P&L ≥ baseline

### RUN43.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Breadth Momentum | Delta |
|--------|---------------|-----------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Avg Entry Timing | T=0 | T=−N | +N bars early |
| Lead Suppress Rate | 0% | N% | — |
| False Suppress Rate | 0% | N% | — |

---

## Why This Could Fail

1. **Breadth momentum is too noisy:** Breadth fluctuates bar-to-bar based on coin-level noise, not just genuine regime shifts. The velocity signal may fire on false transitions.
2. **Momentum confirms, doesn't predict:** Breadth velocity might be a *consequence* of regime change (coins start falling together, which creates the velocity), not a *cause*. In this case, velocity fires at the same time as the level, not before.
3. **Early entry catches the falling knife:** Entering SHORT before breadth crosses 50% means catching a market that is falling but hasn't committed to a full dump. If the drop reverses before reaching 50%, the early SHORT entry hits SL for a loss.

---

## Why It Could Succeed

1. **Anticipation vs reaction is the key edge:** The entire COINCLAW system reacts to regime after the threshold is crossed. Breadth momentum lets it anticipate — entering SHORT earlier means catching the beginning of the dump at better prices.
2. **Breadth is already computed:** No new data fetch needed. The `breadth` value exists; we just need to track its history.
3. **Lead suppress protects against whipsaws:** Suppressing entries during rapid breadth transitions prevents the worst whipsaws — when breadth oscillates around 20% or 50%.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN43 Breadth Momentum |
|--|--|--|
| Regime detection | Static level (≤20%, ≥50%) | Level + velocity + acceleration |
| Entry timing | At threshold crossing | 1-N bars before crossing |
| Short suppression | None during transition | During rapid transitions |
| Data required | Current breadth only | Rolling breadth history |
| Predictiveness | Reactive | Anticipatory |
| Expected early entry | 0 bars | 1–3 bars |
