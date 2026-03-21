# RUN247 — RSI-Volume Divergence: Momentum vs Conviction Discord

## Hypothesis

**Mechanism**: Momentum (RSI) and conviction (volume) should agree. When RSI is rising (momentum improving) but volume is falling (less conviction) → divergence. The price move is losing steam — fade it. When RSI is falling but volume is rising → selling is happening with conviction — follow the volume.

**Why not duplicate**: No prior RUN uses RSI-Volume divergence. All prior divergence RUNs use price-oscillator divergence. RSI-Volume is unique because it compares *two different data streams* (momentum vs volume conviction), not momentum vs price.

## Proposed Config Changes (config.rs)

```rust
// ── RUN247: RSI-Volume Divergence ────────────────────────────────────────
// rsi_trend = RSI(current) - RSI(prior N)
// vol_trend = vol_MA(current) - vol_MA(prior N)
// DIVERGENT SHORT: rsi_trend > 0 AND vol_trend < 0 (momentum up, conviction down)
// DIVERGENT LONG: rsi_trend < 0 AND vol_trend > 0 (momentum down, conviction up)

pub const RSI_VOL_DIV_ENABLED: bool = true;
pub const RSI_VOL_DIV_RSI_PERIOD: usize = 14;
pub const RSI_VOL_DIV_RSI_MA_PERIOD: usize = 5;   // RSI trend period
pub const RSI_VOL_DIV_VOL_MA_PERIOD: usize = 20;
pub const RSI_VOL_DIV_RSI_THRESH: f64 = 5.0;     // RSI change threshold
pub const RSI_VOL_DIV_VOL_THRESH: f64 = -0.1;    // volume change threshold (%)
pub const RSI_VOL_DIV_SL: f64 = 0.005;
pub const RSI_VOL_DIV_TP: f64 = 0.004;
pub const RSI_VOL_DIV_MAX_HOLD: u32 = 36;
```

---

## Validation Method

1. **Historical backtest** (run247_1_rsi_vol_div_backtest.py)
2. **Walk-forward** (run247_2_rsi_vol_div_wf.py)
3. **Combined** (run247_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- RSI_MA_PERIOD sweep: 3 / 5 / 7
- VOL_MA_PERIOD sweep: 14 / 20 / 30
