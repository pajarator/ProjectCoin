# RUN76 — Volatility-Adaptive Stop Loss: Dynamic SL Tightening Based on ATR Percentile Rank

## Hypothesis

**Named:** `vol_adaptive_stop`

**Mechanism:** COINCLAW currently uses a fixed `STOP_LOSS = 0.30%` for all regime trades regardless of market volatility conditions. But volatility is not constant — ATR varies 3-5× across coins and over time. A fixed SL is either too loose during high-volatility spikes (large losses per trade) or too tight during low-volatility periods (stopped out before mean reversion completes).

**Volatility-adaptive stop:** Scale STOP_LOSS inversely with recent ATR percentile rank:
- Measure each coin's ATR percentile rank over the last 100 bars (relative to its own ATR history)
- When ATR percentile is HIGH (volatility spike): tighten SL → `SL = BASE / atr_pct_rank`
- When ATR percentile is LOW (quiet market): widen SL → `SL = BASE / atr_pct_rank`
- Clamp between `SL_MIN = 0.15%` and `SL_MAX = 0.60%`

**Why this is not a duplicate:**
- RUN51 (DD SL widening) changes position sizing based on drawdown — not stop loss value
- RUN52 (z-confidence sizing) changes position size based on entry z-score — not market volatility
- RUN62 (regime BE) activates breakeven stop for regime trades — not a volatility-scaled SL
- No prior RUN has made STOP_LOSS a function of measured market volatility per coin

**Mechanistic rationale:** High-volatility environments mean-revert faster (price is making larger swings), so a tighter SL captures reversions before they become large losses. Low-volatility environments are slow mean-reversion markets — trades need more room to work, so a wider SL prevents premature stops.

---

## Proposed Config Changes

```rust
// RUN76: Volatility-Adaptive Stop Loss
pub const VOL_ADAPTIVE_SL_ENABLE: bool = true;
pub const VOL_SL_BASE: f64 = 0.0030;       // baseline 0.30% SL (current default)
pub const VOL_SL_ATR_WINDOW: usize = 14;   // ATR lookback (same as indicator)
pub const VOL_SL_PCT_WINDOW: usize = 100; // percentile rank lookback
pub const VOL_SL_MIN: f64 = 0.0015;        // minimum 0.15% SL (high-vol cap)
pub const VOL_SL_MAX: f64 = 0.0060;        // maximum 0.60% SL (low-vol cap)
pub const VOL_SL_MID_PCT: f64 = 0.50;      // median percentile for BASE_SL
```

**`state.rs` — CoinState additions:**
```rust
// Track per-coin ATR history for percentile computation
pub struct CoinState {
    // ... existing fields ...
    pub atr_history: Vec<f64>,     // rolling 100-bar ATR% history
}

// Position struct — store computed vol-adaptive SL at entry
pub struct Position {
    // ... existing fields ...
    pub vol_adaptive_sl: Option<f64>,  // computed SL at entry time
}
```

**`engine.rs` — compute_vol_adaptive_sl:**
```rust
/// Compute volatility-adaptive stop loss for a regime trade.
/// Returns (sl_long, sl_short) as price levels.
pub fn compute_vol_adaptive_sl(ind: &Ind15m, cs: &CoinState) -> (f64, f64) {
    if !config::VOL_ADAPTIVE_SL_ENABLE {
        return (ind.p * (1.0 - config::STOP_LOSS),
                ind.p * (1.0 + config::STOP_LOSS));
    }

    let atr_pct = ind.atr_pct;  // ATR as % of price (already computed)
    if atr_pct.is_nan() || atr_pct <= 0.0 {
        return (ind.p * (1.0 - config::STOP_LOSS),
                ind.p * (1.0 + config::STOP_LOSS));
    }

    // Compute percentile rank of current ATR% in atr_history
    let hist = &cs.atr_history;
    if hist.len() < 20 {
        return (ind.p * (1.0 - config::STOP_LOSS),
                ind.p * (1.0 + config::STOP_LOSS));
    }

    let rank = hist.iter().filter(|&&x| x < atr_pct).count() as f64 / hist.len() as f64;
    let clamped_rank = rank.clamp(0.10, 0.90);  // avoid extreme multipliers

    // VOL_SL_BASE / rank: high rank → tighter stop, low rank → wider stop
    let adaptive_sl_pct = config::VOL_SL_BASE / clamped_rank;
    let clamped_sl = adaptive_sl_pct.clamp(config::VOL_SL_MIN, config::VOL_SL_MAX);

    let sl_long = ind.p * (1.0 - clamped_sl);
    let sl_short = ind.p * (1.0 + clamped_sl);
    (sl_long, sl_short)
}

// In check_exit — replace fixed STOP_LOSS with vol_adaptive_sl:
let eff_sl = if !pos.vol_adaptive_sl.is_none() {
    pos.vol_adaptive_sl.unwrap()
} else {
    config::STOP_LOSS
};
// For long: if pnl <= -eff_sl
// For short: if pnl <= -eff_sl
```

**`indicators.rs` — Ind15m additions:**
```rust
pub struct Ind15m {
    // ... existing fields ...
    pub atr_pct: f64,  // ATR / price * 100  (e.g., 0.45 = 0.45% ATR)
}
```

---

## Validation Method

### RUN76.1 — Volatility-Adaptive SL Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed `STOP_LOSS = 0.30%`

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `VOL_SL_BASE` | [0.0020, 0.0030, 0.0040] |
| `VOL_SL_PCT_WINDOW` | [50, 100, 200] |
| `VOL_SL_MIN` | [0.0010, 0.0015, 0.0020] |
| `VOL_SL_MAX` | [0.0050, 0.0060, 0.0080] |

**Per coin:** 3 × 3 × 3 × 3 = 81 configs × 18 coins = 1,458 backtests

**Key metrics:**
- `avg_vol_adaptive_sl`: average computed SL across all trades
- `sl_spread`: `VOL_SL_MAX - VOL_SL_MIN` as % of price (effective range)
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_loss_delta`: change in average loss per losing trade

### RUN76.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `VOL_SL_BASE × VOL_SL_MIN × VOL_SL_MAX` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Average SL stays within [0.15%, 0.60%] range
- No coin shows WR degradation >5pp vs baseline

### RUN76.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed 0.30%) | Vol-Adaptive SL | Delta |
|--------|---------------------------|-----------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Avg Loss% | -0.30% | X% | +/-Xbp |
| SL Range Used | 0.30% (fixed) | X%–X% | — |
| High-Vol Avg SL | 0.30% | X% | — |
| Low-Vol Avg SL | 0.30% | X% | — |

---

## Why This Could Fail

1. **Mean reversion timing is not controlled by volatility:** A wider swing in high volatility doesn't mean reversion happens faster — it just means the noise is larger. The reversion may still take the same number of bars.
2. **ATR is a lagging indicator:** By the time ATR spikes, the volatile move may already be reversing. Computing SL at entry using current ATR means we're reacting to yesterday's volatility.
3. **Coins have different baseline volatility:** SHIB has very different ATR characteristics than BTC. A one-size-fits-all percentile rank may not be comparable across coins.

---

## Why It Could Succeed

1. **Volatility is the primary driver of loss size:** If SL = 0.30% in a coin with 1.5% ATR, a single trade can lose 2× the intended risk. Vol-adaptive SL caps this.
2. **Low-vol environments need room:** During quiet markets (ADR < 1%), price oscillates in tight ranges. A 0.30% SL catches these oscillations before mean reversion. A wider SL (0.50-0.60%) lets the trade work.
3. **Simple and intuitive:** The math is clean — divide a fixed base by the volatility rank. No new indicators needed.
4. **Institutional practice:** Risk-managed stops that account for realized volatility are standard in systematic trading.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN76 Vol-Adaptive SL |
|--|--|--|
| Stop loss | Fixed 0.30% | Dynamic 0.15%–0.60% |
| High-vol ATR (e.g., 1.5%) | 0.30% = 20% of ATR | ~0.20% = 13% of ATR |
| Low-vol ATR (e.g., 0.3%) | 0.30% = 100% of ATR | ~0.60% = 200% of ATR |
| Per-trade loss control | Fixed | Adaptive |
| Implementation complexity | None | Low (ATR history + rank) |
