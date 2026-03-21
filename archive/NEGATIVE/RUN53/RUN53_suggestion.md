# RUN53 — Partial Exit / Scale-Out on Regime Trades: Progressive Profit-Taking

## Hypothesis

**Named:** `partial_scale_out`

**Mechanism:** COINCLAW currently exits positions in full at a single exit signal (SMA20 crossback, Z-score reversion, or SL). For large moves that take 10+ bars to develop, exiting the entire position at the signal exit means leaving significant unrealized profit on the table if the move continues after the exit.

The hypothesis is that **partial exits in tiers** improve risk-adjusted returns:
- Exit 33% of position when profit reaches `tier1_pnl%` (e.g., +0.5%)
- Exit another 33% when profit reaches `tier2_pnl%` (e.g., +1.0%)
- Hold remaining 33% until full signal exit (SMA/Z0)

This approach:
1. Secures gains early, reducing exposure
2. Keeps 33% in the trade to capture extended moves
3. Improves Sharpe ratio by raising the average win while capping the average loss at SL

**Why this is not a duplicate:**
- RUN8 tested fixed TP (exit all at a target) — partial exits scale out progressively, not all at once
- Scalp has TP but regime trades don't — this adds tiered profit-taking to regime
- No prior RUN has tested scale-out or partial exits
- This changes the exit paradigm from "full position at signal" to "tiered position reduction"

---

## Proposed Config Changes

```rust
// RUN53: Partial Scale-Out / Tiered Profit-Taking
pub const PARTIAL_EXIT_ENABLE: bool = true;
pub const PARTIAL_EXIT_TIERS: u8 = 3;        // number of partial exit tiers (3 = thirds)
pub const PARTIAL_TIER1_PNL: f64 = 0.004;  // first tier exit at +0.4% PnL
pub const PARTIAL_TIER2_PNL: f64 = 0.008;  // second tier at +0.8%
// third tier = hold until signal exit (SMA/Z0/SL)
// Remaining position at signal exit is whatever is left
```

**`state.rs` — add position tracking for partial exits:**
```rust
pub struct Position {
    pub e: f64,
    pub s: f64,
    pub high: f64,
    pub low: f64,
    pub margin: f64,
    pub dir: String,
    pub last_price: Option<f64>,
    pub trade_type: Option<TradeType>,
    pub atr_stop: Option<f64>,
    pub trail_distance: Option<f64>,
    pub trail_act_price: Option<f64>,
    pub scalp_bars_held: Option<u32>,
    pub be_active: Option<bool>,
    pub z_at_entry: Option<f64>,
    pub exit_tier: u8,          // NEW: which tier has been exited (0 = none, 1 = tier1 done, etc.)
    pub original_size: f64,      // NEW: original position size for calculating remaining fraction
}
```

**`engine.rs` — tiered exit logic in `check_exit`:**
```rust
fn check_partial_exit(state: &mut SharedState, ci: usize, price: f64) -> bool {
    let cs = &state.coins[ci];
    let pos = match &cs.pos {
        Some(p) => p.clone(),
        None => return false,
    };
    if pos.exit_tier >= config::PARTIAL_EXIT_TIERS { return false; }

    let pnl_pct = match pos.dir.as_str() {
        "long" => (price - pos.e) / pos.e,
        "short" => (pos.e - price) / pos.e,
        _ => return false,
    };

    // Check each tier in order
    let tier_trigger = match pos.exit_tier {
        0 => config::PARTIAL_TIER1_PNL,
        1 => config::PARTIAL_TIER2_PNL,
        _ => return false,
    };

    if pnl_pct >= tier_trigger {
        let fraction = 1.0 / (config::PARTIAL_EXIT_TIERS - pos.exit_tier) as f64;  // e.g., 1/3
        let exit_size = pos.s * fraction;
        let pnl = if pos.dir == "long" {
            exit_size * (price - pos.e)
        } else {
            exit_size * (pos.e - price)
        };
        let margin_released = exit_size * pos.e;  // release margin proportionally

        // Update position
        if let Some(ref mut p) = state.coins[ci].pos {
            p.s -= exit_size;
            p.exit_tier += 1;
        }
        cs.bal += pnl;
        cs.trades.push(TradeRecord {
            pnl,
            reason: format!("PARTIAL_T{}", pos.exit_tier + 1),
            dir: pos.dir.clone(),
            trade_type: pos.trade_type,
        });
        // Don't close position — keep remaining portion
        state.log(format!(
            "PARTIAL EXIT {} [{}] {}% @ {} | tier {} | ${:.2}",
            cs.name, pos.dir, fraction * 100.0, fmt_price(price),
            pos.exit_tier + 1, pnl
        ));
        return true;  // position still open, check exit next bar
    }
    false
}
```

---

## Validation Method

### RUN53.1 — Partial Exit Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — full exit at signal (SMA/Z0) or SL

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `PARTIAL_EXIT_TIERS` | [2, 3, 4] |
| `PARTIAL_TIER1_PNL` | [0.003, 0.004, 0.005, 0.006] |
| `PARTIAL_TIER2_PNL` | [0.006, 0.008, 0.010, 0.012] |

**Per coin:** 3 × 4 × 4 = 48 configs × 18 coins = 864 backtests

**Key metrics:**
- `partial_exit_rate`: % of trades that hit at least one partial exit
- `avg_tiers_used`: average number of partial tiers executed per trade
- `Sharpe_delta`: Sharpe ratio change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `avg_win_delta`: average win $ change (should increase due to securing gains early)

### RUN53.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best partial exit tier parameters per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS Sharpe delta vs baseline
- Portfolio OOS Sharpe ≥ baseline
- Portfolio max_DD < baseline (primary goal of partial exits is drawdown reduction)

### RUN53.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Partial Scale-Out | Delta |
|--------|---------------|------------------|-------|
| Total P&L | $X | $X | +$X |
| Sharpe Ratio | X.XX | X.XX | +0.XX |
| Max DD | X% | X% | −Ypp |
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Avg Win | $X | $X | +$X |
| Avg Loss | $X | $X | $X |
| Partial Exit Rate | 0% | X% | — |
| Avg Tiers Used | 0 | X | +N |
| Remaining at Signal | 100% | X% | — |

---

## Why This Could Fail

1. **Partial exits reduce winners:** If you exit 33% at +0.4% but the full exit doesn't come for another 10 bars, you've reduced your upside on a winning trade.
2. **Tier thresholds are arbitrary:** The optimal tier levels depend on the specific coin's typical reversion magnitude. Grid search finds optimal but it may not generalize OOS.
3. **Interaction with SMA/Z0 exits:** The partial exit reduces position size, but the signal exit still fires at the same time. The remaining 33% still gets hit by the same SL if the trade reverses.

---

## Why It Could Succeed

1. **Improves Sharpe without changing WR:** By securing partial gains early, average win $ increases while average loss $ stays the same (SL is unchanged). This mechanically raises Sharpe ratio.
2. **Reduces max drawdown:** Exiting 33% early means 33% less exposure to subsequent adverse moves. Particularly effective for coins with volatile reversion paths.
3. **Psychologically sound:** Traders often exit too early on their own. Implementing it systematically removes emotion from the equation.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN53 Partial Scale-Out |
|--|--|--|
| Exit type | Full position at signal | Tiered partial exits |
| Tiers | 1 (full exit) | 2–4 tiers |
| Tier triggers | SMA/Z0 | PnL thresholds |
| Avg position exposure | 100% until exit | 67%→44%→33% over tiers |
| Expected Sharpe | X.XX | +0.05–0.15 |
| Expected Max DD | X% | −15–25% |
| Partial exit rate | 0% | 60–80% of trades |
