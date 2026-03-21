# RUN135 — Stress Accumulation Meter: Track Consecutive Directional Bars as Market Stress/Exhaustion Signal

## Hypothesis

**Named:** `stress_accum_meter`

**Mechanism:** COINCLAW uses z-score extremes and oscillators to identify entry points, but these don't measure *how long* price has been moving consistently in one direction. The Stress Accumulation Meter tracks how many consecutive bars have closed in the same direction (up = positive stress, down = negative stress). A high consecutive bar count means the market has been under sustained directional pressure — when this stress finally releases (the streak breaks), it often triggers a strong mean-reversion. The Stress Accumulation Meter uses streak length as both an entry filter (suppress entries when streak is too long, as reversal is imminent) and a risk scaler (reduce size when stress is building, increase when stress has been building for too long and reversal is near).

**Stress Accumulation Meter:**
- Track `bar_streak`: consecutive bars where close > prior close (positive) or close < prior close (negative), reset to 0 on flat
- Track `stress_meter = bar_streak / SAM_WINDOW` (normalized 0 to 1)
- For regime entries: when `stress_meter >= SAM_SUPPRESS_THRESH` (e.g., 0.80 = 8/10 consecutive same-direction bars), market is overstressed — entries may be suppressed as reversal is imminent
- For risk scaling: when stress is building (0.5 to 0.8), reduce RISK × SAM_STRESS_SCALE; when stress peaks and starts releasing, increase RISK
- This measures directional persistence as a proxy for market stress and impending reversal

**Why this is not a duplicate:**
- RUN120 (Mass Index) used High-Low range narrowing — SAM uses consecutive directional closes, completely different data
- RUN105 (z-persistence filter) required z-score to be extreme for N consecutive bars — SAM uses directional bar count, not z-score
- RUN121 (TD Sequential) used close > close 4 bars ago — SAM uses consecutive closes vs immediate prior close, simpler and different
- RUN85 (momentum pulse filter) used ROC magnitude — SAM uses bar count, not magnitude
- No prior RUN has used consecutive directional bar count as a market stress/reversal signal

**Mechanistic rationale:** Markets move in waves — sustained directional movement builds "stress" (positional fatigue among traders). When price has closed in the same direction for 8+ consecutive bars, the market is exhausted in that direction. Mean-reversion trades fired during high-stress periods have a higher probability of success because the directional momentum is depleted. The Stress Accumulation Meter quantifies this exhaustion in a simple, interpretable way: "the market has been going up for X bars straight — it's exhausted and likely to reverse."

---

## Proposed Config Changes

```rust
// RUN135: Stress Accumulation Meter
pub const SAM_ENABLE: bool = true;
pub const SAM_WINDOW: usize = 10;             // window for measuring streak
pub const SAM_SUPPRESS_THRESH: f64 = 0.80;   // suppress entries if stress >= this (reversal imminent)
pub const SAM_STRESS_SCALE: f64 = 0.60;      // multiply RISK by this when stress is building
pub const SAM_STRESS_RANGE_LOW: f64 = 0.50;  // stress range for scaling [0.5, 0.8)
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub bar_streak: i32,       // consecutive bars in same direction (+ = up, - = down, 0 = flat/reset)
    pub stress_meter: f64,     // normalized stress: abs(bar_streak) / SAM_WINDOW
}
```

**`strategies.rs` — add Stress Accumulation Meter helpers:**
```rust
/// Update bar streak each bar.
fn update_bar_streak(cs: &mut CoinState) {
    let candles = &cs.candles_15m;
    if candles.len() < 2 { return; }

    let current = candles[candles.len() - 1].c;
    let prior = candles[candles.len() - 2].c;

    if current > prior {
        if cs.bar_streak > 0 {
            cs.bar_streak += 1;
        } else {
            cs.bar_streak = 1;
        }
    } else if current < prior {
        if cs.bar_streak < 0 {
            cs.bar_streak -= 1;
        } else {
            cs.bar_streak = -1;
        }
    } else {
        cs.bar_streak = 0;
    }

    cs.stress_meter = (cs.bar_streak.abs() as f64) / config::SAM_WINDOW as f64;
}

/// Check if stress accumulation suppresses entries (reversal imminent).
fn stress_suppress(cs: &CoinState) -> bool {
    if !config::SAM_ENABLE { return false; }
    cs.stress_meter >= config::SAM_SUPPRESS_THRESH
}

/// Compute effective RISK based on stress level.
fn sam_effective_risk(cs: &CoinState) -> f64 {
    if !config::SAM_ENABLE { return config::RISK; }
    if stress_suppress(cs) { return 0.0; }  // suppress entries entirely
    if cs.stress_meter >= config::SAM_STRESS_RANGE_LOW && cs.stress_meter < config::SAM_SUPPRESS_THRESH {
        return config::RISK * config::SAM_STRESS_SCALE;
    }
    config::RISK
}

/// Check if stress confirms reversal is building (streak near peak, reversal likely).
fn stress_reversal_confirm(cs: &CoinState) -> bool {
    // If stress is near threshold, reversal is imminent — don't enter
    !stress_suppress(cs)
}
```

**`engine.rs` — modify check_entry to apply stress filter:**
```rust
// In check_entry, before opening position:
if config::SAM_ENABLE {
    if stress_suppress(&state.coins[ci]) {
        return;  // suppress entry when market is overstressed (reversal imminent)
    }
    let effective_risk = sam_effective_risk(&state.coins[ci]);
    // Use effective_risk for position sizing
}
```

---

## Validation Method

### RUN135.1 — Stress Accumulation Meter Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no stress accumulation filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `SAM_WINDOW` | [5, 10, 15] |
| `SAM_SUPPRESS_THRESH` | [0.70, 0.80, 0.90] |
| `SAM_STRESS_SCALE` | [0.50, 0.70] |

**Per coin:** 3 × 3 × 2 = 18 configs × 18 coins = 324 backtests

**Key metrics:**
- `sam_suppress_rate`: % of bars with entries suppressed due to high stress
- `sam_taper_rate`: % of bars with tapered risk
- `avg_streak_at_filtered`: average bar streak at suppressed entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN135.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best SAM_WINDOW × SUPPRESS_THRESH per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Suppressed entries have lower win rate than allowed entries
- Risk tapering reduces drawdown during high-stress periods

### RUN135.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no SAM) | Stress Accumulation Meter | Delta |
|--------|---------------------|------------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Suppress Events | 0% | X% | — |
| Taper Events | 0% | X% | — |
| Avg Streak at Suppressed | — | X | — |
| Avg Stress at Tapered | — | X | — |
| Avg Deployed Risk | 10% | X% | -Y% |

---

## Why This Could Fail

1. **Consecutive bar count is noisy:** In choppy markets, the streak resets frequently, making the meter flip between 0 and 1 constantly. The signal may be too noisy to be useful.
2. **Markets can trend for long periods:** Crypto markets (especially BTC) can have sustained directional trends of 10+ bars. Suppressing entries during these streaks means missing the beginning of major moves.
3. **Stress doesn't predict reversal point:** Knowing the market is "stressed" doesn't tell us when the reversal will happen. The reversal could come 1 bar later or 10 bars later.

---

## Why It Could Succeed

1. **Captures market exhaustion:** When price has closed in the same direction for 8+ bars straight, the market is exhibiting positional exhaustion. This is a genuine phenomenon — markets can't move in one direction indefinitely.
2. **Simple and interpretable:** The stress meter is immediately intuitive: "8 up closes in a row = overstressed = likely reversal." No complex calculations.
3. **Identifies mean-reversion entry timing:** The best mean-reversion entries often fire right after a sustained directional streak — the market is most exhausted at that point. SAM captures this timing.
4. **Different from TD Sequential:** TD Sequential uses close vs close 4 bars ago. SAM uses consecutive vs immediate prior close — a simpler, more direct measure of persistent directional pressure.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN135 Stress Accumulation Meter |
|--|--|--|
| Market stress signal | None | Consecutive directional bar count |
| Entry suppression | None | Suppress when stress >= 80% |
| Risk scaling | Fixed RISK | Scaled by stress level |
| Reversal timing | None | Streak length signals exhaustion |
| Stress measurement | None | bar_streak / SAM_WINDOW |
| Timeframe | All bars | Rolling window of consecutive |
| Market exhaustion | None | bar_streak count |
| Entry timing | Z-score extreme | Z-score + stress exhaustion |
