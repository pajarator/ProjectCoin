# RUN188 — Keltner Channel Breakout: EMA-ATR Envelope for Momentum Explosions

## Hypothesis

**Mechanism**: Keltner Channel = EMA(close, 20) ± ATR(14) × multiplier. Unlike Bollinger Bands (stddev), ATR channels respond faster to volatility regime changes. When price closes above the upper Keltner band → momentum breakout LONG. When price closes below the lower band → momentum breakout SHORT. The ATR multiplier controls sensitivity: 2× = normal, 3× = only extreme moves.

**Why not duplicate**: No prior RUN uses Keltner Channels. All prior envelope RUNs use Bollinger Bands (stddev). Keltner is fundamentally different because it uses ATR (average true range) instead of standard deviation — ATR is smoother and adapts faster to volatility spikes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN188: Keltner Channel Breakout ─────────────────────────────────────
// keltner_mid = EMA(close, ema_period)
// keltner_upper = keltner_mid + ATR(14) × upper_mult
// keltner_lower = keltner_mid - ATR(14) × lower_mult
// Close above upper band → LONG entry
// Close below lower band → SHORT entry
// Middle line retest as exit

pub const KELTNER_ENABLED: bool = true;
pub const KELTNER_EMA_PERIOD: usize = 20;   // middle line EMA period
pub const KELTNER_ATR_PERIOD: usize = 14;    // ATR period for band width
pub const KELTNER_UPPER_MULT: f64 = 2.0;     // upper band = EMA + ATR × this
pub const KELTNER_LOWER_MULT: f64 = 2.0;     // lower band = EMA - ATR × this
pub const KELTNER_CONFIRM_BARS: u32 = 1;     // must close outside for 1 bar
pub const KELTNER_SL: f64 = 0.005;
pub const KELTNER_TP: f64 = 0.004;
pub const KELTNER_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn keltner_channel(closes: &[f64], highs: &[f64], lows: &[f64],
                       ema_period: usize, atr_period: usize,
                       upper_mult: f64, lower_mult: f64) -> (f64, f64, f64) {
    let ema = ema(closes, ema_period);
    let atr_val = atr(highs, lows, closes, atr_period);
    let upper = ema + atr_val * upper_mult;
    let lower = ema - atr_val * lower_mult;
    (ema, upper, lower)
}
```

---

## Validation Method

1. **Historical backtest** (run188_1_keltner_backtest.py)
2. **Walk-forward** (run188_2_keltner_wf.py)
3. **Combined** (run188_3_combined.py)

## Out-of-Sample Testing

- EMA_PERIOD sweep: 10 / 20 / 30
- ATR_PERIOD sweep: 10 / 14 / 20
- UPPER_MULT sweep: 1.5 / 2.0 / 2.5 / 3.0
- LOWER_MULT sweep: 1.5 / 2.0 / 2.5 / 3.0
- CONFIRM_BARS sweep: 1 / 2 / 3
