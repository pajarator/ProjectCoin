# RUN63 — BTC Trend Confirmation for Regime LONG Entries

## Hypothesis

**Named:** `btc_trend_regime_filter`

**Mechanism:** When BTC is in a clear downtrend (or showing weakness), regime LONG entries on altcoins face structural headwind. Even if an altcoin's individual indicators show oversold, BTC's downward momentum can drag the altcoin lower before the altcoin's mean-reversion completes. Regime LONG entries should require BTC to not be in a confirmed downtrend.

**BTC trend signal:**
- `btc_sma9 < btc_sma20` → short-term BTC downtrend
- `btc_ret16 < -0.01` (16-bar return < -1%) → BTC has fallen meaningfully

**Filter logic:**
```
For regime LONG entry:
  if btc_sma9 < btc_sma20 AND btc_ret16 < -0.01:
    block LONG entry (BTC is in confirmed short-term downtrend)

// For SHORT entries: invert — require BTC NOT in uptrend
For regime SHORT entry:
  if btc_sma9 > btc_sma20 AND btc_ret16 > 0.01:
    block SHORT entry (BTC is in confirmed short-term uptrend)
```

**Why this is not a duplicate:**
- RUN40 tested BTC dominance for scalp trades — this tests BTC trend for regime trades
- No prior RUN used BTC's moving average relationship (SMA9 vs SMA20) as an entry filter
- No prior RUN used BTC's short-term return as a directional filter for regime entries
- This is BTC-timeline confirmation, distinct from cross-coin z-score spread (RUN40/55)

---

## Proposed Config Changes

```rust
// RUN63: BTC Trend Confirmation for Regime Entries
pub const BTC_TREND_ENABLE: bool = true;
pub const BTC_TREND_RET_THRESHOLD: f64 = -0.01;  // BTC 16-bar return must be > -1% for LONG
pub const BTC_TREND_SMA_CONFIRM: bool = true;    // also require SMA9 > SMA20 for LONG
```

**`coordinator.rs` — add BTC trend fields to MarketCtx:**
```rust
pub struct MarketCtx {
    pub avg_z: f64,
    pub avg_rsi: f64,
    pub btc_z: f64,
    pub avg_z_valid: bool,
    pub avg_rsi_valid: bool,
    pub btc_z_valid: bool,
    pub btc_sma9: f64,          // NEW
    pub btc_sma20: f64,         // NEW
    pub btc_ret16: f64,         // NEW: BTC 16-bar compounded return
    pub btc_in_downtrend: bool, // NEW: sma9 < sma20 AND ret16 < threshold
}
```

**`strategies.rs` — long_entry and short_entry use BTC trend:**
```rust
pub fn long_entry(ind: &Ind15m, strat: LongStrat, ctx: &MarketCtx) -> bool {
    // ... existing entry checks ...
    // BTC Trend gate
    if config::BTC_TREND_ENABLE {
        if ctx.btc_in_downtrend {
            return false;  // BTC in downtrend — don't LONG alts
        }
    }
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN63.1 — BTC Trend Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins + BTC, 1-year dataset

**Baseline:** Current COINCLAW v16 — no BTC trend filter on regime entries

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `BTC_TREND_RET_THRESHOLD` | [-0.005, -0.010, -0.015, -0.020] |
| `BTC_TREND_SMA_CONFIRM` | [true, false] |

**Per coin:** 4 × 2 = 8 configs × 18 coins = 144 backtests

**Key metrics:**
- `btc_filter_block_rate`: % of LONG entries blocked by BTC trend filter
- `WR_delta`: win rate change vs baseline for non-blocked entries
- `PF_delta`: profit factor change vs baseline
- `false_block_rate`: % of blocked entries that would have been winners
- `correct_block_rate`: % of blocked entries that would have been losers

### RUN63.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `RET_THRESHOLD × SMA_CONFIRM` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- False block rate < 25%
- Portfolio OOS P&L ≥ baseline

### RUN63.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | BTC-Trend Filtered | Delta |
|--------|---------------|-------------------|-------|
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Total P&L | $X | $X | +$X |
| Max DD | X% | X% | -Ypp |
| Trade Count | N | M | −K |
| BTC Block Rate | 0% | X% | — |
| False Block Rate | — | X% | — |
| BTC Downtrend Freq | — | X% of bars | — |

---

## Why This Could Fail

1. **BTC trend and altcoin mean-reversion are independent:** BTC going down doesn't prevent individual altcoins from mean-reverting. The filter blocks valid entries.
2. **BTC signal is lagging:** SMA9 < SMA20 is a slow signal — by the time it's confirmed, the BTC downtrend may already be reversing.
3. **No additional data needed:** BTC trend data is already available in the existing indicator computation

---

## Why It Could Succeed

1. **Structural headwind:** BTC downtrend creates selling pressure across the market. LONGing an altcoin during BTC downtrend is fighting the tide.
2. **Simple and fast:** SMA9 vs SMA20 on BTC is a quick check. No new data required.
3. **Proven in scalp:** RUN40 showed BTC dominance matters for scalp. This extends it to regime.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN63 BTC-Trend Filter |
|--|--|--|
| BTC check | None | SMA9/SMA20 + 16-bar ret |
| LONG filter | None | Blocked during BTC downtrend |
| SHORT filter | None | Blocked during BTC uptrend |
| Data required | BTC price | BTC price + SMA |
| Expected block rate | 0% | 15–25% |
| Expected WR% delta | — | +2–5pp |
