# RUN371 — Williams %R with RSI Overbought/Oversold Confluence

## Hypothesis

**Mechanism**: When Williams %R and RSI both reach extreme levels simultaneously, the signal is much stronger. Both oscillators measuring overbought/oversold from different angles agreeing = high conviction reversal. Williams %R at -100 AND RSI below 30 = extreme oversold with no room left to fall. Williams %R at 0 AND RSI above 70 = extreme overbought with no room left to rise.

**Why not duplicate**: RUN237 uses Williams %R + EMA filter. RUN267 uses Williams %R percentile. This RUN specifically uses dual-oscillator confluence with RSI — the distinct mechanism is requiring both Williams %R and RSI to agree at extreme levels simultaneously.

## Proposed Config Changes (config.rs)

```rust
// ── RUN371: Williams %R with RSI Overbought/Oversold Confluence ────────────────────────────
// williams_r(period) = (highest - close) / (highest - lowest) * -100
// LONG: williams_r < WR_OVERSOLD AND RSI < RSI_OVERSOLD (both agree oversold)
// SHORT: williams_r > WR_OVERBOUGHT AND RSI > RSI_OVERBOUGHT (both agree overbought)
// Confluence requirement: both oscillators must agree for signal

pub const WR_RSI_ENABLED: bool = true;
pub const WR_RSI_WR_PERIOD: usize = 14;
pub const WR_RSI_RSI_PERIOD: usize = 14;
pub const WR_RSI_WR_OVERSOLD: f64 = -90.0;
pub const WR_RSI_WR_OVERBOUGHT: f64 = -10.0;
pub const WR_RSI_RSI_OVERSOLD: f64 = 30.0;
pub const WR_RSI_RSI_OVERBOUGHT: f64 = 70.0;
pub const WR_RSI_SL: f64 = 0.005;
pub const WR_RSI_TP: f64 = 0.004;
pub const WR_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run371_1_wr_rsi_backtest.py)
2. **Walk-forward** (run371_2_wr_rsi_wf.py)
3. **Combined** (run371_3_combined.py)

## Out-of-Sample Testing

- WR_PERIOD sweep: 10 / 14 / 21
- RSI_PERIOD sweep: 10 / 14 / 21
- WR_OVERSOLD sweep: -95 / -90 / -85
- WR_OVERBOUGHT sweep: -15 / -10 / -5
- RSI_OVERSOLD sweep: 25 / 30 / 35
- RSI_OVERBOUGHT sweep: 65 / 70 / 75
