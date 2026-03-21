# RUN47 — Per-Strategy Optimal MIN_HOLD: Strategy-Specific Hold Times

## Hypothesis

**Named:** `per_strategy_min_hold`

**Mechanism:** COINCLAW currently uses a single global `MIN_HOLD_CANDLES = 2` for all strategy types and all coins. The MIN_HOLD prevents the SMA exit from firing immediately after entry (before the trade has had time to develop). However, different strategies mean-revert at different speeds:

- **VwapReversion** — price crossing below VWAP is a fast, tight reversion. MIN_HOLD=2 (30 min) may be too long; price often reverts within 1-2 bars.
- **AdrReversal** — ADR-based reversals require the price to reach the lower ADR band, which is a larger move and takes longer. MIN_HOLD=2 may be too short — the trade hasn't had time to reach the ADR target.
- **BbBounce** — Bollinger Band bounces can be fast (1-2 bars) or slow (漂洗). Harder to generalize.
- **MeanReversion / DualRsi** — z-score based entries are already capturing "how far from mean" — the z-score reversion typically takes 3-6 bars.

The hypothesis is that **per-strategy optimal MIN_HOLD values** will:
1. Increase win rate by giving fast strategies (VwapRev) less time to get stopped out
2. Increase win rate for slow strategies (AdrRev) by giving them more time to reach their target
3. Improve overall PF by reducing premature exits and insufficient holds

**Why this is not a duplicate:**
- RUN4 tested per-coin MIN_HOLD but on a much older system (v4) — no ISO shorts, no complement signals, no scalp overlay
- No prior RUN has tested per-strategy-type MIN_HOLD optimization
- The strategy-specific speed profile is a structural property that hasn't been explored
- This is the first optimization of `MIN_HOLD` since the system grew to 3 layers (regime + complement + scalp)

---

## Proposed Config Changes

```rust
// RUN47: Per-strategy MIN_HOLD parameters
pub const MIN_HOLD_VWAP_REV: u32 = 1;      // fast reversion — shorter hold
pub const MIN_HOLD_BB_BOUNCE: u32 = 2;     // default
pub const MIN_HOLD_ADR_REV: u32 = 4;       // slower ADR target — longer hold
pub const MIN_HOLD_MEAN_REV: u32 = 3;      // z-score based
pub const MIN_HOLD_DUAL_RSI: u32 = 3;      // dual RSI
pub const MIN_HOLD_OU_MEAN_REV: u32 = 4;  // OU mean reversion (DASH only)
```

**`engine.rs` change — `check_exit` uses strategy-specific MIN_HOLD:**
```rust
fn strategy_min_hold(strat_name: &str) -> u32 {
    if strat_name.starts_with("vwap_rev") {
        config::MIN_HOLD_VWAP_REV
    } else if strat_name.starts_with("adr_rev") {
        config::MIN_HOLD_ADR_REV
    } else if strat_name.starts_with("bb_bounce") {
        config::MIN_HOLD_BB_BOUNCE
    } else if strat_name.starts_with("mean_rev") {
        config::MIN_HOLD_MEAN_REV
    } else if strat_name.starts_with("dual_rsi") {
        config::MIN_HOLD_DUAL_RSI
    } else if strat_name.starts_with("ou_mean_rev") {
        config::MIN_HOLD_OU_MEAN_REV
    } else {
        config::MIN_HOLD_CANDLES  // fallback to global default
    }
}

fn effective_min_hold(cs: &CoinState) -> u32 {
    if let Some(ref strat_name) = cs.active_strat {
        strategy_min_hold(strat_name)
    } else {
        config::MIN_HOLD_CANDLES
    }
}
```

In `check_exit`, replace `held >= config::MIN_HOLD_CANDLES` with `held >= effective_min_hold(cs)`.

---

## Validation Method

### RUN47.1 — Per-Strategy MIN_HOLD Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** `MIN_HOLD_CANDLES = 2` globally (current COINCLAW v16)

**Grid search per strategy type:**

| Strategy Type | MIN_HOLD Values |
|--------------|----------------|
| VwapReversion | [1, 2, 3, 4] |
| AdrReversal | [2, 3, 4, 5, 6] |
| BbBounce | [1, 2, 3, 4] |
| MeanReversion | [2, 3, 4, 5] |
| DualRsi | [2, 3, 4, 5] |
| OuMeanRev | [2, 3, 4, 5, 6] |

**Per coin:** The grid is per-strategy-type, so effectively:
- VwapRev coins (LINK, ETH, DOT, XRP, SOL, BNB, UNI, NEAR, ADA, LTC, SHIB): 4 values
- AdrReversal coins (AVAX, ALGO): 5 values
- BbBounce coins (DOGE, BTC): 4 values
- MeanReversion: (none currently in COIN_STRATEGIES)
- DualRsi (XLM): 5 values
- OuMeanRev (DASH): 5 values

Total combos per strategy type × per coin = ~4-5 × 18 coins = ~72-90 combos × 18 coins ≈ 1,440 backtests

**Also test:** Does applying `MIN_HOLD` to complement strategies (Laguerre RSI, Kalman, KST) change outcomes? Complement signals may need their own MIN_HOLD since they're different signal types.

**Key metrics:**
- `PF_delta = new_PF − baseline_PF` per strategy type
- `WR_delta = new_WR% − baseline_WR%` per strategy type
- `optimal_MIN_HOLD[strategy]` = value with highest PF on train

### RUN47.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find optimal MIN_HOLD per strategy type per coin
2. Test: evaluate on held-out month with those per-strategy values

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- For each strategy type, at least one coin's optimal value is consistent across ≥ 2/3 windows
- Portfolio OOS P&L ≥ baseline

### RUN47.3 — Combined Comparison

Side-by-side:

| Strategy | Baseline MIN_HOLD | Optimal MIN_HOLD | WR Delta | PF Delta |
|----------|-----------------|-----------------|----------|----------|
| VwapRev | 2 | X | +Ypp | +0.XX |
| AdrRev | 2 | X | +Ypp | +0.XX |
| BbBounce | 2 | X | +Ypp | +0.XX |
| MeanRev | 2 | X | +Ypp | +0.XX |
| DualRsi | 2 | X | +Ypp | +0.XX |
| OuMeanRev | 2 | X | +Ypp | +0.XX |

| Metric | Baseline (v16) | Per-Strategy MIN_HOLD | Delta |
|--------|---------------|----------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Avg Held Bars | X | X | +N |

---

## Why This Could Fail

1. **MIN_HOLD=2 is already near-optimal:** Most profitable trades in mean-reversion happen within 1-3 bars. Optimizing beyond this may yield marginal gains that don't survive OOS.
2. **Strategy-speed hypothesis is wrong:** All strategies may be mean-reverting at roughly the same speed when measured in 15m bars. The apparent speed difference between VwapRev and AdrRev may be due to the coin, not the strategy type.
3. **Per-coin variance dominates:** The "optimal" MIN_HOLD for a strategy may vary so much by coin that per-strategy optimization has no predictive power.

---

## Why It Could Succeed

1. **Mechanically motivated:** VwapRev (price crossing VWAP) and AdrRev (price reaching ADR band) have fundamentally different reversion distances. VwapRev typically needs 1-2 bars; AdrRev may need 4-6.
2. **Low-risk change:** This doesn't change entry signals or stop losses — only the minimum hold time before exits are evaluated. The downside is limited to opportunity cost (missed exits).
3. **Complements the partial reversion exit (RUN46):** If RUN46 is also run, per-strategy MIN_HOLD + partial reversion exit could compound their effects — faster strategies exit sooner, slower strategies hold longer.

---

## Comparison to Baseline

| | Current Global MIN_HOLD (v16) | RUN47 Per-Strategy MIN_HOLD |
|--|--|--|
| MIN_HOLD | Global constant = 2 | Strategy-specific (1–6) |
| Per-coin optimization | None | Per coin × per strategy |
| Exit timing | Uniform | Strategy-adaptive |
| VwapRev MIN_HOLD | 2 | 1 (faster) |
| AdrRev MIN_HOLD | 2 | 4 (slower) |
| Expected WR% | ~38% | +2–4pp |
| Expected PF | ~0.88 | +0.05–0.10 |
