# RUN95 — Scalp Momentum Alignment: Require 1m Scalp Entries to Align With 15m Regime Direction

## Hypothesis

**Named:** `scalp_momentum_align`

**Mechanism:** COINCLAW's scalp overlay uses F6 filter (`dir_roc_3 < -0.195` blocks counter-momentum entries) but doesn't check the broader 15m momentum context. Scalp trades on 1m timeframes can fire in any direction regardless of whether the 15m chart shows a trending or mean-reverting environment. The Scalp Momentum Alignment filter requires scalp entries to be consistent with the 15m regime: in LONG market mode, only long scalp entries are allowed; in SHORT market mode, only short scalp entries are allowed; scalps in the counter-trend direction are suppressed.

**Scalp Momentum Alignment:**
- In MarketMode::Long (breadth ≤ 20%): allow scalp LONG entries only, block scalp SHORT entries
- In MarketMode::Short (breadth ≥ 50%): allow scalp SHORT entries only, block scalp LONG entries
- In MarketMode::IsoShort (20-50%): allow both LONG and SHORT scalp entries (mixed market)
- Additionally: require 15m z-score to be in the favorable range for the scalp direction
  - For scalp LONG: require 15m z < Z_SCALP_ALIGN_MAX (e.g., -0.5) — price should be below mean
  - For scalp SHORT: require 15m z > Z_SCALP_ALIGN_MIN (e.g., +0.5) — price should be above mean

**Why this is not a duplicate:**
- RUN12 (scalp market mode filter) already aligns scalp direction with market mode (no shorts in LONG, no longs in SHORT) — RUN95 adds the 15m z-score alignment requirement
- RUN40 (BTC_DOM_SCALE) uses btc_z − avg_z spread for scalp — this uses 15m z-score for the specific coin being traded
- No prior RUN has required the 15m z-score to be in the favorable range for scalp entries

**Mechanistic rationale:** Scalp trades are fastest timeframes and most susceptible to noise. Requiring the 15m context to be favorable — not just the 1m context — adds a multi-timeframe confirmation that reduces false scalp entries in trending markets. A scalp LONG at z = +1.0 on the 15m is fighting a strong uptrend; requiring 15m z < -0.5 ensures we're scalping with the grain.

---

## Proposed Config Changes

```rust
// RUN95: Scalp Momentum Alignment
pub const SCALP_MOMENTUM_ALIGN_ENABLE: bool = true;
pub const SCALP_Z_ALIGN_MAX_LONG: f64 = -0.5;   // 15m z must be < -0.5 for scalp LONG
pub const SCALP_Z_ALIGN_MIN_SHORT: f64 = 0.5;   // 15m z must be > +0.5 for scalp SHORT
```

**`engine.rs` — check_scalp_entry modified:**
```rust
pub fn check_scalp_entry(state: &mut SharedState, ci: usize) {
    if state.coins[ci].pos.is_some() { return; }

    // ... existing staleness checks ...

    let ind_1m = match &state.coins[ci].ind_1m {
        Some(i) => i.clone(),
        None => return,
    };

    let price = state.coins[ci].candles_1m.last().map(|c| c.c).unwrap_or(0.0);
    if price == 0.0 { return; }

    if let Some((dir, strat_name)) = strategies::scalp_entry_with_price(&ind_1m, price) {
        // RUN95: Scalp Momentum Alignment
        if config::SCALP_MOMENTUM_ALIGN_ENABLE {
            // Get 15m z-score for this coin
            let z_15m = state.coins[ci].ind_15m.as_ref()
                .map(|i| i.z)
                .unwrap_or(0.0);

            // Market mode check
            match state.market_mode {
                MarketMode::Long => {
                    // Only allow LONG scalps in LONG mode
                    if dir != Direction::Long {
                        return;  // block SHORT scalp in LONG mode
                    }
                }
                MarketMode::Short => {
                    // Only allow SHORT scalps in SHORT mode
                    if dir != Direction::Short {
                        return;  // block LONG scalp in SHORT mode
                    }
                }
                MarketMode::IsoShort => {
                    // Allow both in IsoShort mode
                }
            }

            // 15m z-score alignment check
            match dir {
                Direction::Long => {
                    if z_15m >= config::SCALP_Z_ALIGN_MAX_LONG {
                        return;  // 15m not favorable for scalp LONG
                    }
                }
                Direction::Short => {
                    if z_15m <= config::SCALP_Z_ALIGN_MIN_SHORT {
                        return;  // 15m not favorable for scalp SHORT
                    }
                }
            }
        }

        let regime = state.coins[ci].regime;
        open_position(state, ci, price, &regime.to_string(), strat_name, dir, TradeType::Scalp);
    }
}
```

---

## Validation Method

### RUN95.1 — Scalp Momentum Alignment Grid Search (Rust, parallel across 18 coins)

**Data:** 15m + 1m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — scalp entries use F6 only, no 15m alignment

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `SCALP_Z_ALIGN_MAX_LONG` | [-0.3, -0.5, -0.7] |
| `SCALP_Z_ALIGN_MIN_SHORT` | [0.3, 0.5, 0.7] |

**Per coin:** 3 × 3 = 9 configs × 18 coins = 162 backtests

**Key metrics:**
- `scalp_block_rate`: % of scalp entries blocked by alignment filter
- `mode_alignment_rate`: % of scalp entries consistent with market mode
- `PF_delta`: profit factor change vs baseline
- `scalp_PnL_delta`: scalp P&L change vs baseline
- `scalp_WR_delta`: scalp win rate change vs baseline

### RUN95.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best Z_ALIGN thresholds per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS scalp P&L delta vs baseline
- Scalp block rate 10–35% (meaningful filtering)
- Blocked entries have lower win rate than allowed entries (filter working)

### RUN95.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, F6 only) | Scalp Momentum Alignment | Delta |
|--------|------------------------|------------------------|-------|
| Scalp P&L | $X | $X | +$X |
| Scalp Win Rate | X% | X% | +Ypp |
| Scalp Profit Factor | X.XX | X.XX | +0.XX |
| Scalp Entries Blocked | 0% | X% | — |
| 15m Z at Allowed LONG | X | X | — |
| 15m Z at Allowed SHORT | X | X | — |
| Mode-Aligned Entry % | X% | X% | +Y% |

---

## Why This Could Fail

1. **Scalp operates on its own timeframe:** The 1m scalp entry logic is designed to be independent of the 15m regime. Requiring 15m alignment may filter out valid 1m setups that the 1m indicators already capture.
2. **F6 already does the heavy lifting:** The F6 filter (`dir_roc_3 < -0.195`) already blocks counter-momentum entries on the 1m. Adding 15m z alignment may be redundant.
3. **Mode alignment is already in RUN12:** RUN12 confirmed that scalp direction should match market mode. This RUN adds z-score alignment on top — the marginal value may be small.

---

## Why It Could Succeed

1. **Multi-timeframe confirmation reduces noise:** The 1m scalp can fire in any 15m environment. Requiring the 15m to be in the favorable range (z extreme in the direction of trade) ensures we're not scalping against a strong 15m trend.
2. **Captures the "trend is your friend" principle:** A scalp LONG when 15m z = +1.0 is fighting a strong uptrend. The scalp might work once but will fail more often than not. The z-alignment filter blocks this.
3. **Additive to RUN12:** RUN12 confirmed market mode alignment. This adds z-score alignment — a more precise measure of the 15m environment than just breadth.
4. **Simple and interpretable:** One comparison per scalp entry. Easy to understand and backtest.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN95 Scalp Momentum Alignment |
|--|--|--|
| Scalp direction filter | F6 (counter-momentum only) | F6 + market mode + 15m z-score |
| LONG scalp in LONG mode | Allowed | Allowed |
| SHORT scalp in SHORT mode | Allowed | Allowed |
| LONG scalp in SHORT mode | Blocked (RUN12) | Blocked (RUN12 + z-align) |
| 15m z for scalp LONG | Not checked | Must be < -0.5 |
| 15m z for scalp SHORT | Not checked | Must be > +0.5 |
| Multi-timeframe alignment | None | 1m signal + 15m z confirmation |
