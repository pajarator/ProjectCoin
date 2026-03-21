# RUN84 — Session-Based Partial Exit Scaling: Time-of-Day Dependent Take-Profit Levels

## Hypothesis

**Named:** `session_partial_scaling`

**Mechanism:** COINCLAW uses tiered partial exits (RUN53) with fixed PnL thresholds regardless of time of day. But crypto markets have distinct sessions: the UTC morning (00:00-08:00) is typically lower volatility with range-bound mean reversion; the UTC afternoon/evening (08:00-16:00) sees higher volume and larger directional moves; the UTC night (16:00-24:00) overlaps with US market hours and can be more volatile. Partial exit levels should adapt to these session characteristics.

**Session-Based Partial Exit Scaling:**
- Divide UTC day into sessions:
  - **Asia session** (UTC 00:00-08:00): Low volatility, tighter ranges → scale partial exit thresholds down by `ASIA_TIER_MULT` (e.g., 0.7×)
  - **Europe/US session** (UTC 08:00-16:00): Higher volatility, larger swings → keep default or scale up slightly by `EUUS_TIER_MULT` (e.g., 1.0-1.2×)
  - **US/Late session** (UTC 16:00-24:00): Mixed, overlapping with US equity market → scale by `US_TIER_MULT` (e.g., 0.9×)
- Apply multiplier to the two partial exit thresholds from RUN53
- Exit reason becomes `PARTIAL_T2` or `PARTIAL_T3` (same as RUN53 but session-scaled)

**Why this is not a duplicate:**
- RUN41 (session filter) used session gates to completely block entries during certain sessions — this scales EXIT THRESHOLDS, not entries
- RUN53 (tiered partial exits) established the partial exit framework with fixed thresholds — this makes thresholds session-dependent
- No prior RUN has adapted partial exit levels to intraday session characteristics

**Mechanistic rationale:** In Asia session, price oscillates in tighter ranges. A partial exit at 0.4% in Asia may be optimal — the move may not extend further. In US session, with higher volume and larger moves, holding longer for 0.8% or more may be better. Session-adaptive scaling aligns partial exits with the characteristic move size of each session.

---

## Proposed Config Changes

```rust
// RUN84: Session-Based Partial Exit Scaling
pub const SESSION_PARTIAL_ENABLE: bool = true;

// Session definitions (UTC hours)
pub const SESSION_ASIA_START: u8 = 0;   // UTC 00:00
pub const SESSION_ASIA_END: u8 = 8;      // UTC 08:00
pub const SESSION_EUUS_START: u8 = 8;    // UTC 08:00
pub const SESSION_EUUS_END: u8 = 16;     // UTC 16:00
pub const SESSION_US_START: u8 = 16;     // UTC 16:00
pub const SESSION_US_END: u8 = 24;       // UTC 24:00 (midnight)

// Session multipliers for partial exit tiers
pub const SESSION_ASIA_TIER_MULT: f64 = 0.70;   // scale down tiers in Asia session
pub const SESSION_EUUS_TIER_MULT: f64 = 1.00;   // default in Europe/US session
pub const SESSION_US_TIER_MULT: f64 = 0.85;      // slightly down in late US session
```

**`engine.rs` — get_session_multiplier:**
```rust
/// Determine current UTC session multiplier for partial exit thresholds.
fn get_session_multiplier() -> f64 {
    use chrono::Timelike;
    let utc_hour = chrono::Utc::now().hour();

    if utc_hour >= config::SESSION_ASIA_START && utc_hour < config::SESSION_ASIA_END {
        config::SESSION_ASIA_TIER_MULT
    } else if utc_hour >= config::SESSION_EUUS_START && utc_hour < config::SESSION_EUUS_END {
        config::SESSION_EUUS_TIER_MULT
    } else {
        config::SESSION_US_TIER_MULT
    }
}

/// Compute session-adjusted partial exit thresholds.
fn session_adjusted_tiers() -> (f64, f64) {
    let mult = get_session_multiplier();
    let tier1 = config::PARTIAL_TIER1_PNL * mult;
    let tier2 = config::PARTIAL_TIER2_PNL * mult;
    (tier1, tier2)
}

// In check_exit — partial exit logic uses session_adjusted_tiers():
let (tier1_thresh, tier2_thresh) = session_adjusted_tiers();
```

---

## Validation Method

### RUN84.1 — Session Partial Scaling Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 + RUN53 partial exits (fixed tier thresholds)

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `SESSION_ASIA_TIER_MULT` | [0.60, 0.70, 0.80] |
| `SESSION_US_TIER_MULT` | [0.80, 0.85, 0.90] |
| `SESSION_EUUS_TIER_MULT` | [0.90, 1.00, 1.10] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `avg_tier_at_partial`: average PnL threshold at partial exit by session
- `partial_exit_rate_delta`: change in partial exit rate by session
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_exit_pnl_delta`: change in average exit PnL (should increase if scaling is working)

### RUN84.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best session multipliers per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Partial exit rate does not drop below 50% of baseline (don't over-scale and eliminate exits)
- Asia session avg exit PnL improves vs baseline

### RUN84.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (RUN53 fixed tiers) | Session-Adaptive Tiers | Delta |
|--------|------------------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Asia Session Avg Exit | $X | $X | +$X |
| EUUS Session Avg Exit | $X | $X | +$X |
| US Session Avg Exit | $X | $X | +$X |
| Partial Exit Rate | X% | X% | +/-Y% |

---

## Why This Could Fail

1. **Crypto has no closing bell:** Unlike equities, crypto trades 24/7. The "session" effect may be much weaker because there are no market-open/close dynamics. The Asia session is not a low-volume period for crypto.
2. **Fixed partial exits are already optimized:** RUN53 already optimized the partial exit thresholds. Session-specific multipliers on top of optimized values may not improve further.
3. **Overfitting risk:** Session definitions are somewhat arbitrary. Asia session boundaries (UTC 0-8) may not align with actual crypto liquidity patterns. Different exchange user bases may shift the actual session effects.

---

## Why It Could Succeed

1. **US equity market hours affect crypto:** When US equity markets open (UTC 13:30-14:00), there's often increased volatility and directional flow in crypto. Partial exits during this overlap may need to be wider to capture larger moves.
2. **Asia session is genuinely lower volatility:** Historical data shows that UTC 00:00-08:00 has lower average True Range than other sessions. Scaling partial exits down in this session prevents leaving profits on the table in a low-move environment.
3. **Simple and additive:** Session multipliers are easy to compute and add to the existing partial exit framework from RUN53.
4. **Backtestable with existing data:** UTC timestamps are preserved in OHLCV data — no new data sources needed.

---

## Comparison to Baseline

| | Current COINCLAW v16 (RUN53 fixed tiers) | RUN84 Session-Adaptive Tiers |
|--|--|--|
| Asia session tier 1 | 0.4% | 0.28% (0.7×) |
| Asia session tier 2 | 0.8% | 0.56% (0.7×) |
| EUUS session tier 1 | 0.4% | 0.40% (1.0×) |
| EUUS session tier 2 | 0.8% | 0.80% (1.0×) |
| US session tier 1 | 0.4% | 0.34% (0.85×) |
| US session tier 2 | 0.8% | 0.68% (0.85×) |
| Session awareness | None | UTC hour-based |
| Implementation | RUN53 partial tiers | Multipliers on top of RUN53 |
