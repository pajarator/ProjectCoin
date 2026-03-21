# RUN167 — Heikin-Ashi Trend Confirmation: HA Candle Momentum Filter for Regime Entries

## Hypothesis

**Mechanism**: Heikin-Ashi (HA) candles smooth price noise by averaging OHLC data. A sequence of HA candles with all-green bodies signals strong upward momentum — confirm LONG entries. All-red bodies signal strong downward — confirm SHORT entries. HA trend confirmation as a filter for regime entries reduces false breakouts.

**Why not duplicate**: No prior RUN uses HA candles. All existing signals use raw OHLCV. HA is a smoothed overlay that filters noise.

## Proposed Config Changes (config.rs)

```rust
// ── RUN167: Heikin-Ashi Trend Confirmation ──────────────────────────────
// HA requires N consecutive same-color candles to confirm trend
// GREEN: all HA candles bullish for N bars → confirm LONG
// RED: all HA candles bearish for N bars → confirm SHORT

pub const HA_ENABLED: bool = true;
pub const HA_BARS_CONFIRM: usize = 3;    // 3 consecutive HA candles same color to confirm
pub const HA_FILTER_MODE: bool = true;   // true=filter entries, false=trigger entries
```

Add to `CoinState` in `state.rs`:

```rust
pub ha_consecutive_green: usize,   // consecutive green HA candles
pub ha_consecutive_red: usize,     // consecutive red HA candles
```

Add HA calculation in `indicators.rs`:

```rust
/// Compute Heikin-Ashi candle from raw OHLC
pub fn compute_ha(candle: &Candle, prev_ha_close: f64) -> HACandle {
    let ha_close = (candle.o + candle.h + candle.l + candle.c) / 4.0;
    let ha_open = if prev_ha_close == 0.0 {
        (candle.o + candle.c) / 2.0
    } else {
        (prev_ha_close + prev_ha_close) / 2.0  // simplified: prev HA open
    };
    let ha_high = candle.h.max(ha_close).max(ha_open);
    let ha_low = candle.l.min(ha_close).min(ha_open);
    HACandle { open: ha_open, high: ha_high, low: ha_low, close: ha_close }
}

pub struct HACandle { pub open: f64, pub high: f64, pub low: f64, pub close: f64 }
```

Add entry filter in `engine.rs`:

```rust
fn ha_confirm_long(state: &SharedState, ci: usize) -> bool {
    if !config::HA_ENABLED { return true; }
    state.coins[ci].ha_consecutive_green >= config::HA_BARS_CONFIRM
}
fn ha_confirm_short(state: &SharedState, ci: usize) -> bool {
    if !config::HA_ENABLED { return true; }
    state.coins[ci].ha_consecutive_red >= config::HA_BARS_CONFIRM
}
```

Modify regime LONG entry: only allow if `ha_confirm_long(state, ci)` returns true.
Modify regime SHORT entry: only allow if `ha_confirm_short(state, ci)` returns true.

---

## Validation Method

1. **Historical backtest** (run167_1_ha_backtest.py): 18 coins, sweep HA_BARS_CONFIRM
2. **Walk-forward** (run167_2_ha_wf.py): 3-window walk-forward
3. **Combined** (run167_3_combined.py): vs baseline

## Out-of-Sample Testing

- BARS_CONFIRM sweep: 2 / 3 / 4 / 5
- FILTER_MODE: true vs false
