# RUN150 — BTC Sentiment Extreme Reversal: Counter-Cycle Rotation on Market Exhaustion

## Hypothesis

**Mechanism**: When BTC's 15m RSI reaches extreme levels (>80 or <20), it signals market sentiment exhaustion at the sector level. BTC has likely peaked/pooled and is due for a reversal. Alts tend to move inversely at these turning points: BTC RSI >80 (greed) → BTC overheated → alts的机会来了 → alts bounce; BTC RSI <20 (fear) → BTC oversold → alts dump before BTC recovers. This is a **counter-rotational** signal — not a filter, but a trigger for directionally opposite trades in alts.

**Why this is not a duplicate**: RUN21 (Sentiment Regime Filter) used BTC RSI as a **blocking filter** (block longs when fear, block shorts when greed) and found it reduced WR to 30.4%. This RUN uses BTC RSI extremes as a **trade trigger** for counter-directional alt trades. Blocking vs triggering are opposite mechanisms. The insight is different: when BTC reaches extreme RSI, it's a warning that BTC's move is exhausting AND that alts are likely to move the opposite way.

**Why it could work**: BTC's dominance creates a hub-and-spoke dynamic. When BTC is greedily extended, capital has been rotated into BTC and alts are neglected — creating pent-up demand for alts. When BTC is fearfully depressed, alts have sold off harder (liquidations) and bounce first. This is the "altcoin rotation" trade that traders talk about but COINCLAW doesn't capture. If BTC RSI >80 and alt RSI <40, that alt is a LONG candidate; the inverse for shorts.

---

## Proposed Config Changes (config.rs)

```rust
// ── RUN10: BTC Sentiment Extreme Counter-Rotation ──────────────────────
// When BTC RSI exceeds threshold (GREED > 80 or FEAR < 20), it signals exhaustion
// Alts with opposing RSI position (alt RSI < 30 when BTC GREED, alt RSI > 70 when BTC FEAR) are entries
// Direction: opposite to BTC's direction at the extreme
// Exit: RSI normalization OR MAX_HOLD bars

pub const SENTIMENT_ENABLED: bool = true;
pub const SENTIMENT_BTC_RSI_GREED: f64 = 80.0;   // BTC RSI above this = greed exhaustion
pub const SENTIMENT_BTC_RSI_FEAR: f64 = 20.0;    // BTC RSI below this = fear exhaustion
pub const SENTIMENT_ALT_RSI_LONG: f64 = 35.0;     // alt RSI below this when BTC GREED = LONG
pub const SENTIMENT_ALT_RSI_SHORT: f64 = 65.0;    // alt RSI above this when BTC FEAR = SHORT
pub const SENTIMENT_SL: f64 = 0.004;             // 0.4% stop
pub const SENTIMENT_TP: f64 = 0.003;             // 0.3% take profit
pub const SENTIMENT_MAX_HOLD: u32 = 16;           // ~4 hours at 15m bars
pub const SENTIMENT_MIN_BTC_RSI_AGE: u32 = 4;     // BTC RSI extreme must persist for 4 bars
```

Note: BTC RSI is available from the BTC coin's `ind_15m.rsi` (coin index 17).

Add to `MarketCtx` in `coordinator.rs`:
```rust
pub btc_rsi: f64,
pub btc_rsi_prev: f64,
pub btc_rsi_greed_bars: u32,   // consecutive bars with BTC RSI > SENTIMENT_BTC_RSI_GREED
pub btc_rsi_fear_bars: u32,    // consecutive bars with BTC RSI < SENTIMENT_BTC_RSI_FEAR
```

Add sentiment tracking in `coordinator.rs`:
```rust
/// Update BTC RSI extreme counter
fn update_btc_sentiment(ctx: &mut MarketCtx, btc_rsi: f64) {
    if btc_rsi > config::SENTIMENT_BTC_RSI_GREED {
        ctx.btc_rsi_greed_bars += 1;
        ctx.btc_rsi_fear_bars = 0;
    } else if btc_rsi < config::SENTIMENT_BTC_RSI_FEAR {
        ctx.btc_rsi_fear_bars += 1;
        ctx.btc_rsi_greed_bars = 0;
    } else {
        ctx.btc_rsi_greed_bars = 0;
        ctx.btc_rsi_fear_bars = 0;
    }
    ctx.btc_rsi = btc_rsi;
}
```

Add entry logic in engine.rs:
```rust
/// Fires when BTC RSI extreme has persisted for MIN_BTC_RSI_AGE bars
/// LONG: BTC RSI > GREED threshold AND alt RSI < ALT_RSI_LONG (alt oversold vs BTC)
/// SHORT: BTC RSI < FEAR threshold AND alt RSI > ALT_RSI_SHORT (alt overbought vs BTC)
fn check_sentiment_entry(state: &mut SharedState, ci: usize, ctx: &MarketCtx) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::SENTIMENT_ENABLED { return None; }

    // Must have fresh BTC RSI extreme
    if ctx.btc_rsi_greed_bars < config::SENTIMENT_MIN_BTC_RSI_AGE
        && ctx.btc_rsi_fear_bars < config::SENTIMENT_MIN_BTC_RSI_AGE
    {
        return None;
    }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }
    if ind.rsi.is_nan() { return None; }

    // BTC GREED + alt RSI depressed = LONG
    if ctx.btc_rsi_greed_bars >= config::SENTIMENT_MIN_BTC_RSI_AGE
        && ind.rsi < config::SENTIMENT_ALT_RSI_LONG
    {
        return Some((Direction::Long, "sentiment_rev"));
    }

    // BTC FEAR + alt RSI elevated = SHORT
    if ctx.btc_rsi_fear_bars >= config::SENTIMENT_MIN_BTC_RSI_AGE
        && ind.rsi > config::SENTIMENT_ALT_RSI_SHORT
    {
        return Some((Direction::Short, "sentiment_rev"));
    }

    None
}
```

Integration: Call from `check_entry` — fires independently of regime mode.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (no sentiment counter-rotation strategy)
- **Comparison**: sentiment trades tracked separately

**Metrics to measure**:
- Sentiment counter-rotation WR (hypothesis: >55%)
- PF on sentiment trades
- BTC RSI extreme frequency (how often does this fire?)
- Comparison: does the counter-rotational approach (this RUN) beat the blocking approach (RUN21)?

**Hypothesis**: Counter-rotational entries (triggered by BTC extremes) should achieve WR >55% because BTC's extreme moves precede sector rotations. The key difference from RUN21: we're not blocking trades, we're triggering counter-directional ones. This avoids the "no regime clears breakeven" problem by targeting a different signal type entirely.

---

## Validation Method

1. **Historical backtest** (run10_1_sentiment_backtest.py):
   - 18 coins, 1-year 15m data
   - Identify all BTC RSI extreme events (>80 or <20) persisting ≥4 bars
   - For each, identify qualifying alts (opposing RSI position)
   - Record: BTC RSI value, alt RSI value, direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time, BTC RSI extreme type (greed/fear) vs outcome

2. **Walk-forward** (run10_2_sentiment_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep SENTIMENT_BTC_RSI_GREED: 75 / 80 / 85
   - Sweep SENTIMENT_BTC_RSI_FEAR: 15 / 20 / 25
   - Sweep SENTIMENT_ALT_RSI_LONG: 30 / 35 / 40
   - Sweep SENTIMENT_ALT_RSI_SHORT: 60 / 65 / 70

3. **Combined comparison** (run10_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + sentiment_counter_rotation
   - Portfolio stats, sentiment trade contribution, comparison with RUN21 blocking approach

---

## Out-of-Sample Testing

- GREED threshold sweep: 75 / 80 / 85
- FEAR threshold sweep: 15 / 20 / 25
- MIN_AGE sweep: 2 / 4 / 6 bars
- OOS: final 4 months held out from all parameter selection
- Key OOS test: does this improve WR vs RUN21's blocking approach in the same OOS period?
