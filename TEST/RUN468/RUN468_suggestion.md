# RUN468 — Copock Curve with RSI Extreme Filter

## Hypothesis

**Mechanism**: Copock Curve is a long-term momentum indicator combining rate-of-change with smoothed moving averages, originally designed for equity markets to identify buying opportunities. RSI Extreme Filter adds timing precision: only take Copock Curve buy signals when RSI is at oversold extremes, ensuring entries occur at the beginning of uptrends when momentum also confirms oversold conditions.

**Why not duplicate**: RUN404 uses Copock Curve with Ease of Movement Divergence. This RUN uses RSI Extreme instead — the distinct mechanism is using RSI extremes for entry timing versus Ease of Movement for volume-price relationship confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN468: Copock Curve with RSI Extreme Filter ─────────────────────────────────
// copock_curve: weighted smoothed roc for long-term momentum
// copock_cross: copock crosses above/below 0
// rsi_extreme: rsi < RSI_OVERSOLD or rsi > RSI_OVERBOUGHT
// LONG: copock_cross bullish (below 0) AND rsi < RSI_OVERSOLD
// SHORT: copock_cross bearish (above 0) AND rsi > RSI_OVERBOUGHT

pub const COPO_RSI_ENABLED: bool = true;
pub const COPO_RSI_COPO_PERIOD1: usize = 11;
pub const COPO_RSI_COPO_PERIOD2: usize = 14;
pub const COPO_RSI_COPO_PERIOD3: usize = 21;
pub const COPO_RSI_COPO_SMA: usize = 10;
pub const COPO_RSI_RSI_PERIOD: usize = 14;
pub const COPO_RSI_RSI_OVERSOLD: f64 = 35.0;
pub const COPO_RSI_RSI_OVERBOUGHT: f64 = 65.0;
pub const COPO_RSI_SL: f64 = 0.005;
pub const COPO_RSI_TP: f64 = 0.004;
pub const COPO_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run468_1_copo_rsi_backtest.py)
2. **Walk-forward** (run468_2_copo_rsi_wf.py)
3. **Combined** (run468_3_combined.py)

## Out-of-Sample Testing

- COPO_PERIOD1 sweep: 9 / 11 / 14
- COPO_PERIOD2 sweep: 11 / 14 / 17
- COPO_PERIOD3 sweep: 18 / 21 / 25
- RSI_OVERSOLD sweep: 30 / 35 / 40
- RSI_OVERBOUGHT sweep: 60 / 65 / 70
