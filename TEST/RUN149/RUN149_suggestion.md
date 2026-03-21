# RUN149 — BTC Trend Rotation Signal: Catch-Up Mean Reversion in Altcoins

## Hypothesis

**Mechanism**: BTC sets the market regime for altcoins. When BTC's 4h SMA crosses (bullish or bearish), it signals a directional rotation. Altcoins that have not yet responded to the BTC signal are "lagging" — they have pent-up directional movement in the same direction as BTC. These lagging alts are high-probability mean-reversion entries: they revert from their depressed/elevated price toward fair value as the BTC signal propagates.

**Why this is not a duplicate**: RUN63 (BTC Trend Confirmation for Regime Entries) uses BTC SMA9 < SMA20 as a BLOCK filter. This RUN is the inverse: it uses BTC's 4h SMA crossover as an ENTRY trigger for alts that haven't moved yet. No prior RUN uses BTC as a timing signal for alt entries rather than as a gate. RUN40 (BTC Dominance Scalp Filter) is about scalp direction, not sector rotation.

**Why it could work**: In crypto, BTC's dominance means its trend changes precede alt moves by 2-8 hours. When BTC turns bullish, alts that are still below their own SMAs have "catching up" to do. The trade is a mean-reversion of the lag — not BTC-following, but BTC-timing-adjusted mean-reversion.

---

## Proposed Config Changes (config.rs)

```rust
// ── RUN9: BTC Trend Rotation Signal ──────────────────────────────────
// When BTC's 4h SMA crosses (SMA9 crosses SMA20), it signals rotation
// Altcoins below their own SMA at the time of BTC's cross are "lagging"
// LONG laggards: BTC turned bullish (SMA9 > SMA20) AND alt below its SMA20
// SHORT laggards: BTC turned bearish (SMA9 < SMA20) AND alt above its SMA20
// Exit: alt crosses its own SMA20 (lag caught up) OR MAX_HOLD bars

pub const BTC_ROTATION_ENABLED: bool = true;
pub const BTC_ROTATION_LOOKBACK: u32 = 4;    // look back 4 bars for BTC SMA cross
pub const BTC_ROTATION_MAX_AGE: u32 = 16;    // rotation signal valid for 16 bars (4h)
pub const BTC_ROTATION_SL: f64 = 0.004;      // 0.4% stop
pub const BTC_ROTATION_TP: f64 = 0.003;      // 0.3% take profit
pub const BTC_ROTATION_MAX_HOLD: u32 = 24;   // ~6 hours at 15m bars
pub const BTC_ROTATION_Z_FILTER: f64 = -1.0; // alt's z-score must confirm (< -1.0 for LONG)
```

Note: BTC's 4h SMA indicators need to be computed. BTC/USDT is already in the COINCLAW universe (coin index 17). The `MarketCtx` already tracks `btc_z` — need to also track `btc_sma9` and `btc_sma20`.

Add to `MarketCtx` in `coordinator.rs`:
```rust
pub btc_sma9: f64,
pub btc_sma20: f64,
pub btc_sma9_prev: f64,
pub btc_sma20_prev: f64,
pub btc_rotation_signal: Option<Direction>,  // None, or Some(Long/Short) if fresh cross
pub btc_rotation_age: u32,                  // bars since rotation signal fired
```

Add rotation detection in `coordinator.rs`:
```rust
/// Detect BTC 4h SMA crossover: returns Some(Long) if SMA9 crossed above SMA20,
/// Some(Short) if SMA9 crossed below SMA20, None if no fresh cross
fn detect_btc_rotation(ctx: &mut MarketCtx, btc_ind: &Ind15m) {
    let cross = match (&ctx.btc_sma9, &ctx.btc_sma20, &btc_ind.sma9, &btc_ind.sma20) {
        (Some(prev9), Some(prev20), cur9, cur20)
        if !prev9.is_nan() && !prev20.is_nan() && !cur9.is_nan() && !cur20.is_nan() => {
            if prev9 <= prev20 && cur9 > cur20 { Some(Direction::Long) }
            else if prev9 >= prev20 && cur9 < cur20 { Some(Direction::Short) }
            else { ctx.btc_rotation_signal }
        }
        _ => None,
    };
    ctx.btc_rotation_signal = cross;
    ctx.btc_rotation_age = cross.map(|_| 0).unwrap_or(ctx.btc_rotation_age + 1);
}
```

Add entry logic in engine.rs:
```rust
/// Fires when BTC has signaled a rotation AND this alt is lagging (direction matches but alt hasn't moved)
fn check_btc_rotation_entry(state: &mut SharedState, ci: usize, ctx: &MarketCtx) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::BTC_ROTATION_ENABLED { return None; }

    // Signal must be fresh (< BTC_ROTATION_MAX_AGE bars old)
    if ctx.btc_rotation_age > config::BTC_ROTATION_MAX_AGE { return None; }
    let rot_dir = ctx.btc_rotation_signal?;

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }

    // Alt must be lagging in the direction of the rotation
    // LONG rotation + alt below SMA20 = lagging bullish
    // SHORT rotation + alt above SMA20 = lagging bearish
    let is_lagging = match rot_dir {
        Direction::Long => ind.p < ind.sma20,
        Direction::Short => ind.p > ind.sma20,
    };
    if !is_lagging { return None; }

    // Z-score must confirm
    if ind.z < config::BTC_ROTATION_Z_FILTER { return None; }

    Some((rot_dir, "btc_rotation"))
}
```

Integration: Call from `check_entry` after regime and before momentum.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (no BTC rotation strategy)
- **Comparison**: BTC rotation trades tracked separately

**Metrics to measure**:
- BTC rotation WR (hypothesis: >55% — lag-catch-up is predictable)
- PF on BTC rotation trades
- Average lag time between BTC cross and alt response (should be 4-12 bars)
- Correlation with regime trades (should be moderate — BTC rotation triggers different entries)

**Hypothesis**: BTC rotation trades should achieve WR >55% because the lag between BTC's regime change and alt response is predictable. When BTC turns bullish and an alt is still below SMA20, the "catch-up" trade has a mechanical catalyst.

---

## Validation Method

1. **Historical backtest** (run9_1_btcrot_backtest.py):
   - 18 coins, 1-year 15m data
   - Identify all BTC 4h SMA crossovers
   - For each cross, identify lagging alts (alt's price vs its SMA20)
   - Record: rotation direction, lag magnitude (alt price vs SMA), entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg lag time

2. **Walk-forward** (run9_2_btcrot_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep BTC_ROTATION_MAX_AGE: 8 / 16 / 24 bars
   - Sweep BTC_ROTATION_Z_FILTER: -0.5 / -1.0 / -1.5

3. **Combined comparison** (run9_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + btc_rotation
   - Portfolio stats, rotation trade contribution, per-coin analysis

---

## Out-of-Sample Testing

- MAX_AGE sweep: 8 / 16 / 24 bars
- Z_FILTER sweep: -0.5 / -1.0 / -1.5
- OOS: final 4 months held out from all parameter selection
