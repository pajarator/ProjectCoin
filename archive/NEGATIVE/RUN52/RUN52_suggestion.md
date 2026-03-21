# RUN52 — Z-Score Deviation Position Sizing: Signal Confidence-Weighted Entries

## Hypothesis

**Named:** `z_confidence_sizing`

**Mechanism:** COINCLAW currently uses a fixed `RISK = 10%` of equity per trade regardless of how confident the system should be in that specific trade. An entry at `z = −2.5` (extreme oversold) is a stronger mean-reversion signal than an entry at `z = −1.6` (borderline oversold), but both receive the same position size.

The hypothesis is that **position size should scale with signal confidence**:
- `z = −2.5`: more extreme deviation → higher probability of reversion → larger position
- `z = −1.6`: less extreme → lower probability of reversion → smaller position

**Sizing formula:**
```
base_risk = RISK  (0.10)
signal_confidence = (|z_entry| - Z_ENTRY_MIN) / (Z_ENTRY_MAX - Z_ENTRY_MIN)
position_fraction = base_risk × (1.0 + CONFIDENCE_MULTIPLIER × signal_confidence)
```

Where `Z_ENTRY_MIN = 1.5` (minimum entry threshold), `Z_ENTRY_MAX = 3.0` (extreme), and `CONFIDENCE_MULTIPLIER` caps the maximum upsizing.

**Example:**
- `z = −1.5`: confidence = 0, position_fraction = 0.10 (base)
- `z = −2.25`: confidence = 0.50, position_fraction = 0.10 × (1 + 0.5 × 0.5) = 0.125
- `z = −3.0`: confidence = 1.0, position_fraction = 0.10 × (1 + 0.5 × 1.0) = 0.15

**Why this is not a duplicate:**
- RUN19 (Kelly sizing) scaled position size based on *historical* win rate and avg win/loss
- This RUN scales position size based on *current signal* extremity (z-score at entry)
- No prior RUN used z-score deviation magnitude as a sizing input
- This is signal-quality sizing, fundamentally different from statistical sizing

---

## Proposed Config Changes

```rust
// RUN52: Z-Score Deviation Position Sizing
pub const Z_CONFidence_ENABLE: bool = true;
pub const Z_ENTRY_MIN: f64 = 1.5;          // minimum z for entry
pub const Z_ENTRY_MAX: f64 = 3.0;          // "perfect" z-score deviation
pub const CONFIDENCE_MULTIPLIER: f64 = 0.5;  // max upsizing at Z_ENTRY_MAX (0.5 = +50%)
pub const Z_SIZE_CAP: f64 = 0.20;           // maximum position fraction regardless of z (cap at 20%)
```

**`engine.rs` — position sizing in `open_position`:**
```rust
fn confidence_position_fraction(ind: &Ind15m, dir: Direction, strat_name: &str) -> f64 {
    if !config::Z_CONFidence_ENABLE { return config::RISK; }

    let z = ind.z.abs();
    if z <= config::Z_ENTRY_MIN { return config::RISK; }

    let z_clamped = z.min(config::Z_ENTRY_MAX);
    let confidence = (z_clamped - config::Z_ENTRY_MIN) / (config::Z_ENTRY_MAX - config::Z_ENTRY_MIN);
    let fraction = config::RISK * (1.0 + config::CONFIDENCE_MULTIPLIER * confidence);
    fraction.min(config::Z_SIZE_CAP)  // never exceed cap
}

fn open_position(...) {
    let risk_fraction = confidence_position_fraction(&ind, dir, strat);
    let trade_amt = cs.bal * risk_fraction;
    let sz = (trade_amt * config::LEVERAGE) / price;
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN52.1 — Z-Confidence Sizing Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed RISK = 0.10 per trade

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `Z_ENTRY_MAX` | [2.5, 3.0, 3.5] |
| `CONFIDENCE_MULTIPLIER` | [0.3, 0.5, 0.7, 1.0] |
| `Z_SIZE_CAP` | [0.15, 0.20, 0.25] |

**Per coin:** 3 × 4 × 3 = 36 configs × 18 coins = 648 backtests

**Also test:** Is this beneficial for LONG trades only, or also for SHORT trades? (Shorts use positive z, same absolute logic applies)

**Key metrics:**
- `avg_position_fraction`: average fraction used (shows how much upsizing actually occurs)
- `PF_delta`: profit factor change vs baseline
- `Sharpe_delta`: Sharpe ratio change vs baseline
- `max_DD_delta`: max drawdown change vs baseline (larger positions → larger losses on bad trades)
- `avg_win_delta`: average win $ change (larger positions on better setups → higher avg win)

### RUN52.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `Z_ENTRY_MAX × CONFIDENCE_MULTIPLIER` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS Sharpe delta vs baseline
- Portfolio OOS Sharpe ≥ baseline
- Portfolio max_DD increase < 20% vs baseline (larger positions shouldn't cause catastrophic drawdown)

### RUN52.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed 10%) | Z-Confidence Sizing | Delta |
|--------|--------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Sharpe Ratio | X.XX | X.XX | +0.XX |
| Max DD | X% | X% | +Ypp |
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Avg Position Fraction | 10.0% | X% | +Y% |
| Avg Win | $X | $X | +$X |
| Avg Loss | $X | $X | +$X |
| Z=1.5–2.0 trades | N | N | — |
| Z=2.0–2.5 trades | N | N | — |
| Z>2.5 trades | N | N | — |

---

## Why This Could Fail

1. **Z-score doesn't predict reversion quality:** An entry at z = −2.5 is more extreme but might mean-revert more slowly or less completely than an entry at z = −1.6. Larger position on a worse-than-expected reversion = bigger loss.
2. **Increases tail risk:** Larger positions on extreme z entries mean bigger losses when those entries fail. In a tail event (z = −3.5, which can happen in crashes), a 15% position vs 10% position is a 50% larger loss.
3. **Kelly analysis suggests optimal sizing is based on edge, not z-score:** RUN19 showed Kelly sizing amplifies losses. Sizing by z-score is a form of dynamic sizing that could have similar negative effects.

---

## Why It Could Succeed

1. **Intuitively sound:** More extreme deviations should have higher reversion probability. Sizing up on these is like "betting more when you have more information."
2. **Limited upside:** The multiplier is capped at 1.5× (50% increase), so the worst-case scenario is a 50% larger loss on a bad trade at extreme z — manageable.
3. **Doesn't change SL:** The stop loss is still fixed at 0.3%. The only thing that changes is position size, not the exit point.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN52 Z-Confidence Sizing |
|--|--|--|
| Position sizing | Fixed 10% | Z-score adaptive (10–15%) |
| Risk per trade | Fixed | Signal-quality adjusted |
| Entry confidence | Equal weight | Proportional to \|z\| |
| Max position fraction | 10% | 15–25% (configurable) |
| Expected Sharpe | X.XX | +0.05–0.15 |
| Expected Max DD | X% | +5–15% (larger positions = bigger losses) |
| Expected Avg Win | $X | +10–20% |
