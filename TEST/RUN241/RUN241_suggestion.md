# RUN241 — RSI Bollinger Band Squeeze: Oscillator Compression Before Momentum Explosions

## Hypothesis

**Mechanism**: Apply Bollinger Bands to the RSI values themselves (not price). When RSI compresses within its Bollinger Bands (bandwidth drops below a threshold) → the oscillator is coiling. When RSI breaks OUT of the bands → strong momentum signal that's more reliable than standard RSI crosses. RSI breaks above upper band → strong bullish momentum. RSI breaks below lower band → strong bearish momentum.

**Why not duplicate**: No prior RUN applies Bollinger Bands to RSI. All prior RSI RUNs use RSI on price. This is a second-order application: RSI as the input to Bollinger Bands, creating a momentum-of-momentum signal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN241: RSI Bollinger Band Squeeze ───────────────────────────────────
// rsi_val = RSI(close, period)
// bb_rsi = Bollinger Bands applied to RSI values
// rsi_upper = bb_mid + bb_std × 2
// rsi_lower = bb_mid - bb_std × 2
// LONG: RSI crosses above rsi_upper AND prior RSI < rsi_upper (breakout)
// SHORT: RSI crosses below rsi_lower AND prior RSI > rsi_lower (breakout)

pub const RSI_BB_SQUEEZE_ENABLED: bool = true;
pub const RSI_BB_PERIOD: usize = 14;         // RSI period
pub const RSI_BB_BB_PERIOD: usize = 20;      // Bollinger period for RSI
pub const RSI_BB_STD: f64 = 2.0;             // BB std dev multiplier
pub const RSI_BB_SL: f64 = 0.005;
pub const RSI_BB_TP: f64 = 0.004;
pub const RSI_BB_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run241_1_rsi_bb_backtest.py)
2. **Walk-forward** (run241_2_rsi_bb_wf.py)
3. **Combined** (run241_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- BB_PERIOD sweep: 15 / 20 / 30
- BB_STD sweep: 1.5 / 2.0 / 2.5
