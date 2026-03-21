# RUN338 — Market Compression Ratio: Volume-Doji-ATR Hybrid

## Hypothesis

**Mechanism**: Market compression ratio = (ATR / volume) * body_size. High ratio = high volatility per unit volume (explosive moves). Low ratio = low volatility per unit volume (compressed, quiet market). When ratio drops below a threshold for N consecutive bars → compression → expect explosive move in direction of volume surge. When ratio spikes above threshold → volatility expansion breakout.

**Why not duplicate**: No prior RUN uses this specific ratio. ATR is used for stops and volatility (RUN26, RUN245, RUN250, RUN266). Volume alone is covered in many RUNs. The ATR/volume ratio as a compression indicator is unique — it captures whether the market is coiled tight (low ratio) or releasing energy (high ratio).

## Proposed Config Changes (config.rs)

```rust
// ── RUN338: Market Compression Ratio ──────────────────────────────────────────
// mcr = (ATR(period) / volume) * body_size_normalized
// compression = mcr < COMPRESS_THRESH for CONSEC_BARS consecutive bars
// expansion = mcr > EXPAND_THRESH (volatility breakout)
// LONG: compression_complete AND volume > avg_vol * VOL_SPIKE
// SHORT: compression_complete AND volume > avg_vol * VOL_SPIKE (direction from prior trend)

pub const MCR_ENABLED: bool = true;
pub const MCR_ATR_PERIOD: usize = 14;
pub const MCR_COMPRESS_THRESH: f64 = 0.0001;  // below this = compressed
pub const MCR_EXPAND_THRESH: f64 = 0.001;     // above this = expansion
pub const MCR_CONSEC_BARS: u32 = 5;           // bars of compression before signal
pub const MCR_VOL_SPIKE: f64 = 2.0;          // volume must exceed 2x avg
pub const MCR_SL: f64 = 0.005;
pub const MCR_TP: f64 = 0.004;
pub const MCR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run338_1_mcr_backtest.py)
2. **Walk-forward** (run338_2_mcr_wf.py)
3. **Combined** (run338_3_combined.py)

## Out-of-Sample Testing

- ATR_PERIOD sweep: 10 / 14 / 21
- COMPRESS_THRESH sweep: 0.00005 / 0.0001 / 0.0002
- EXPAND_THRESH sweep: 0.0005 / 0.001 / 0.002
- CONSEC_BARS sweep: 3 / 5 / 8
