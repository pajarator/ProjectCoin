# RUN189 — Donchian Channel Breakout: N-Period High/Low Momentum System

## Hypothesis

**Mechanism**: Donchian Channel = highest high and lowest low over a lookback period. Price breaking above the channel's upper band = momentum explosion higher (LONG). Price breaking below the lower band = momentum crash lower (SHORT). The break must be confirmed by volume (volume > 20-period average) to filter false breakouts.

**Why not duplicate**: No prior RUN uses Donchian Channels. All prior breakout RUNs use Bollinger Bands, ATR channels, or EMA crosses. Donchian is the cleanest pure momentum system (price-only, no smoothing).

## Proposed Config Changes (config.rs)

```rust
// ── RUN189: Donchian Channel Breakout ───────────────────────────────────
// donchian_upper = max(high, period)
// donchian_lower = min(low, period)
// donchian_mid = (upper + lower) / 2
// LONG: close crosses above donchian_upper AND volume > vol_ma
// SHORT: close crosses below donchian_lower AND volume > vol_ma
// Exit: retest of mid-line or opposite band

pub const DONCHIAN_ENABLED: bool = true;
pub const DONCHIAN_PERIOD: usize = 20;        // lookback for HH/LL
pub const DONCHIAN_VOL_MA: usize = 20;        // volume MA for confirmation
pub const DONCHIAN_VOL_MULT: f64 = 1.2;        // volume must exceed MA × this
pub const DONCHIAN_SL: f64 = 0.005;
pub const DONCHIAN_TP: f64 = 0.004;
pub const DONCHIAN_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn donchian_channel(highs: &[f64], lows: &[f64], period: usize) -> (f64, f64, f64) {
    let period = period.min(highs.len().min(lows.len()));
    if period == 0 {
        return (0.0, 0.0, 0.0);
    }
    let upper = highs[..period].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let lower = lows[..period].iter().cloned().fold(f64::INFINITY, f64::min);
    let mid = (upper + lower) / 2.0;
    (upper, mid, lower)
}
```

---

## Validation Method

1. **Historical backtest** (run189_1_donchian_backtest.py)
2. **Walk-forward** (run189_2_donchian_wf.py)
3. **Combined** (run189_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 20 / 40 / 60
- VOL_MA sweep: 10 / 20 / 30
- VOL_MULT sweep: 1.0 / 1.2 / 1.5
