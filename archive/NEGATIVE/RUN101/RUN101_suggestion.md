# RUN101 — Partial Position Split: Split Each Position Into Core and Satellite Halves With Different Exits

## Hypothesis

**Named:** `partial_position_split`

**Mechanism:** COINCLAW currently manages each position as a single unit with one set of exit rules. But the optimal exit for a position changes over its lifetime: early in the trade, we want to give it room to work; later in the trade, we want to protect profits. The Partial Position Split splits each regime entry into two equal halves: a "core" position (50%) and a "satellite" position (50%). The core uses strict exit rules (tight SL, early SMA/Z0), while the satellite uses宽松 exit rules (wider SL, later exits, allows holding through chop).

**Partial Position Split:**
- At entry: open TWO positions of equal size (50% of normal size each)
  - Core: uses normal SL, normal SMA/Z0 exit rules — disciplined, tight management
  - Satellite: uses wider SL (e.g., 1.5× normal), holds through temporary drawdowns, exits only at MAX_HOLD or Z_RECOVERY — gives the trade room to work
- Both positions share the same entry signal and direction
- Both tracked independently in `trades` with separate `TradeRecord` entries
- The split is invisible to the portfolio from a balance standpoint (total exposure = one full position)

**Why this is not a duplicate:**
- RUN53 (tiered partial exits) exits portions of a single position at different PnL thresholds — this splits into two independent positions with different exit rules
- RUN46 (partial reversion capture) exits at fixed PnL tiers — this differentiates by time in trade and exit strictness
- No prior RUN has split a single entry into two sub-positions with fundamentally different exit risk profiles

**Mechanistic rationale:** The best trades are the ones where we enter at the extreme and price moves our way immediately — these should be managed tightly (core). But some trades take time to develop — the satellite gives them room without forcing the core to hold through noise. This is analogous to "let winners run" (satellite) while protecting capital with strict management (core).

---

## Proposed Config Changes

```rust
// RUN101: Partial Position Split
pub const POSITION_SPLIT_ENABLE: bool = true;
pub const SPLIT_CORE_SIZE: f64 = 0.50;      // core = 50% of normal position
pub const SPLIT_SATELLITE_SIZE: f64 = 0.50; // satellite = 50% of normal position
pub const SPLIT_SL_MULT: f64 = 1.50;        // satellite uses 1.5× normal SL
pub const SPLIT_SAT_Z_EXIT: f64 = 0.0;      // satellite exits only at Z0 or MAX_HOLD (not early SMA)
```

**`state.rs` — Position additions:**
```rust
pub struct Position {
    // ... existing fields ...
    pub is_core: bool,           // true = core position, false = satellite
}

// In CoinPersist — track split trades separately:
pub struct TradeRecord {
    pub pnl: f64,
    pub reason: String,
    pub dir: String,
    pub trade_type: Option<TradeType>,
    pub is_core: bool,          // NEW: was this a core or satellite trade?
}
```

**`engine.rs` — open_split_position:**
```rust
/// Open a position split into core and satellite halves.
fn open_split_position(
    state: &mut SharedState,
    ci: usize,
    price: f64,
    regime: &str,
    strat: &str,
    dir: Direction,
    trade_type: TradeType,
) {
    let cs = &mut state.coins[ci];
    let base_risk = if trade_type == TradeType::Scalp {
        config::SCALP_RISK
    } else {
        config::RISK
    };

    let core_size = base_risk * config::SPLIT_CORE_SIZE;
    let sat_size = base_risk * config::SPLIT_SATELLITE_SIZE;

    // Core position — normal rules
    cs.pos = Some(Position {
        e: price,
        s: (core_size * config::LEVERAGE) / price,
        high: price,
        low: price,
        margin: core_size,
        dir: dir.to_string(),
        last_price: None,
        trade_type: Some(trade_type),
        atr_stop: None,
        trail_distance: None,
        trail_act_price: None,
        scalp_bars_held: None,
        be_active: None,
        is_core: true,
        z_at_entry: cs.ind_15m.as_ref().map(|i| i.z),
    });

    // Satellite position — separate tracking for wider exits
    // Note: only one position can be open per coin at a time in current architecture
    // For RUN101, we track the satellite as a separate "virtual" position in trades
    // when it exits. The core position holds the primary bal/pos tracking.
    // When satellite exits, it adds to bal directly via a separate mechanism.

    cs.candles_held = 0;
    cs.active_strat = Some(strat.to_string());
}

// Satellite position — separate bal addition when it exits
// For simplicity: the bal is tracked as a single position, but we log the satellite exit
// When the core exits, check if satellite also needs to be closed
fn close_satellite(cs: &mut CoinState, price: f64, reason: &str) {
    // Satellite exits when: MAX_HOLD, Z0 (satellite exits at Z0 not early SMA), or special rules
    // The satellite PnL = sat_size * LEVERAGE * price_diff / price
    // This is added to bal directly when satellite exits
}
```

---

## Validation Method

### RUN101.1 — Position Split Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — single position, uniform exit rules

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `SPLIT_CORE_SIZE` | [0.40, 0.50, 0.60] |
| `SPLIT_SL_MULT` | [1.2, 1.5, 2.0] |
| `SPLIT_SAT_Z_EXIT` | [0.0, 0.2] (z-threshold for satellite exit) |

**Per coin:** 3 × 3 × 2 = 18 configs × 18 coins = 324 backtests

**Key metrics:**
- `core_exit_rate`: % of core vs satellite exits
- `core_vs_sat_pnl`: P&L comparison between core and satellite trades
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_held_delta`: change in average hold duration

### RUN101.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best SPLIT_CORE_SIZE × SL_MULT per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Core positions have higher win rate than satellite (confirming split is working)
- Satellite positions have higher avg PnL when they win (confirming "let winners run")

### RUN101.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, single pos) | Partial Position Split | Delta |
|--------|--------------------------|---------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Core Win Rate | X% | X% | +Ypp |
| Satellite Win Rate | — | X% | — |
| Core Avg PnL | $X | $X | +$X |
| Satellite Avg PnL | — | $X | +$X |

---

## Why This Could Fail

1. **Implementation complexity:** COINCLAW's architecture assumes one position per coin. Splitting into core/satellite requires significant refactoring of position tracking and balance management.
2. **Satellite holds through drawdowns:** The satellite uses a wider SL, but that means it can hold through large drawdowns that the core avoids. If the satellite holds through a -5% move, the psychological and capital impact may outweigh the benefit.
3. **Two trades, one signal:** The split treats two halves of the same trade differently — but they're the same trade. If the thesis was correct, both should be managed the same way.

---

## Why It Could Succeed

1. **Captures both fast and slow mean reversion:** The core catches quick reversions (exits early at SMA/Z0). The satellite holds for larger moves. Together they capture more of the opportunity set.
2. **Psychological discipline:** The core always exits at the planned stop — no regret about "what if I had held." The satellite provides the "let winners run" component without risking the core.
3. **Adaptable to trade personality:** Coins with fast mean reversion are dominated by core exits. Coins with slow mean reversion are captured by satellites. The split adapts to both.
4. **No additional capital at risk:** Total exposure is the same as a single position. The split is in exit management, not position size.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN101 Partial Position Split |
|--|--|--|
| Positions per entry | 1 | 2 (core + satellite) |
| Core exit rules | Normal | Normal SL, SMA/Z0 |
| Satellite exit rules | Same as core | Wider SL, Z0/MAX_HOLD only |
| Core size | 100% | 40-60% |
| Satellite size | N/A | 40-60% |
| Total exposure | 100% | 100% (sum of halves) |
| Fast reversion capture | One chance | Core captures fast, satellite holds slow |
