# RUN142 — ATR Spike Reversion: Fade the Overshoot

## Hypothesis

**Mechanism**: Large single-candle directional moves (driven by news, liquidations, or order flow imbalance) frequently overshoot fair value and reverse. When a 15m candle's body exceeds 2x the ATR(14), the move is "too fast" and likely to mean-revert within 4-8 bars. COINCLAW has no mechanism to detect or exploit these spike-and-revert patterns — they fall through the cracks between scalp (1m), regime (15m slow), and momentum (breakout continuation).

**Why this is not a duplicate**: No prior RUN addresses ATR-based spike detection. BB squeeze (RUN71) measures compression before the move, not the move itself. Scalp vol_spike_rev uses volume ratio, not ATR. Momentum (RUN27/28) fades breakouts. This is specifically targeting the *post-spike reversion* — a distinct mechanical pattern.

**Why it could work**: In crypto, liquidation cascades and news-driven candles are common. A candle that moves 2x+ ATR in one direction creates an overshoot. Mean reversion after such spikes is well-documented in forex and futures markets. If WR >55% on these trades, ATR spike reversion adds an orthogonal signal layer.

---

## Proposed Config Changes (config.rs)

```rust
// ── RUN2: ATR Spike Reversion ───────────────────────────────────────
// Spike: candle body >= ATR(14) * SPIKE_MULT
// Entry: after spike, fade the move (LONG after bearish spike, SHORT after bullish)
// Exits: Z-score reversion OR MAX_HOLD bars

pub const SPIKE_ENABLED: bool = true;
pub const SPIKE_ATR_MULT: f64 = 2.0;     // candle body must be ≥ 2x ATR(14)
pub const SPIKE_SL: f64 = 0.004;         // 0.4% stop loss
pub const SPIKE_TP: f64 = 0.003;         // 0.3% take profit
pub const SPIKE_MAX_HOLD: u32 = 12;      // ~3 hours at 15m bars
pub const SPIKE_MIN_BOOST: f64 = 1.5;    // spike body must be ≥ 1.5x avg_body_20
pub const SPIKE_Z_ENTRY: f64 = -1.0;     // z-score must confirm overshoot
```

Add new trade type:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeType {
    Regime,
    Scalp,
    Momentum,
    SpikeRev,  // NEW
}
```

Add spike detection in engine.rs:

```rust
/// Returns true if the most recent candle is a "spike" — body >= ATR * SPIKE_ATR_MULT
fn is_spike_candle(candle: &Candle, atr14: f64) -> bool {
    if atr14 <= 0.0 { return false; }
    let body = (candle.c - candle.o).abs();
    body >= atr14 * config::SPIKE_ATR_MULT
}

/// Returns LONG if last candle was bearish spike, SHORT if bullish spike
fn detect_spike_direction(candles_15m: &[Candle], atr14: f64) -> Option<Direction> {
    let curr = candles_15m.last()?;
    if !is_spike_candle(curr, atr14) { return None; }
    if curr.c < curr.o { Some(Direction::Long) } else { Some(Direction::Short) }
}
```

Add `check_spike_entry` in engine.rs — fires when:
1. Last candle was a spike (body >= ATR × 2.0)
2. Spike direction matches intended entry (fade the spike)
3. Current price has moved at least 30% back toward pre-spike direction
4. Z-score < SPIKE_Z_ENTRY for LONG (price still depressed)
5. No position, no cooldown, not in squeeze regime

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 regime + scalp + momentum (no spike strategy)
- **Comparison**: spike trades tracked separately

**Metrics to measure**:
- Spike reversion win rate (hypothesis: >55%)
- Profit factor on spike trades
- Correlation with regime trades (should be low — orthogonal signal)
- Avg hold time (should be short — 3-6 bars)

**Hypothesis**: Spike reversion trades should achieve WR >55% and PF >1.5 because large directional candles mechanically overshoot. If confirmed, add as a 4th trade type (SpikeRev) firing independently of regime/scalp/momentum.

---

## Validation Method

1. **Historical backtest** (run2_1_spike_backtest.py):
   - 18 coins, 1-year 15m data
   - Identify all spikes (body >= 2x ATR14) in dataset
   - Simulate fade-the-spike entry on bar following spike
   - Record: spike magnitude, direction, entry price, stop, TP, exit reason, P&L
   - Output: per-coin WR, PF, avg hold time, spike magnitude vs outcome

2. **Walk-forward** (run2_2_spike_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep SPIKE_ATR_MULT: 1.5 / 2.0 / 2.5
   - Sweep SPIKE_Z_ENTRY: -0.5 / -1.0 / -1.5

3. **Combined comparison** (run2_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + spike_reversion
   - Portfolio stats, correlation matrix of trade types, per-coin contribution

---

## Out-of-Sample Testing

- SPIKE_ATR_MULT sweep: 1.5 / 2.0 / 2.5 / 3.0
- SPIKE_MAX_HOLD sweep: 8 / 12 / 16 bars
- OOS: final 4 months held out from all parameter selection
- Correlation check: spike trades vs regime trades should be <0.3 to confirm orthogonality
