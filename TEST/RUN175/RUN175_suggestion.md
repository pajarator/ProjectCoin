# RUN175 — Volume-Price Divergence: Volume Surge Without Price Movement as Reversal Signal

## Hypothesis

**Mechanism**: When volume spikes (2x+ average) but price barely moves (<0.2% in the same direction), it signals distribution (high volume, price flat = smart money absorbing selling without price drop). This is a classic accumulation/distribution pattern. Price must eventually follow the volume — in the opposite direction. Large volume without price movement = reversal.

**Why not duplicate**: No prior RUN uses volume-price divergence specifically. Scalp vol_spike_rev (existing) uses volume spike + RSI extreme. This uses volume spike + price STALLING (no directional price move).

## Proposed Config Changes (config.rs)

```rust
// ── RUN175: Volume-Price Divergence ─────────────────────────────────────
// vol_r > VOL_SPIKE_MULT AND |price_change| < STALL_THRESH → divergence
// divergence → mean-reversion entry in opposite direction

pub const VOLDIV_ENABLED: bool = true;
pub const VOLDIV_VOL_MULT: f64 = 2.0;      // volume must be 2x average
pub const VOLDIV_STALL_THRESH: f64 = 0.002;  // price change < 0.2% = stall
pub const VOLDIV_LOOKBACK: usize = 4;        // bars to measure stall
```

Add to `CoinState` in `state.rs`:

```rust
pub vol_divergence_bars: usize,  // consecutive bars of vol spike + price stall
```

Add in `engine.rs`:

```rust
fn check_voldiv_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::VOLDIV_ENABLED { return None; }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }

    let vol_r = ind.vol / ind.vol_ma;
    if vol_r < config::VOLDIV_VOL_MULT { return None; }

    // Measure price change over last 4 bars
    let bars = &cs.candles_15m[cs.candles_15m.len().saturating_sub(config::VOLDIV_LOOKBACK)..];
    if bars.len() < 2 { return None; }
    let start_price = bars.first()?.o;
    let end_price = bars.last()?.c;
    let pct_change = (end_price - start_price) / start_price;

    if pct_change.abs() > config::VOLDIV_STALL_THRESH { return None; }  // not stalled

    // Volume spiked + price stalled → reversal coming
    if pct_change > 0.0 {
        return Some((Direction::Short, "vol_price_div"));  // price up but stalling → SHORT
    } else {
        return Some((Direction::Long, "vol_price_div"));    // price down but stalling → LONG
    }
}
```

---

## Validation Method

1. **Historical backtest** (run175_1_voldiv_backtest.py)
2. **Walk-forward** (run175_2_voldiv_wf.py)
3. **Combined** (run175_3_combined.py)

## Out-of-Sample Testing

- VOL_MULT sweep: 1.5 / 2.0 / 2.5
- STALL_THRESH sweep: 0.001 / 0.002 / 0.003
