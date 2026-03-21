# RUN143 — Coin-Specific Energy Signal: Idiosyncratic Momentum Fade

## Hypothesis

**Mechanism**: When a coin moves significantly (Z-score spike) while BTC is flat (low BTC Z-score), the move is coin-specific — driven by news, accumulation/distribution, or a localized event. These idiosyncratic moves tend to mean-revert faster than systemic moves, because they lack broader market support. COINCLAW currently has no cross-coin momentum filter for this pattern — it only uses BTC for breadth calculation, not as a reference for coin-specific energy.

**Why this is not a duplicate**: RUN78 (Cross-Coin Z-Score Confirmation) uses multiple coins' Z-scores to confirm a trade. RUN49/86 use correlation to avoid over-concentration. RUN63 uses BTC trend direction. None use BTC-flat-as-negative-filter — i.e., "this coin moved but BTC didn't, so it's coin-specific and likely to revert."

**Why it could work**: In crypto, coin-specific events (partnerships, listings, protocol upgrades) create isolated price moves that lack staying power. If BTC Z-score is near zero during a coin's Z-score spike, the coin's move is noise-like and reversible. If BTC is also moving in the same direction, the move is systemic and may continue. Distinguishing these two cases is novel and mechanically sound.

---

## Proposed Config Changes (config.rs)

```rust
// ── RUN3: Coin-Specific Energy Signal ────────────────────────────────
// Fires when a coin's z-score spikes AND BTC is relatively flat.
// Mechanism: idiosyncratic move = coin-specific event = mean-reverts fast.
// Exits: Z-score reversion OR MAX_HOLD bars

pub const COIN_SPECIFIC_ENABLED: bool = true;
pub const COIN_SPECIFIC_Z_THRESHOLD: f64 = 2.0;    // coin z must exceed this
pub const COIN_SPECIFIC_BTC_Z_MAX: f64 = 0.5;       // BTC z must be below this (flat)
pub const COIN_SPECIFIC_SL: f64 = 0.004;            // 0.4% stop
pub const COIN_SPECIFIC_TP: f64 = 0.003;            // 0.3% take profit
pub const COIN_SPECIFIC_MAX_HOLD: u32 = 16;         // ~4 hours at 15m bars
pub const COIN_SPECIFIC_RET16_MIN: f64 = 0.005;     // coin must have 0.5%+ 16-bar return
```

Note: BTC Z-score must be computed from the BTC/USDT `Ind15m` (already tracked in COINCLAW's market context via `MarketCtx::btc_z`). This requires passing BTC's indicator state to `check_entry`.

Add to `MarketCtx` in `coordinator.rs`:
```rust
pub btc_z: f64,           // BTC 15m z-score
pub btc_z_valid: bool,    // BTC indicators are valid
```

Add new trade type:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeType {
    Regime,
    Scalp,
    Momentum,
    CoinSpecific,  // NEW
}
```

Add entry logic in engine.rs:
```rust
/// Fires when: coin.z > Z_THRESHOLD AND btc_z < BTC_Z_MAX AND coin.ret16 > RET16_MIN
/// Direction: SHORT when coin.z > Z_THRESHOLD (coin-specific rally)
/// Direction: LONG when coin.z < -Z_THRESHOLD AND btc_z > -BTC_Z_MAX
fn check_coin_specific_entry(state: &mut SharedState, ci: usize, btc_z: f64) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid || ind.z.is_nan() { return None; }
    if ind.ret16.is_nan() || ind.ret16.abs() < COIN_SPECIFIC_RET16_MIN { return None; }

    // BTC must be flat for coin-specific signal to fire
    if btc_z.abs() > COIN_SPECIFIC_BTC_Z_MAX { return None; }

    // SHORT: coin rallied (z > 0), BTC didn't
    if ind.z > COIN_SPECIFIC_Z_THRESHOLD {
        return Some((Direction::Short, "coin_specific"));
    }
    // LONG: coin sold off (z < 0), BTC didn't
    if ind.z < -COIN_SPECIFIC_Z_THRESHOLD {
        return Some((Direction::Long, "coin_specific"));
    }
    None
}
```

Integration: Call `check_coin_specific_entry` from `check_entry` after regime entries are evaluated. If no position opened and BTC z is available, attempt coin-specific entry.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (no coin-specific layer)
- **Comparison**: coin-specific trades tracked separately

**Metrics to measure**:
- Coin-specific WR (hypothesis: >55% — idiosyncratic moves revert faster)
- PF on coin-specific trades
- Correlation with regime and momentum trades (should be low — orthogonal)
- BTC-flat cases vs BTC-trending cases: do they actually differ in outcomes?

**Hypothesis**: When BTC is flat (|btc_z| < 0.5) and a coin moves >2σ (|z| > 2.0), the coin-specific move reverts with WR >55%. This is a stronger signal than same-coin Z-score alone because it filters out systemic moves.

---

## Validation Method

1. **Historical backtest** (run3_1_cs_backtest.py):
   - 18 coins, 1-year 15m data
   - Identify all cases where |coin_z| > 2.0 AND |btc_z| < 0.5
   - Simulate fade-the-move entry
   - Record: z magnitude, BTC z value, ret16, entry price, stop, TP, exit reason, P&L
   - Output: per-coin WR, PF, BTC-flat vs BTC-moving comparison

2. **Walk-forward** (run3_2_cs_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep COIN_SPECIFIC_Z_THRESHOLD: 1.5 / 2.0 / 2.5
   - Sweep COIN_SPECIFIC_BTC_Z_MAX: 0.3 / 0.5 / 0.7

3. **Combined comparison** (run3_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + coin_specific
   - Portfolio stats, trade type correlation matrix, per-coin contribution

---

## Out-of-Sample Testing

- Z_THRESHOLD sweep: 1.5 / 2.0 / 2.5 / 3.0
- BTC_Z_MAX sweep: 0.3 / 0.5 / 0.7
- OOS: final 4 months held out from all parameter selection
- Key test: Does BTC-flat filter actually improve WR vs no-BTC-filter on coin-specific moves?
