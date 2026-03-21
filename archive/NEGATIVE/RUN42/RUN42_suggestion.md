# RUN42 — Dynamic Leverage by Volatility Regime: Risk-Adjusted Position Sizing

## Hypothesis

**Named:** `dynamic_leverage_regime`

**Mechanism:** COINCLAW currently uses a fixed `LEVERAGE = 5.0` for all trades regardless of market conditions. The `RISK = 10%` of equity is used to compute position size (`trade_amt = bal × RISK`, then `size = trade_amt × leverage / price`). This means position size scales with leverage, not just with risk.

**Problem:** During HighVol and Squeeze regimes, price moves are larger and more unpredictable. A 0.3% SL (fixed) represents a larger absolute price move in high-vol environments, but the position size is the same as in a calm Ranging regime. This asymmetry means high-vol periods generate larger losses per trade, which compounds disproportionately.

**Fix:** Adjust leverage downward during volatile regimes and/or upward during calm regimes. This changes the position size without changing the SL%:
- `LEVERAGE_RANGING = 7.0` (calm = more confident, larger position)
- `LEVERAGE_WEAKTREND = 5.0` (current default)
- `LEVERAGE_STRONGTREND = 3.0` (momentum running, reduce exposure)
- `LEVERAGE_HIGHVOL = 2.0` (high vol = reduce before event)
- `LEVERAGE_SQUEEZE = 3.0` (compression = likely breakout, moderate exposure)

**Why this is not a duplicate:**
- RUN19 tested Kelly criterion (position fraction, not leverage)
- RUN26 tested ATR-based dynamic stops (not position size)
- No prior RUN changed leverage based on detected regime
- The `detect_regime` function already classifies regimes — this RUN uses it for position sizing

---

## Proposed Config Changes

```rust
// RUN42: Dynamic leverage by regime
pub const LEVERAGE_RANGING: f64 = 7.0;
pub const LEVERAGE_WEAKTREND: f64 = 5.0;    // current default
pub const LEVERAGE_STRONGTREND: f64 = 3.0;
pub const LEVERAGE_HIGHVOL: f64 = 2.0;
pub const LEVERAGE_SQUEEZE: f64 = 3.0;

// Scalp layer: separate leverage table (scalp already uses SCALP_RISK = 5%)
pub const SCALP_LEVERAGE_RANGING: f64 = 5.0;
pub const SCALP_LEVERAGE_HIGHVOL: f64 = 2.0;
```

**`engine.rs` change — `open_position` uses regime-based leverage:**
```rust
fn regime_leverage(regime: Regime) -> f64 {
    match regime {
        Regime::Ranging => config::LEVERAGE_RANGING,
        Regime::WeakTrend => config::LEVERAGE_WEAKTREND,
        Regime::StrongTrend => config::LEVERAGE_STRONGTREND,
        Regime::HighVol => config::LEVERAGE_HIGHVOL,
        Regime::Squeeze => config::LEVERAGE_SQUEEZE,
    }
}

fn scalp_leverage(regime: Regime) -> f64 {
    match regime {
        Regime::HighVol => config::SCALP_LEVERAGE_HIGHVOL,
        _ => config::SCALP_LEVERAGE_RANGING,
    }
}
```

In `open_position`:
```rust
let lev = match trade_type {
    TradeType::Regime | TradeType::Momentum => regime_leverage(regime),
    TradeType::Scalp => scalp_leverage(regime),
};
let sz = (trade_amt * lev) / price;
```

---

## Validation Method

### RUN42.1 — Grid Search: Dynamic Leverage Parameters (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** `LEVERAGE = 5.0` fixed, `RISK = 0.10`

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `LEVERAGE_RANGING` | [5.0, 6.0, 7.0, 8.0] |
| `LEVERAGE_HIGHVOL` | [1.5, 2.0, 2.5, 3.0] |
| `LEVERAGE_SQUEEZE` | [2.0, 3.0, 4.0] |
| `LEVERAGE_STRONGTREND` | [2.0, 3.0, 4.0] |

`LEVERAGE_WEAKTREND` stays at 5.0 (baseline anchor).

**Per coin:** 4 × 4 × 3 × 3 = 144 configs × 18 coins = 2,592 backtests

**Note on RISK interaction:** Since `trade_amt = bal × RISK` is independent of leverage, and `sz = trade_amt × lev / price`, the actual position risk (in $) is `sz × price × SL% = trade_amt × lev × SL`. So doubling leverage doubles the $ risk. The grid search tests whether the regime-specific leverage changes improve risk-adjusted returns, not raw P&L.

**Key metric:** `Sharpe_ratio = mean_return / std_return` (annualized)
Also track: `max_dd`, `PF`, `total_PnL`, `avg_loss`

**Also test:** Does removing leverage entirely (lev=1.0) for HighVol produce better risk-adjusted returns?

### RUN42.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best leverage config per coin
2. Test: evaluate on held-out month with those params

**Pass criteria:**
- ≥ 10/18 coins show improved Sharpe ratio OOS vs baseline
- Portfolio OOS Sharpe ≥ baseline
- Portfolio max DD reduction > 20% vs baseline (the primary goal)

### RUN42.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, 5×) | Dynamic Leverage | Delta |
|--------|-------------------|-----------------|-------|
| Sharpe Ratio | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Avg Loss | $X | $X | -$X |
| Ranging Lev | 5.0 | X | — |
| HighVol Lev | 5.0 | X | — |
| Squeeze Lev | 5.0 | X | — |

---

## Why This Could Fail

1. **Leverage is already factored into RISK:** Since `trade_amt = bal × RISK` and RISK=10%, the actual position risk is determined by RISK, not leverage. Changing leverage only changes position size in dollar terms, which is already controlled by RISK. The leverage constant might be redundant.
2. **Volatility regime detection is noisy:** `detect_regime` classifies each bar independently — the regime can flicker between HighVol and Ranging on consecutive bars, causing leverage to oscillate rapidly.
3. **Reducing leverage in HighVol reduces both losses AND wins:** If the best trades happen during HighVol (when moves are big in both directions), lowering leverage cuts winners as much as losers. Net effect may be neutral or negative.

---

## Why It Could Succeed

1. **Max DD is the primary weakness of COINCLAW:** RUN17 Monte Carlo showed typical max DD of 4.0% — a 2× leverage reduction in HighVol could cut this significantly without proportionally reducing gains.
2. **Simple, no new data:** Leverages are constants; regime is already detected. Zero implementation overhead for the discovery phase.
3. **Asymmetric effect:** SL% is fixed at 0.3% — in high vol environments, this SL is reached more frequently (more noise). Reducing leverage during these periods reduces exposure to noise while preserving signal when conditions are calm.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN42 Dynamic Leverage |
|--|--|--|
| Leverage | Fixed 5.0 | Regime-conditional (2.0–8.0) |
| Position sizing | `bal × RISK × 5 / price` | `bal × RISK × lev(regime) / price` |
| Risk control | Fixed SL% only | SL% + dynamic leverage |
| Expected Max DD | X% | −20–40% |
| Expected Sharpe | X.XX | +0.1–0.3 |
| Regime Ranging leverage | 5.0 | 6–8 |
| Regime HighVol leverage | 5.0 | 1.5–3 |
