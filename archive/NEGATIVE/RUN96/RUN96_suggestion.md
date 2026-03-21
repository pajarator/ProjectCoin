# RUN96 — Z-Confluence Exit: Exit When Multiple Coins' Z-Scores Simultaneously Converge Toward Mean

## Hypothesis

**Named:** `z_confluence_exit`

**Mechanism:** COINCLAW currently exits individual positions based on individual coin z-score (Z0 exit: z crosses 0) or price action (SMA exit). But market-wide z-score convergence is a stronger signal — when many coins' z-scores are simultaneously converging toward their means, it means the market-wide mean reversion is complete. Exiting when multiple coins' z-scores have converged gives more confidence that the opportunity was real.

**Z-Confluence Exit:**
- Track market-wide z-score convergence: how many coins have z-scores within a band of zero (e.g., |z| < 0.5)
- When a position is open, if the count of "converged" coins (|z| < 0.5) reaches `CONFLUENCE_MIN_COINS` (e.g., 5+ out of 18):
  - Force exit all positions with reason `Z_CONFLUENCE`
  - The thesis: if most of the market has mean-reverted, the remaining open position is now fighting an improving market
- Alternative (less aggressive): only apply to the specific coin's own z — require `|z| < CONVERGENCE_Z_THRESH` AND `converged_coin_count >= CONVERGENCE_COIN_THRESH` before Z0 exit fires

**Why this is not a duplicate:**
- RUN70 (convergence threshold) checked z-score convergence across coins for entry filtering — this uses convergence for EXIT filtering
- RUN55 (divergence threshold) used BTC z < 0 as ISO short confirmation — this uses multi-coin z convergence as an exit accelerator
- No prior RUN has used market-wide z-score convergence as an exit condition

**Mechanistic rationale:** When 8 out of 18 coins have |z| < 0.5, the market-wide mean reversion episode is largely complete. Any remaining open LONG position is in a coin that hasn't yet converged — but if most coins have already reverted, there's less reason to believe this one will continue. Exiting in confluence with the broader market avoids holding positions in coins that are lagging the mean-reversion move.

---

## Proposed Config Changes

```rust
// RUN96: Z-Confluence Exit
pub const Z_CONFLUENCE_EXIT_ENABLE: bool = true;
pub const CONFLUENCE_Z_BAND: f64 = 0.50;   // z must be within ±0.5 of zero to count as converged
pub const CONFLUENCE_MIN_COINS: u32 = 5;     // minimum converged coins to activate confluence exit
pub const CONFLUENCE_MODE: &str = "soft";   // "soft" = add Z_CONFLUENCE exit reason, "hard" = force close all
```

**`engine.rs` — check_z_confluence_exit:**
```rust
/// Count how many coins have z-scores within the convergence band.
fn count_converged_coins(state: &SharedState) -> usize {
    state.coins.iter()
        .filter(|cs| {
            if let Some(ref ind) = cs.ind_15m {
                if !ind.z.is_nan() {
                    return ind.z.abs() < config::CONFLUENCE_Z_BAND;
                }
            }
            false
        })
        .count()
}

/// Check if confluence exit should fire for a specific position.
fn check_z_confluence_exit(state: &SharedState, cs: &CoinState, pos: &Position) -> bool {
    if !config::Z_CONFLUENCE_EXIT_ENABLE { return false; }
    if pos.trade_type != Some(TradeType::Regime) { return false; }

    let converged = count_converged_coins(state);
    if converged < config::CONFLUENCE_MIN_COINS as usize { return false; }

    // Soft mode: add Z_CONFLUENCE as an exit reason alongside Z0/SMA
    // In soft mode, the exit still requires the normal Z0/SMA trigger
    // But we add confluence context to the exit reason string
    if config::CONFLUENCE_MODE == "soft" {
        return false;  // soft mode just tags the reason, doesn't force exit
    }

    // Hard mode: force exit all positions when confluence is reached
    true
}

// In check_exit — for soft mode, add confluence to Z0 exit reason:
fn check_exit(/* ... */) -> bool {
    // ... existing logic ...

    // Soft mode: if confluence is active and Z0 would fire, add confluence tag
    if config::Z_CONFLUENCE_EXIT_ENABLE
        && config::CONFLUENCE_MODE == "soft"
        && converged >= config::CONFLUENCE_MIN_COINS as usize
        && z_crosses_zero {
        close_position(state, ci, price, "Z0_CONFLUENCE", TradeType::Regime);
        return true;
    }
}
```

---

## Validation Method

### RUN96.1 — Z-Confluence Grid Search (Rust, 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — exits based on individual coin z/sma, no confluence

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CONFLUENCE_Z_BAND` | [0.3, 0.5, 0.7] |
| `CONFLUENCE_MIN_COINS` | [4, 5, 6, 8] |
| `CONFLUENCE_MODE` | ["soft", "hard"] |

**Note:** This is a portfolio-level optimization. Total configs: 3 × 4 × 2 = 24.

**Key metrics:**
- `confluence_exit_rate`: % of regime trades exited with confluence tag/force
- `convergence_at_exit`: average converged coin count at confluence exit
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN96.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best CONFLUENCE params per window
2. Test: evaluate on held-out month

**Pass criteria:**
- Portfolio P&L delta ≥ 0 vs baseline
- Confluence exit rate 5–25% of regime trades
- Max drawdown does not increase >15% vs baseline

### RUN96.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, individual exits) | Z-Confluence Exit | Delta |
|--------|--------------------------------|------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Confluence Exit Rate | 0% | X% | — |
| Avg Converged at Exit | — | X | — |
| Soft vs Hard Mode | N/A | X | — |

---

## Why This Could Fail

1. **Market-wide convergence is a lagging indicator:** By the time 5+ coins have |z| < 0.5, the mean reversion episode is already over. Forcing exits at this point may be too late — the best exits happened earlier.
2. **Hard mode force-closes all positions:** If confluence activates mid-session, the "hard" mode closes all positions simultaneously — this could close positions at bad prices and override individual coin exit signals.
3. **The "lagging coin" may be the best trade:** If a coin hasn't converged yet, it may be the one with the most remaining reversion potential. Force-closing it because other coins have converged is counterproductive.

---

## Why It Could Succeed

1. **Market-wide mean reversion is coordinated:** When BTC and altcoins mean-revert, they tend to do so together. If most coins have converged, the specific coin is likely to follow soon. The exit is a market-wide confirmation.
2. **Prevents holding through reversals:** If 8/18 coins have already mean-reverted, the remaining open positions are in coins that are "late." These late coins often reverse next — the confluence exit avoids holding through this.
3. **Soft mode adds information without overriding:** The soft mode simply adds the confluence tag to Z0 exits — it doesn't force any new exits, just adds context that could be used in post-trade analysis.
4. **Simple and portfolio-aware:** Only requires counting converged coins — a trivial computation.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN96 Z-Confluence Exit |
|--|--|--|
| Exit trigger | Individual coin z-score/SMA | Individual + market-wide convergence |
| Market-wide awareness | None | Counts converged coins |
| Convergence threshold | N/A | 5+ coins within \|z\| < 0.5 |
| Soft mode | N/A | Tags exits, no forced closes |
| Hard mode | N/A | Force-closes all positions at confluence |
| Exit confidence | Per-coin only | Per-coin + market-wide |
