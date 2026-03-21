# RUN372 — Volume Ratio Spike with RSI Reversal

## Hypothesis

**Mechanism**: Volume ratio = current volume / average volume. A sudden spike in volume ratio (e.g., >3× average) without a proportional price move indicates unusual activity — either accumulation or distribution. When volume spikes AND RSI shows the move is extended → reversal. The volume spike reveals what the price hasn't yet disclosed.

**Why not duplicate**: No prior RUN uses volume ratio spike combined with RSI. RUN109 uses volume surge confirmation. This RUN specifically combines sudden volume ratio spikes with RSI extension — the distinct mechanism is the combination of volume anomaly detection + RSI timing.

## Proposed Config Changes (config.rs)

```rust
// ── RUN372: Volume Ratio Spike with RSI Reversal ────────────────────────────────
// vol_ratio = volume / SMA(volume, period)
// vol_ratio_spike = vol_ratio > RATIO_SPIKE (e.g., > 3x average)
// LONG: vol_ratio_spike AND RSI < RSI_OVERSOLD AND price at support
// SHORT: vol_ratio_spike AND RSI > RSI_OVERBOUGHT AND price at resistance

pub const VOL_RATIO_RSI_ENABLED: bool = true;
pub const VOL_RATIO_RSI_VOL_PERIOD: usize = 20;
pub const VOL_RATIO_RSI_RATIO_SPIKE: f64 = 3.0;   // spike threshold
pub const VOL_RATIO_RSI_RSI_PERIOD: usize = 14;
pub const VOL_RATIO_RSI_RSI_OVERSOLD: f64 = 35.0;
pub const VOL_RATIO_RSI_RSI_OVERBOUGHT: f64 = 65.0;
pub const VOL_RATIO_RSI_SL: f64 = 0.005;
pub const VOL_RATIO_RSI_TP: f64 = 0.004;
pub const VOL_RATIO_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run372_1_vol_ratio_rsi_backtest.py)
2. **Walk-forward** (run372_2_vol_ratio_rsi_wf.py)
3. **Combined** (run372_3_combined.py)

## Out-of-Sample Testing

- RATIO_SPIKE sweep: 2.0 / 3.0 / 4.0
- VOL_PERIOD sweep: 14 / 20 / 30
- RSI_OVERSOLD sweep: 30 / 35 / 40
- RSI_OVERBOUGHT sweep: 60 / 65 / 70
