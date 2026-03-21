# RUN83 — Cooldown by Market Mode: Adaptive Cooldown Periods Based on Current Regime

## Hypothesis

**Named:** `cooldown_by_mode`

**Mechanism:** COINCLAW currently uses fixed cooldown periods after exits: 2 bars after any non-SL exit, 60 bars after consecutive SLs for ISO shorts. But the optimal cooldown length depends on the market mode — ISO shorts in a high-breadth (oversold) market need different cooldown periods than LONG entries in a low-breadth market. The current fixed cooldowns don't adapt to the regime environment.

**Cooldown by Market Mode:**
- Different cooldown periods for different market modes:
  - LONG mode: shorter cooldown after exit (market is healthy, opportunities come back quickly)
  - ISO_SHORT mode: longer cooldown (oversold conditions are episodic, not frequent)
  - SHORT mode: medium cooldown (trending conditions take time to resolve)
- Different cooldowns for different exit reasons:
  - Exit by Z0 or SMA (good exit): shorter cooldown
  - Exit by SL (bad exit): longer cooldown
  - Exit by MAX_HOLD (timeout): medium cooldown
- Example configs:
  - LONG mode, good exit: 2 bars
  - LONG mode, bad exit: 6 bars
  - ISO_SHORT mode, good exit: 6 bars
  - ISO_SHORT mode, bad exit: 20 bars
  - SHORT mode: 4 bars

**Why this is not a duplicate:**
- RUN39 (asymmetric win/loss cooldown) differentiates by WIN vs LOSS, not by market mode
- RUN34 (ISO cooldown escalation) adds 60-bar escalation only after consecutive SLs — this makes ALL cooldowns mode-dependent
- No prior RUN has implemented market-mode-dependent cooldown periods for all trade types

**Mechanistic rationale:** ISO shorts in a deeply oversold market (breadth ≥ 40%) are high-conviction trades that come around infrequently. A short position in that environment may take time to establish a new opportunity. LONG entries in a healthy market (breadth ≤ 15%) are more frequent — a rejection can quickly set up another opportunity. Mode-adaptive cooldowns prevent both over-trading in low-opportunity environments and under-trading in high-opportunity ones.

---

## Proposed Config Changes

```rust
// RUN83: Cooldown by Market Mode
pub const COOLDOWN_BY_MODE_ENABLE: bool = true;

// Cooldown after good exit (Z0, SMA, BE)
pub const COOLDOWN_LONG_GOOD: u32 = 2;        // bars
pub const COOLDOWN_ISO_GOOD: u32 = 6;          // bars
pub const COOLDOWN_SHORT_GOOD: u32 = 4;        // bars

// Cooldown after bad exit (SL, MAX_HOLD)
pub const COOLDOWN_LONG_BAD: u32 = 6;          // bars
pub const COOLDOWN_ISO_BAD: u32 = 20;         // bars
pub const COOLDOWN_SHORT_BAD: u32 = 10;        // bars

// Consecutive SL escalation (keeps RUN34 behavior but per-mode)
pub const CONSEC_SL_THRESHOLD: u32 = 2;        // bars of consecutive SL before escalation
pub const COOLDOWN_ESCALATE_MULT: u32 = 3;     // multiply cooldown by this after consec SLs
```

**`engine.rs` — compute_cooldown in close_position:**
```rust
/// Compute effective cooldown based on market mode and exit reason.
fn compute_cooldown(state: &SharedState, reason: &str, trade_type: TradeType) -> u32 {
    if !config::COOLDOWN_BY_MODE_ENABLE {
        // Fall back to RUN34 behavior
        if reason == "SL" && trade_type == TradeType::Regime {
            return config::ISO_SL_ESCALATE_COOLDOWN;
        }
        return 2;
    }

    let mode = state.market_mode;
    let is_bad_exit = reason == "SL" || reason == "MAX_HOLD";

    let base = match (mode, is_bad_exit) {
        // LONG mode
        (MarketMode::Long, false) => config::COOLDOWN_LONG_GOOD,
        (MarketMode::Long, true) => config::COOLDOWN_LONG_BAD,
        // ISO_SHORT mode
        (MarketMode::IsoShort, false) => config::COOLDOWN_ISO_GOOD,
        (MarketMode::IsoShort, true) => config::COOLDOWN_ISO_BAD,
        // SHORT mode
        (MarketMode::Short, false) => config::COOLDOWN_SHORT_GOOD,
        (MarketMode::Short, true) => config::COOLDOWN_SHORT_BAD,
    };

    // Escalation for consecutive SLs (per coin, tracked in cs.consecutive_sl)
    let cs = /* current coin state */;
    if reason == "SL" && cs.consecutive_sl >= config::CONSEC_SL_THRESHOLD {
        return base * config::COOLDOWN_ESCALATE_MULT;
    }

    base
}

// In close_position — replace fixed cooldown assignments:
let new_cooldown = compute_cooldown(state, reason, trade_type);
cs.cooldown = new_cooldown;
```

---

## Validation Method

### RUN83.1 — Cooldown by Mode Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed 2-bar cooldown (60-bar for ISO consecutive SL)

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `COOLDOWN_LONG_GOOD` | [1, 2, 3] |
| `COOLDOWN_LONG_BAD` | [4, 6, 8] |
| `COOLDOWN_ISO_GOOD` | [4, 6, 8] |
| `COOLDOWN_ISO_BAD` | [15, 20, 30] |
| `COOLDOWN_ESCALATE_MULT` | [2, 3, 5] |

**Per coin:** 3 × 3 × 3 × 3 × 3 = 243 configs × 18 coins = 4,374 backtests (use Rayon for parallelization)

**Key metrics:**
- `avg_cooldown_used`: average cooldown by market mode and exit reason
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `total_PnL_delta`: P&L change vs baseline
- `entry_rate_delta`: change in entry frequency

### RUN83.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best cooldown config per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- No mode shows >30% reduction in entry rate (avoid over-cooldown)
- Max drawdown does not increase >15% vs baseline

### RUN83.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed 2-bar) | Cooldown by Mode | Delta |
|--------|--------------------------|-----------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Avg Cooldown (LONG) | 2 | X | +N |
| Avg Cooldown (ISO_SHORT) | 2 | X | +N |
| Entry Rate (LONG) | X | X | -Y% |
| Entry Rate (ISO_SHORT) | X | X | -Y% |

---

## Why This Could Fail

1. **Cooldown is a blunt instrument:** Adjusting cooldowns by mode may not move the needle if the underlying opportunity frequency is already determined by the entry signals. The cooldown may be redundant with the entry signal frequency.
2. **ISO shorts are infrequent anyway:** ISO shorts fire on breadth ≥ 20% — during high-breadth periods, opportunities are already infrequent. A longer ISO cooldown may be redundant in a low-opportunity environment.
3. **Grid is large (243 configs):** The combinatorial space is large. With only 3 walk-forward windows, the best-in-sample config may overfit.

---

## Why It Could Succeed

1. **ISO shorts need more time:** ISO shorts fire when breadth is elevated (oversold). The market doesn't become oversold every day — it takes time for the conditions to rebuild. A longer ISO cooldown prevents over-trading in a low-opportunity environment.
2. **LONG mode is high-frequency:** In a low-breadth (healthy) market, coins oscillate around their means frequently. Short cooldowns allow capturing more of these oscillations.
3. **Aligns with market structure:** Different market modes have different characteristic timescales. LONG opportunities are higher-frequency than ISO_SHORT opportunities. Cooldown should reflect this.
4. **Simple and interpretable:** Just a lookup table by (mode, exit_reason). No new indicators.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN83 Cooldown by Mode |
|--|--|--|
| Cooldown after LONG good exit | 2 bars | 1-3 bars (configurable) |
| Cooldown after ISO_SHORT good exit | 2 bars | 4-8 bars (longer) |
| Cooldown after LONG bad exit | 2 bars | 4-8 bars (longer) |
| Cooldown after ISO_SHORT bad exit | 60 bars (consecutive SL only) | 15-30 bars (all bad exits) |
| Mode adaptation | None | Explicit per-mode cooldowns |
| Escalation | Only for ISO SL streaks | Per-mode escalation multiplier |
