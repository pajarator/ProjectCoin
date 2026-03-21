# RUN81 — Equity Curve Circuit Breaker: Halt New Entries During Sustained Drawdown Periods

## Hypothesis

**Named:** `equity_circuit_breaker`

**Mechanism:** COINCLAW currently trades continuously regardless of portfolio performance. During extended drawdown periods (e.g., 3+ consecutive losing days), the market regime may have shifted — regime strategies that work in one market condition may fail in another. Continuing to trade during a drawdown streak not only loses money but also degrades capital that could be deployed when conditions improve.

**Equity Curve Circuit Breaker:**
- Track portfolio `peak_equity` (highest total balance ever achieved)
- Compute current `drawdown_pct = (peak_equity - current_equity) / peak_equity`
- When `drawdown_pct >= CIRCUIT_BREAKER_THRESHOLD` (e.g., 15%) for `CIRCUIT_BREAKER_BARS` consecutive bars → activate circuit breaker mode
- During circuit breaker mode: suppress all new regime entries (scalp and momentum remain active — they operate on different timescales)
- Exit circuit breaker mode when equity recovers to within `CIRCUIT_BREAKER_RECOVERY` of peak (e.g., 10% of peak — 5% drawdown)
- ISO shorts remain allowed during circuit breaker (shorting in a drawdown market can be high-conviction)

**Why this is not a duplicate:**
- RUN51 (DD-contingent SL widening) sizes individual positions based on per-coin drawdown — this halts new entries based on portfolio-wide drawdown
- RUN34 (ISO cooldown escalation) adds cooldown after consecutive SLs — this is per-coin, not portfolio-wide
- RUN74 (daily compound reset) resets equity periodically — this is a circuit breaker, not a reset; it preserves capital by not trading bad regimes
- No prior RUN has implemented a portfolio-level equity circuit breaker that halts regime entries during poor market conditions

**Mechanistic rationale:** COINCLAW regime trades are mean-reversion strategies. During severe, sustained drawdowns, the market may have entered a regime where mean reversion fails systematically (e.g., prolonged trending market). Halting regime entries during these periods avoids feeding a losing streak while preserving capital. Scalp and momentum continue — scalp operates on 1m timeframes and is regime-agnostic, momentum has its own ATR-based risk management.

---

## Proposed Config Changes

```rust
// RUN81: Equity Curve Circuit Breaker
pub const CIRCUIT_BREAKER_ENABLE: bool = true;
pub const CIRCUIT_BREAKER_THRESHOLD: f64 = 0.15;   // 15% drawdown from peak activates breaker
pub const CIRCUIT_BREAKER_BARS: u32 = 10;           // must be below threshold for 10 consecutive bars (~2.5h)
pub const CIRCUIT_BREAKER_RECOVERY: f64 = 0.10;    // recover to within 10% of peak to deactivate
pub const CIRCUIT_BREAKER_SCALP_EXEMPT: bool = true; // scalp trades exempt (different timeframe)
pub const CIRCUIT_BREAKER_MOMENTUM_EXEMPT: bool = true; // momentum trades exempt (own risk mgmt)
pub const CIRCUIT_BREAKER_ISO_ALLOWED: bool = true; // ISO shorts allowed during breaker
```

**`state.rs` — SharedState additions:**
```rust
pub struct SharedState {
    // ... existing fields ...
    pub peak_equity: f64,          // highest total equity ever
    pub equity_breakers_bars: u32, // consecutive bars below threshold
    pub circuit_breaker_active: bool,
}
```

**`engine.rs` — circuit_breaker_check in coordinator tick:**
```rust
/// Update portfolio peak equity and check circuit breaker status.
pub fn update_circuit_breaker(state: &mut SharedState) {
    if !config::CIRCUIT_BREAKER_ENABLE { return; }

    let current_equity = state.total_balance();

    // Update peak
    if current_equity > state.peak_equity {
        state.peak_equity = current_equity;
    }

    let drawdown = (state.peak_equity - current_equity) / state.peak_equity;

    if drawdown >= config::CIRCUIT_BREAKER_THRESHOLD {
        state.equity_breakers_bars += 1;
        if state.equity_breakers_bars >= config::CIRCUIT_BREAKER_BARS {
            state.circuit_breaker_active = true;
        }
    } else {
        state.equity_breakers_bars = 0;
        // Check recovery: if equity is within recovery threshold of peak, deactivate
        if drawdown <= config::CIRCUIT_BREAKER_RECOVERY {
            state.circuit_breaker_active = false;
        }
    }
}

/// Check if a regime entry is allowed under circuit breaker rules.
pub fn is_entry_allowed(state: &SharedState, trade_type: TradeType) -> bool {
    if !config::CIRCUIT_BREAKER_ENABLE { return true; }
    if !state.circuit_breaker_active { return true; }

    // Scalp and momentum exempt
    if config::CIRCUIT_BREAKER_SCALP_EXEMPT && trade_type == TradeType::Scalp { return true; }
    if config::CIRCUIT_BREAKER_MOMENTUM_EXEMPT && trade_type == TradeType::Momentum { return true; }

    // ISO shorts allowed
    if config::CIRCUIT_BREAKER_ISO_ALLOWED { return true; }

    false  // regime entries blocked during circuit breaker
}

// In check_entry — before opening position:
if !is_entry_allowed(state, TradeType::Regime) {
    return;  // suppress regime entry during circuit breaker
}
```

---

## Validation Method

### RUN81.1 — Circuit Breaker Grid Search (Rust, 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no circuit breaker

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CIRCUIT_BREAKER_THRESHOLD` | [0.10, 0.15, 0.20] |
| `CIRCUIT_BREAKER_BARS` | [5, 10, 20] |
| `CIRCUIT_BREAKER_RECOVERY` | [0.05, 0.10, 0.15] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests (note: circuit breaker is portfolio-level, so grid is per full portfolio run, not per-coin; use same grid for portfolio-level evaluation)

**Key metrics:**
- `breaker_activation_rate`: % of bars where circuit breaker is active
- `regime_entries_blocked`: number of regime entries suppressed during breaker
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN81.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best threshold/bars/recovery per window
2. Test: evaluate on held-out month

**Pass criteria:**
- Portfolio P&L delta ≥ 0 vs baseline (breaker should not reduce profits)
- Circuit breaker activates at least once per quarter (meaningful use)
- Max drawdown reduced vs baseline

### RUN81.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no breaker) | Equity Circuit Breaker | Delta |
|--------|--------------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Breaker Activations | 0 | X | — |
| Bars in Breaker | 0 | X% | — |
| Regime Entries Blocked | 0 | X | — |
| Equity Recovery Time | — | X bars | — |

---

## Why This Could Fail

1. **Drawdown is backward-looking:** By the time the circuit breaker activates, the worst of the drawdown may be over. The market may have already mean-reverted, and the breaker is blocking entries at the bottom.
2. **Harming the winning periods:** If the threshold is too tight, the breaker blocks entries during normal volatility drawdowns — these are not the catastrophic regime changes it's designed to avoid. This could reduce overall profits.
3. **ISO shorts may not save capital:** If ISO shorts are allowed during the breaker, the portfolio may still accumulate losses from short positions if the drawdown was from a broad market rally rather than a selloff.

---

## Why It Could Succeed

1. **Prevents trading in shifted regimes:** The best evidence that a regime has changed is a sustained portfolio drawdown. A circuit breaker that detects this and halts regime entries preserves capital for the next regime.
2. **Forces discipline during losing periods:** Traders often overtrade during drawdown periods, trying to "make back" losses. The circuit breaker provides a mechanical rule that overrides this impulse.
3. **Complementary to per-coin risk controls:** While RUN51 widens SLs and RUN39 adds cooldowns, neither halts new entries entirely. The circuit breaker is a portfolio-level circuit breaker that no other RUN has tested.
4. **Institutional practice:** All systematic funds use portfolio-level drawdown limits that halt new trading. This is standard risk management.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN81 Equity Circuit Breaker |
|--|--|--|
| Entries during drawdown | Always allowed | Regime entries blocked when dd ≥ 15% for 10 bars |
| Scalp during drawdown | Always allowed | Always allowed (timeframe different) |
| Momentum during drawdown | Always allowed | Always allowed (own risk mgmt) |
| ISO shorts during drawdown | Always allowed | Always allowed (high-conviction) |
| Circuit breaker | None | Activates at 15% dd, deactivates at 5% dd |
| Drawdown protection | Per-coin (RUN51, RUN39) | Portfolio-wide |
| Capital preservation | None | Explicit halt rule |
