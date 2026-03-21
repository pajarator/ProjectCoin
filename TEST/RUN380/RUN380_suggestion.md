# RUN380 — Volume-Weighted RSI with Bollinger Band Filter

## Hypothesis

**Mechanism**: Volume-weighted RSI adds volume to the RSI calculation, making it more responsive to volume-backed price moves. Combine with Bollinger Band filter: only take RSI signals when price is at or near the lower BB (for LONG) or upper BB (for SHORT). When RSI triggers AND price is at the band, the signal has both oscillator and price-structure confirmation.

**Why not duplicate**: RUN131 uses volume-weighted RSI confirmation. This RUN specifically adds the Bollinger Band positional filter — requiring price to be at the band when RSI triggers adds a structural dimension that basic VWRSI lacks.

## Proposed Config Changes (config.rs)

```rust
// ── RUN380: Volume-Weighted RSI with Bollinger Band Filter ────────────────────────────────
// vw_rsi = sum(volume * price_change) / sum(volume)
// bb_position = (price - bb_lower) / (bb_upper - bb_lower)
// LONG: vw_rsi < RSI_OVERSOLD AND bb_position < BB_LOWER (price at lower band)
// SHORT: vw_rsi > RSI_OVERBOUGHT AND bb_position > BB_UPPER (price at upper band)

pub const VWRSI_BB_ENABLED: bool = true;
pub const VWRSI_BB_RSI_PERIOD: usize = 14;
pub const VWRSI_BB_RSI_OVERSOLD: f64 = 30.0;
pub const VWRSI_BB_RSI_OVERBOUGHT: f64 = 70.0;
pub const VWRSI_BB_BB_PERIOD: usize = 20;
pub const VWRSI_BB_BB_STD: f64 = 2.0;
pub const VWRSI_BB_BB_LOWER: f64 = 0.15;   // price must be in bottom 15% of BB
pub const VWRSI_BB_BB_UPPER: f64 = 0.85;   // price must be in top 15% of BB
pub const VWRSI_BB_SL: f64 = 0.005;
pub const VWRSI_BB_TP: f64 = 0.004;
pub const VWRSI_BB_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run380_1_vwrsi_bb_backtest.py)
2. **Walk-forward** (run380_2_vwrsi_bb_wf.py)
3. **Combined** (run380_3_combined.py)

## Out-of-Sample Testing

- RSI_PERIOD sweep: 10 / 14 / 21
- BB_PERIOD sweep: 15 / 20 / 30
- BB_LOWER sweep: 0.10 / 0.15 / 0.20
- BB_UPPER sweep: 0.80 / 0.85 / 0.90
