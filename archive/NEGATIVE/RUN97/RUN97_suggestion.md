# RUN97 — BB Width Scalp Gate: Suppress Scalp Entries When Bollinger Band Width Indicates Low Volatility

## Hypothesis

**Named:** `bb_width_scalp_gate`

**Mechanism:** Scalp trades profit from small price oscillations within a tight range. When Bollinger Band width is narrow (low volatility), the oscillation range is compressed — price bounces between bands less violently, and scalp TP targets (0.8%) are harder to reach. The BB Width Scalp Gate suppresses scalp entries when BB width is below a threshold, allowing scalp to fire only when there's enough volatility for the 0.8% TP to be achievable.

**BB Width Scalp Gate:**
- Measure each coin's BB width relative to its recent average: `bb_width_ratio = bb_width / bb_width_avg`
- When `bb_width_ratio < BB_SCALP_WIDTH_MIN` (e.g., 0.60) for the specific coin:
  - Suppress all scalp entries for that coin
  - Regime entries remain unaffected
- BB width is already computed per coin — just need a threshold comparison

**Why this is not a duplicate:**
- RUN65 (BB squeeze duration) used BB width for regime entry filtering — this uses BB width specifically for SCALP entry filtering
- RUN54 (vol entry threshold) used ATR percentile rank for regime entries — this uses BB width ratio for scalp entries
- RUN71 (BB width filter) used BB width as a regime filter — this is specifically a scalp gate, not a regime filter
- No prior RUN has gated scalp entries based on BB width ratio

**Mechanistic rationale:** Scalp TP = 0.8% requires price to oscillate at least that much within the scalp hold period. When BB width is narrow (bb_width_ratio < 0.60), the coin is in a low-volatility squeeze — the oscillation is too small for the scalp strategy to reach 0.8% TP before reversing. Suppressing scalp during these periods avoids entries that are unlikely to hit TP and will likely return to the mean without triggering either TP or SL.

---

## Proposed Config Changes

```rust
// RUN97: BB Width Scalp Gate
pub const BB_SCALP_GATE_ENABLE: bool = true;
pub const BB_SCALP_WIDTH_MIN: f64 = 0.60;  // scalp suppressed when bb_width / bb_width_avg < 0.60
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

    // RUN97: BB Width Scalp Gate
    if config::BB_SCALP_GATE_ENABLE {
        // Check BB width ratio on 1m timeframe
        if !ind_1m.bb_width_avg.is_nan() && ind_1m.bb_width_avg > 0.0 {
            let bb_ratio = ind_1m.bb_width / ind_1m.bb_width_avg;
            if bb_ratio < config::BB_SCALP_WIDTH_MIN {
                return;  // suppress scalp — not enough volatility
            }
        }
    }

    if let Some((dir, strat_name)) = strategies::scalp_entry_with_price(&ind_1m, price) {
        let regime = state.coins[ci].regime;
        open_position(state, ci, price, &regime.to_string(), strat_name, dir, TradeType::Scalp);
    }
}
```

---

## Validation Method

### RUN97.1 — BB Width Scalp Gate Grid Search (Rust, parallel across 18 coins)

**Data:** 1m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — no BB width gate on scalp entries

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `BB_SCALP_WIDTH_MIN` | [0.50, 0.60, 0.70, 0.80] |

**Per coin:** 4 configs × 18 coins = 72 backtests

**Key metrics:**
- `bb_gate_block_rate`: % of scalp entries blocked by BB width gate
- `bb_ratio_at_blocked`: average bb_ratio of blocked entries
- `scalp_PF_delta`: scalp profit factor change vs baseline
- `scalp_WR_delta`: scalp win rate change vs baseline
- `scalp_PnL_delta`: scalp P&L change vs baseline

### RUN97.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best BB_SCALP_WIDTH_MIN per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS scalp P&L delta vs baseline
- Block rate 15–40% (meaningful filtering without over-suppressing)
- Blocked entries have lower win rate than allowed entries

### RUN97.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, no BB gate) | BB Width Scalp Gate | Delta |
|--------|--------------------------|---------------------|-------|
| Scalp P&L | $X | $X | +$X |
| Scalp Win Rate | X% | X% | +Ypp |
| Scalp Profit Factor | X.XX | X.XX | +0.XX |
| Scalp Entries Blocked | 0% | X% | — |
| Avg BB Ratio (blocked) | — | X | — |
| Avg BB Ratio (allowed) | — | X | — |
| Non-Blocked Scalp WR% | X% | X% | +Ypp |

---

## Why This Could Fail

1. **BB width is a lagging indicator:** By the time BB width drops below the threshold, the low-volatility period may already be ending. The gate may suppress entries at exactly the wrong time.
2. **Scalp doesn't need wide BB to work:** The scalp strategy uses RSI and stochastic for entries, not BB width. A coin with narrow BB could still oscillate enough for the scalp TP to fire — the gate may be filtering valid entries.
3. **Regime entries unaffected:** If BB width is narrow, regime entries are still allowed — but the regime entry conditions (z-score, volume) already filter low-volatility environments indirectly.

---

## Why It Could Succeed

1. **Directly targets scalp's Achilles heel:** Scalp TP = 0.8% requires oscillation. When BB width is narrow, the oscillation is mechanically constrained. Filtering scalp entries during these periods is principled.
2. **Already computed:** BB width and BB width avg are already computed in Ind1m. No new indicators needed — just a threshold comparison.
3. **Focused improvement:** RUN29 confirmed scalp loses on all 18 coins. Any improvement to scalp filtering is valuable. This gate directly addresses the volatility precondition for scalp success.
4. **Low complexity:** One comparison per scalp entry check. Minimal code, clear logic.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN97 BB Width Scalp Gate |
|--|--|--|
| Scalp in narrow BB | Always allowed | Blocked when bb_ratio < 0.60 |
| Scalp in wide BB | Always allowed | Allowed |
| Regime entries | Unaffected | Unaffected |
| Volatility awareness | None (scalp) | BB width ratio gate |
| TP achievability check | None | Implicit (BB width = oscillation range) |
| Block rate | 0% | X% |
