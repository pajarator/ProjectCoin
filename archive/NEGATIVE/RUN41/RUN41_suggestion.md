# RUN41 — Session-Based Trade Filter: Asia/Europe/US Session Conditional Engagement

## Hypothesis

**Named:** `session_conditional_trading`

**Mechanism:** Cryptocurrency markets exhibit distinct behavioral patterns across the 24-hour cycle:
- **US session (14:00–23:00 UTC):** Highest volume, most directional momentum, trend-following works better
- **Europe session (07:00–16:00 UTC):** Moderate volume, mixed conditions
- **Asia session (00:00–09:00 UTC):** Lowest volume, most ranging, mean reversion should work better

COINCLAW's strategies (vwap_rev, bb_bounce, adr_rev, etc.) are all mean-reversion variants. If mean reversion works better during Asia session (low volume, ranging conditions), and worse during US session (momentum-driven), then selectively suppressing trades during US session and/or selectively engaging during Asia session should raise win rate.

Additionally, scalp trades may perform differently by session: scalping requires volatility and liquidity, which are highest during US hours. Asia session scalp entries may be mostly noise.

**Why this is not a duplicate:**
- No prior RUN tested time-of-day conditional performance
- RUN21 tested BTC RSI sentiment — a different signal measured at a different timescale
- RUN12 tested 3-mode market regime — Long/Short/ISO_SHORT, not intraday session
- The `hour_of_day` feature in the feature matrix (RUN15) has never been tested as a trade filter

---

## Session Definitions

Using UTC-based session windows:

| Session | UTC Hours | Typical Character |
|---------|-----------|------------------|
| Asia | 00:00–08:59 | Low volume, ranging, mean-reversion favorable |
| Europe | 09:00–16:59 | Moderate volume, mixed |
| US | 17:00–23:59 | High volume, directional momentum |

**Note:** UTC alignment is approximate. Crypto is 24/7 but US institutional hours (when large moves occur) overlap with the 14:00–23:00 UTC window.

---

## Proposed Config Changes

```rust
// RUN41: Session-based trade filter
// When a session filter is enabled, trades are only taken during those sessions
// SESSION_FILTER_MODE: 0=disabled, 1=Asia only, 2=Europe only, 3=US only, 4=Asia+Europe
pub const SESSION_FILTER_MODE: u8 = 0;
pub const SESSION_ENABLE_ASIA: bool = true;
pub const SESSION_ENABLE_EUROPE: bool = true;
pub const SESSION_ENABLE_US: bool = false;  // disable US for mean-reversion strategies
```

For **scalp layer specifically:**
```rust
// Scalp trades need volatility — only enable during high-volatility sessions
pub const SCALP_SESSION_FILTER_MODE: u8 = 0;  // 0=disabled, 1=US only, 2=non-Asia
pub const SCALP_SESSION_US_ONLY: bool = false;
```

**`strategies.rs` change — add session gate to entry functions:**
```rust
pub fn is_in_allowed_session(hour_utc: u32) -> bool {
    match config::SESSION_FILTER_MODE {
        0 => true,  // disabled
        1 => hour_utc < 9,   // Asia only
        2 => hour_utc >= 9 && hour_utc < 17,  // Europe only
        3 => hour_utc >= 17,  // US only
        4 => hour_utc < 17,  // Asia + Europe
        _ => true,
    }
}
```

The session gate is applied in `check_entry` and `check_scalp_entry` in `engine.rs` before calling the entry functions. The hour is extracted from the current candle's timestamp.

---

## Validation Method

### RUN41.1 — Session Performance Profiling (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset. Extract hour from each candle timestamp.

**Step 1 — Profile current COINCLAW strategies by session:**

For each coin:
1. Run COINCLAW v13 strategy (regime + scalp) on full year
2. Tag each trade with the session (Asia/Europe/US) of its entry bar
3. Compute per-session: WR%, PF, total_PnL, trade_count, avg_win%, avg_loss%

Expected pattern:
- Asia session: highest WR% (ranging = mean-reversion friendly), lowest trade count
- US session: lowest WR% (directional = mean-reversion hostile), highest trade count

**Step 2 — Grid search:**

| Parameter | Values |
|-----------|--------|
| `SESSION_FILTER_MODE` | [0=disabled, 1=Asia, 2=Europe, 3=US, 4=Asia+Europe] |
| `SCALP_SESSION_FILTER_MODE` | [0=disabled, 1=US_only, 2=non-Asia] |

**Per coin:** 5 × 3 = 15 configs × 18 coins = 270 backtests

**Score:** `WR_delta × sqrt(trades) / max(max_dd, 1)` (rewards high WR% delta with sufficient trades)

**Also measure:**
- `session concentration`: % of profitable trades that come from a specific session
- `session P&L share`: what % of total P&L comes from each session

### RUN41.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: measure per-session performance, find best session filter config per coin
2. Test: evaluate with those params on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS WR% delta vs baseline during filtered sessions
- Portfolio OOS P&L ≥ baseline despite fewer trades (quality over quantity)
- At least one session is clearly dominant (WR% gap > 5pp between sessions)

### RUN41.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | Session-Filtered | Delta |
|--------|---------------|-----------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K (−X%) |
| Asia WR% | X% | X% | +Ypp |
| Europe WR% | X% | X% | +Ypp |
| US WR% | X% | X% | +Ypp |
| Asia P&L | $X | $X | +$X |
| US P&L | $X | $X | +$X |

---

## Why This Could Fail

1. **Session划分 is arbitrary:** UTC session boundaries don't match actual crypto market dynamics. US institutions may dominate at different hours than expected.
2. **Crypto is 24/7:** Unlike equities, there's no true "open/close" to define sessions. The session effect may be too diffuse to capture.
3. **Reducing trades hurts more than quality helps:** If session filtering cuts trade count by 50% but WR% only rises 3pp, the net P&L may be worse due to opportunity cost.

---

## Why It Could Succeed

1. **Simple, well-motivated hypothesis:** US hours have the most institutional volume and directional flow — the opposite of what mean reversion needs. Filtering out US sessions should naturally select higher-quality reversion setups.
2. **No external data needed:** Session is derived from timestamp — available in any OHLCV dataset.
3. **Applicable to both regime and scalp layers:** Scalp specifically needs liquidity and volatility, which are highest during US hours. Disabling scalp during Asia and enabling during US could be the optimal direction (opposite of regime trades).

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN41 Session Filter |
|--|--|--|
| Entry filter | None (market mode only) | Session gate |
| Session awareness | None | Asia/Europe/US conditional |
| Trade timing | All hours | Filtered hours only |
| Expected WR% delta | — | +3–8pp during Asia vs US |
| Expected trade reduction | — | −20–40% |
| Regime trades | Unfiltered | Asia/Europe preferred |
| Scalp trades | Unfiltered | US-only preferred |
