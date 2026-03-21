# RUN299 — Ichimoku Cloud Twist: Tenkan-Kijun Cross With Kumo Confirmation

## Hypothesis

**Mechanism**: Ichimoku Cloud uses three signals: Tenkan-Kijun cross (fast signal), price position relative to Kumo cloud (trend direction), and Kijun slope (trend strength). LONG when: TK cross above + price above cloud + Kijun rising. SHORT when: TK cross below + price below cloud + Kijun falling. This combines momentum signal with structural cloud confirmation.

**Why not duplicate**: RUN185 uses basic Ichimoku breakout (price crosses cloud). RUN126 uses Ichimoku as confirmation filter. This RUN focuses specifically on TK cross WITH Kumo twist — the cloud twist is a more refined signal than simple price-cloud crossing. TK cross is faster and more responsive than cloud boundaries.

## Proposed Config Changes (config.rs)

```rust
// ── RUN299: Ichimoku Cloud Twist ─────────────────────────────────────────────
// tenkan = (HH + LL) / 2 over period (default 9)
// kijun = (HH + LL) / 2 over period (default 26)
// senkou_a = (tenkan + kijun) / 2, projected 26 bars forward
// senkou_b = (HH + LL) / 2 over period (default 52), projected 26 bars forward
// kumo = cloud between senkou_a and senkou_b
// LONG: tk_cross_up AND price > min(senkou_a, senkou_b) AND kijun_tenkan > kijun_tenkan[1]
// SHORT: tk_cross_down AND price < max(senkou_a, senkou_b) AND kijun_tenkan < kijun_tenkan[1]

pub const ICHIMOKU_TWIST_ENABLED: bool = true;
pub const ICHIMOKU_TWIST_TENKAN: usize = 9;
pub const ICHIMOKU_TWIST_KIJUN: usize = 26;
pub const ICHIMOKU_TWIST_SENKOU_B: usize = 52;
pub const ICHIMOKU_TWIST_SL: f64 = 0.005;
pub const ICHIMOKU_TWIST_TP: f64 = 0.004;
pub const ICHIMOKU_TWIST_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run299_1_ichimoku_twist_backtest.py)
2. **Walk-forward** (run299_2_ichimoku_twist_wf.py)
3. **Combined** (run299_3_combined.py)

## Out-of-Sample Testing

- TENKAN sweep: 7 / 9 / 12
- KIJUN sweep: 22 / 26 / 34
- SENKOU_B sweep: 44 / 52 / 72
