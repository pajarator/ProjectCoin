# RUN78 — Cross-Coin Z-Score Confirmation: BTC Z as Market-Wide Regime Filter

## Hypothesis

**Named:** `cross_coin_z_confirm`

**Mechanism:** COINCLAW currently uses `MarketCtx.btc_z` only for the `IsoDivergence` strategy (RUN55), where BTC z-score being negative confirms an ISO short entry. But BTC's z-score contains broader market information: when BTC is deeply negative (z < -1.5), altcoins tend to correlate — the whole market is in risk-off mode and mean reversion works differently than when BTC is near zero or positive.

**Cross-Coin Z-Score Confirmation Filter:**
- For regime LONG entries on any coin: require `btc_z >= BTC_Z_MIN_LONG` (e.g., -1.0)
- For regime SHORT entries on any coin: require `btc_z <= BTC_Z_MAX_SHORT` (e.g., +1.0)
- When BTC z-score is in the "danger zone" (e.g., between -1.5 and +1.5), all regime entries are suppressed
- Momentum trades (RUN27/28) are exempt — momentum thrives on directional market moves

**Why this is not a duplicate:**
- RUN55 (BTC divergence for ISO shorts) uses BTC z < 0 as a confirmation for a specific ISO short condition — this uses BTC z as a blanket filter on ALL regime entries for ALL coins
- RUN40 (BTC_DOM_SCALE) uses the btc_z − avg_z spread for scalp entries — this uses absolute BTC z for regime entries
- RUN63 (BTC trend filter) uses BTC SMA+return for regime entries — this uses z-score (deviation from mean) rather than trend
- No prior RUN has used BTC z-score as a market-wide regime-entry gate

**Mechanistic rationale:** BTC's z-score captures systemic market risk. When BTC is deeply negative, the market is under stress — regime mean reversion may not work as expected (the whole market is moving down together). When BTC is near zero, regime entries are more reliable. This is a simple, portfolio-level filter that requires no new indicators.

---

## Proposed Config Changes

```rust
// RUN78: Cross-Coin Z-Score Confirmation
pub const CROSS_Z_CONFIRM_ENABLE: bool = true;
pub const CROSS_Z_BTC_MIN_LONG: f64 = -1.0;   // BTC z must be >= -1.0 for LONG entries
pub const CROSS_Z_BTC_MAX_SHORT: f64 = 1.0;    // BTC z must be <= +1.0 for SHORT entries
pub const CROSS_Z_SUPPRESS_ZONE: f64 = 1.5;   // if |btc_z| < 1.5, all regime entries suppressed
pub const CROSS_Z_MOMENTUM_EXEMPT: bool = true; // momentum trades ignore this filter
```

**`engine.rs` — modify check_entry:**
```rust
/// Check if BTC z-score confirms a regime entry direction.
fn cross_coin_z_confirmed(state: &SharedState, dir: Direction) -> bool {
    if !config::CROSS_Z_CONFIRM_ENABLE { return true; }

    let btc_z = state.coins
        .iter()
        .find(|c| c.name == "BTC")
        .and_then(|c| c.ind_15m.as_ref().map(|i| i.z))
        .unwrap_or(0.0);

    // Momentum exempt
    if config::CROSS_Z_MOMENTUM_EXEMPT { return true; }

    // Suppress zone: |btc_z| < threshold means BTC is near mean — market neutral
    // Allow entries in both directions during suppress zone
    if btc_z.abs() < config::CROSS_Z_SUPPRESS_ZONE { return true; }

    match dir {
        Direction::Long => btc_z >= config::CROSS_Z_BTC_MIN_LONG,
        Direction::Short => btc_z <= config::CROSS_Z_BTC_MAX_SHORT,
    }
}

// In check_entry, after regime signal fires but before open_position:
if !cross_coin_z_confirmed(state, dir) {
    return;  // suppress entry
}
```

**`state.rs` — no changes required** (BTC z is already available via `MarketCtx.btc_z`)

---

## Validation Method

### RUN78.1 — Cross-Coin Z-Confirm Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no BTC z-score filter on regime entries

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CROSS_Z_BTC_MIN_LONG` | [-0.5, -1.0, -1.5] |
| `CROSS_Z_BTC_MAX_SHORT` | [0.5, 1.0, 1.5] |
| `CROSS_Z_SUPPRESS_ZONE` | [1.0, 1.5, 2.0] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `entry_suppression_rate`: % of regime entries blocked by BTC z filter
- `btc_z_at_entries`: distribution of BTC z-score at entry (should shift positive for longs)
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN78.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `BTC_MIN_LONG × BTC_MAX_SHORT` threshold pair per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Entry suppression rate 10–40% (meaningful filtering without over-suppressing)
- Momentum trades unaffected (if `CROSS_Z_MOMENTUM_EXEMPT = true`)

### RUN78.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no filter) | Cross-Coin Z Confirm | Delta |
|--------|--------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Entries Suppressed | 0% | X% | — |
| BTC z at Long Entry | X | X | +N |
| BTC z at Short Entry | X | X | −N |
| Avg BTC z at Entry | X | X | — |

---

## Why This Could Fail

1. **BTC z-score can be neutral while altcoins have good setups:** The suppress zone (|btc_z| < 1.5) would block entries when BTC is near its mean — but this is exactly when altcoin regime trades may be most reliable. Over-filtering reduces edge.
2. **BTC and altcoin z-scores are not independent:** In a BTC-led selloff, all z-scores move together. By the time BTC's z recovers enough to allow LONG entries, the best mean reversion opportunity is gone.
3. **Momentum exemption undermines the thesis:** If momentum trades are exempt during high-stress periods, the filter may not reduce drawdowns as intended.

---

## Why It Could Succeed

1. **Market-wide risk is real:** BTC is the market. When BTC is deeply negative, regime mean reversion on alts is fighting a systemic headwind. Filtering entries during these periods avoids low-probability setups.
2. **Simple, no new data:** BTC z-score is already computed and available. This is a one-line filter in `check_entry`.
3. **Walk-forward friendly:** BTC z is a stable, mean-reverting indicator. Thresholds learned on historical data are more likely to generalize than thresholds on noisy coin-specific signals.
4. **Institutional practice:** Cross-asset confirmation is standard — no equity long trade when VIX is spiking.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN78 Cross-Coin Z Confirm |
|--|--|--|
| BTC z filter on entries | None | LONG requires btc_z ≥ -1.0; SHORT requires btc_z ≤ +1.0 |
| Entries suppressed | 0% | X% |
| Market risk handling | None | Explicit BTC z gate |
| Implementation complexity | None | Low (one comparison per entry) |
| Momentum trades | Always allowed | Exempt from filter |
