# RUN185 — Ichimoku Cloud Breakout: Multi-Component Trend Confirmation

## Hypothesis

**Mechanism**: The Ichimoku Cloud (Tenkan-sen, Kijun-sen, Senkou Span A/B) fuses support/resistance, trend direction, and momentum into one cohesive system. When price breaks above the cloud in a LONG regime → strong bullish confirmation. When price breaks below the cloud in a SHORT/ISO_SHORT regime → strong bearish confirmation. The cloud's thickness also signals conviction: thin cloud = easier breakout, thick cloud = more significant breakout.

**Why not duplicate**: No prior RUN uses Ichimoku components. All are single-indicator (RSI, MACD, Bollinger, ATR, ADX). Ichimoku is a multi-component system with no duplicate in the catalog.

## Proposed Config Changes (config.rs)

```rust
// ── RUN185: Ichimoku Cloud Breakout ─────────────────────────────────────
// tenkanSen = (HH + LL) / 2 over 9 periods
// kijunSen = (HH + LL) / 2 over 26 periods
// senkouA = (tenkan + kijun) / 2, projected 26 bars forward
// senkouB = (HH + LL) / 2 over 52 periods, projected 26 bars forward
// cloud_top = max(senkouA, senkouB)
// cloud_bot = min(senkouA, senkouB)
// cloud_thickness = (cloud_top - cloud_bot) / cloud_bot
//
// LONG entry: price crosses above cloud_top AND cloud_thickness > 0.001
// SHORT entry: price crosses below cloud_bot AND cloud_thickness > 0.001
// cloud_thickness < 0.001 = flat market → skip

pub const ICHIMOKU_ENABLED: bool = true;
pub const ICHIMOKU_TENKAN: usize = 9;        // conversion line period
pub const ICHIMOKU_KIJUN: usize = 26;       // base line period
pub const ICHIMOKU_SENKOUU: usize = 52;     // leading span B period
pub const ICHIMOKU_DISPLACEMENT: usize = 26; // forward projection
pub const ICHIMOKU_THICKNESS: f64 = 0.001;   // minimum cloud thickness filter
pub const ICHIMOKU_SL: f64 = 0.005;
pub const ICHIMOKU_TP: f64 = 0.004;
pub const ICHIMOKU_MAX_HOLD: u32 = 48;
```

Add to `CoinState` in `state.rs`:

```rust
pub tenkan_history: Vec<f64>,
pub kijun_history: Vec<f64>,
pub senkou_a_history: Vec<f64>,
pub senkou_b_history: Vec<f64>,
pub cloud_top_history: Vec<f64>,
pub cloud_bot_history: Vec<f64>,
```

Add in `indicators.rs`:

```rust
pub fn ichimoku(highs: &[f64], lows: &[f64], closes: &[f64],
                tenkan: usize, kijun: usize, senkou: usize) -> (f64, f64, f64, f64) {
    let hh = highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let ll = lows.iter().cloned().fold(f64::INFINITY, f64::min);
    let tenkan_sen = (hh + ll) / 2.0;

    // Kijun uses last kijun-period bars (not including current)
    let kijun_hh = highs[..kijun].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let kijun_ll = lows[..kijun].iter().cloned().fold(f64::INFINITY, f64::min);
    let kijun_sen = (kijun_hh + kijun_ll) / 2.0;

    let senkou_a = (tenkan_sen + kijun_sen) / 2.0;

    let senkou_hh = highs[..senkou].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let senkou_ll = lows[..senkou].iter().cloned().fold(f64::INFINITY, f64::min);
    let senkou_b = (senkou_hh + senkou_ll) / 2.0;

    let cloud_top = senkou_a.max(senkou_b);
    let cloud_bot = senkou_a.min(senkou_b);

    (tenkan_sen, kijun_sen, cloud_top, cloud_bot)
}
```

---

## Validation Method

1. **Historical backtest** (run185_1_ichimoku_backtest.py)
2. **Walk-forward** (run185_2_ichimoku_wf.py)
3. **Combined** (run185_3_combined.py)

## Out-of-Sample Testing

- TENKAN sweep: 7 / 9 / 12
- KIJUN sweep: 22 / 26 / 34
- SENKOU sweep: 44 / 52 / 72
- THICKNESS sweep: 0.0005 / 0.001 / 0.002
