# RUN87 — Drawdown Recovery Mode: Shift Market Mode Bias During Portfolio Drawdown

## Hypothesis

**Named:** `drawdown_recovery_mode`

**Mechanism:** COINCLAW's market mode detection (LONG / ISO_SHORT / SHORT) is based solely on breadth. But when the portfolio is in a drawdown from peak equity, the market may have entered a different regime than breadth alone captures. Specifically: during portfolio drawdown, the strategy should bias toward ISO_SHORT trades (high-breadth opportunities) and away from LONG trades (which may be fighting a bearish market).

**Drawdown Recovery Mode:**
- Track portfolio `peak_equity` and `drawdown_pct` continuously (same as RUN81)
- When `drawdown_pct >= DD_RECOVERY_THRESHOLD` (e.g., 10%) → activate DD Recovery Mode
- In DD Recovery Mode:
  - Tighten `ISO_SHORT_BREADTH_MAX` (e.g., from 0.20 → 0.15) so ISO shorts fire more easily
  - Widen `BREADTH_MAX` for LONG (e.g., from 0.20 → 0.25) so fewer LONG entries
  - Suppress `ComplementStrat` entries (they are lower-conviction and may add losses in drawdown)
- Exit DD Recovery Mode when `drawdown_pct <= DD_RECOVERY_EXIT` (e.g., 5%)
- This complements RUN81 (circuit breaker) — RUN81 halts entries during drawdown, RUN87 shifts entry TYPE during drawdown

**Why this is not a duplicate:**
- RUN51 (DD-contingent SL widening) sizes individual positions by per-coin drawdown — this shifts portfolio-wide market mode bias
- RUN81 (equity circuit breaker) halts entries entirely during drawdown — this shifts WHICH entries are allowed, not whether entries are allowed
- No prior RUN has made the market mode thresholds themselves conditional on portfolio equity state

**Mechanistic rationale:** A 10% portfolio drawdown signals that the market environment has shifted — LONG trades from the current regime may be lower probability than usual. Biasing toward ISO shorts (which perform well in oversold markets, which often accompany drawdowns) and away from LONG trades preserves capital during a difficult period without halting all trading.

---

## Proposed Config Changes

```rust
// RUN87: Drawdown Recovery Mode
pub const DD_RECOVERY_ENABLE: bool = true;
pub const DD_RECOVERY_THRESHOLD: f64 = 0.10;    // activate at 10% portfolio drawdown
pub const DD_RECOVERY_EXIT: f64 = 0.05;         // exit recovery mode at 5% drawdown
pub const DD_ISO_BREADTH_TIGHTEN: f64 = 0.05;  // tighten ISO_SHORT_BREADTH_MAX by this amount (e.g., 0.20 → 0.15)
pub const DD_LONG_BREADTH_WIDEN: f64 = 0.05;    // widen BREADTH_MAX for LONG by this amount (e.g., 0.20 → 0.25)
pub const DD_SUPPRESS_COMPLEMENT: bool = true;  // suppress complement entries in recovery mode
```

**`engine.rs` — DD recovery mode helpers:**
```rust
/// Returns true if portfolio is currently in DD Recovery Mode.
pub fn in_dd_recovery_mode(state: &SharedState) -> bool {
    if !config::DD_RECOVERY_ENABLE { return false; }
    let current_equity = state.total_balance();
    let peak = state.peak_equity;
    if peak == 0.0 { return false; }
    let drawdown = (peak - current_equity) / peak;
    drawdown >= config::DD_RECOVERY_THRESHOLD
}

/// Get effective ISO_SHORT_BREADTH_MAX (may be tightened in DD Recovery Mode).
pub fn effective_iso_breadth_max() -> f64 {
    if in_dd_recovery_mode(state) {
        return config::ISO_SHORT_BREADTH_MAX - config::DD_ISO_BREADTH_TIGHTEN;
    }
    config::ISO_SHORT_BREADTH_MAX
}

/// Get effective BREADTH_MAX for LONG mode (may be widened in DD Recovery Mode).
pub fn effective_breadth_max() -> f64 {
    if in_dd_recovery_mode(state) {
        return config::BREADTH_MAX + config::DD_LONG_BREADTH_WIDEN;
    }
    config::BREADTH_MAX
}

// In check_entry — use effective thresholds:
let iso_breadth_max = effective_iso_breadth_max();
let breadth_max_long = effective_breadth_max();

// In coordinator — detect_market_mode uses effective thresholds:
let mode = if breadth <= breadth_max_long {  // ← use effective breadth_max_long
    MarketMode::Long
} else if breadth >= config::SHORT_BREADTH_MIN {
    MarketMode::Short
} else {
    MarketMode::IsoShort
};

// In check_entry, complement suppression in recovery mode:
if config::DD_SUPPRESS_COMPLEMENT && in_dd_recovery_mode(state) {
    // Skip complement_entry during recovery mode
} else if strategies::complement_entry(...) {
    // ... existing complement logic
}
```

**`state.rs` — SharedState additions (reuse from RUN81 if both are enabled):**
```rust
pub struct SharedState {
    // ... existing fields ...
    pub peak_equity: f64,
}
```

---

## Validation Method

### RUN87.1 — Drawdown Recovery Grid Search (Rust, 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed breadth thresholds regardless of portfolio state

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `DD_RECOVERY_THRESHOLD` | [0.08, 0.10, 0.15] |
| `DD_RECOVERY_EXIT` | [0.04, 0.05, 0.08] |
| `DD_ISO_BREADTH_TIGHTEN` | [0.03, 0.05, 0.08] |
| `DD_LONG_BREADTH_WIDEN` | [0.03, 0.05, 0.08] |

**Note:** This is a portfolio-level optimization. Total configs: 3 × 3 × 3 × 3 = 81.

**Key metrics:**
- `recovery_activation_rate`: % of bars in DD Recovery Mode
- `iso_breadth_delta`: change in effective ISO_SHORT threshold during recovery
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN87.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best threshold/exit/widen combinations per window
2. Test: evaluate on held-out month

**Pass criteria:**
- Portfolio P&L delta ≥ 0 vs baseline
- Max drawdown reduced vs baseline
- Recovery mode activates at least once per quarter

### RUN87.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed thresholds) | Drawdown Recovery Mode | Delta |
|--------|--------------------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Recovery Mode Activations | 0 | X | — |
| Bars in Recovery Mode | 0% | X% | — |
| ISO Short Entry Rate | X | X | +Y% |
| LONG Entry Rate | X | X | -Y% |
| Complement Entry Rate | X | X | -Y% |

---

## Why This Could Fail

1. **Portfolio drawdown is not a market forecast:** A drawdown from peak equity is backward-looking — it tells us what happened, not what will happen. The market may recover immediately, and our biased entry selection would have been wrong.
2. **ISO shorts may already be dominating:** If the portfolio is in drawdown, ISO shorts may already be firing frequently. Tightening the threshold further may not help if ISO shorts are already the dominant mode.
3. **Compounds RUN81 behavior:** If both RUN81 and RUN87 are active, RUN81 halts regime entries during drawdown while RUN87 tries to shift entry type. These may conflict or be redundant.

---

## Why It Could Succeed

1. **Drawdown often follows regime change:** A portfolio drawdown of 10%+ often coincides with a shift from a mean-reversion-friendly market to a more directional one. The breadth thresholds that worked before the drawdown may not be appropriate after it.
2. **ISO shorts are high-conviction during oversold:** The market conditions that cause portfolio drawdowns (broad selloffs) are exactly the conditions where ISO shorts perform best. Biasing toward ISO shorts during drawdown aligns with market opportunity.
3. **Simple threshold adjustment:** No new indicators — just making the existing breadth thresholds conditional on portfolio equity state. This is a minimal, low-risk change.
4. **Additive with RUN81:** RUN81 halts entries during severe drawdown. RUN87 is more surgical — it keeps trading but shifts toward higher-conviction setups. Together they form a layered drawdown protection system.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN87 Drawdown Recovery Mode |
|--|--|--|
| ISO_SHORT_BREADTH_MAX | Fixed 0.20 | Tighter (0.15) during drawdown |
| BREADTH_MAX for LONG | Fixed 0.20 | Wider (0.25) during drawdown |
| Complement entries | Always allowed | Suppressed during drawdown |
| Market mode detection | Breadth only | Breadth + portfolio equity state |
| Drawdown awareness | None | Active at 10%+ dd |
| Recovery exit | N/A | Deactivates at 5% dd |
