# RUN404 — Coppock Curve with Ease of Movement Divergence

## Hypothesis

**Mechanism**: The Coppock Curve is a long-term momentum indicator designed to identify trend reversals at major market bottoms (originally for the S&P 500). It combines rate of change measurements over longer periods. Ease of Movement (EOM) measures how much price moves relative to volume — high EOM means price moves easily with low volume. When the Coppock Curve bottoms and turns up AND EOM shows divergence (price making lows but EOM not confirming), the reversal has both long-term momentum and volume-facilitated price movement alignment. This catches major turning points.

**Why not duplicate**: RUN315 uses Coppock Curve standalone. RUN336 uses Ease of Movement with EMA Slope. This RUN specifically combines Coppock Curve (long-term momentum reversal) with EOM divergence (volume-backed ease of movement) — the distinct mechanism is using long-term Coppock timing combined with EOM's volume-efficiency divergence for confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN404: Coppock Curve with Ease of Movement Divergence ──────────────────────────────
// coppock_curve = weighted moving average of (ROC(close, period1) + ROC(close, period2))
// coppock_turn: coppock curve crosses above/below its signal line (WMA)
// eom = (high - low) / volume * 1000  (simplified)
// eom_divergence: price makes lower low but eom makes higher low (bullish div)
// LONG: coppock turns bullish AND eom_divergence present
// SHORT: coppock turns bearish AND eom_divergence present (price higher high, eom lower high)

pub const COPPOCK_EOM_ENABLED: bool = true;
pub const COPPOCK_EOM_ROC1_PERIOD: usize = 11;
pub const COPPOCK_EOM_ROC2_PERIOD: usize = 14;
pub const COPPOCK_EOM_SIGNAL_PERIOD: usize = 10;
pub const COPPOCK_EOM_EOM_PERIOD: usize = 14;
pub const COPPOCK_EOM_EOM_SMA_PERIOD: usize = 20;  // smooth EOM
pub const COPPOCK_EOM_SL: f64 = 0.005;
pub const COPPOCK_EOM_TP: f64 = 0.004;
pub const COPPOCK_EOM_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run404_1_coppock_eom_backtest.py)
2. **Walk-forward** (run404_2_coppock_eom_wf.py)
3. **Combined** (run404_3_combined.py)

## Out-of-Sample Testing

- ROC1_PERIOD sweep: 8 / 11 / 14
- ROC2_PERIOD sweep: 11 / 14 / 18
- SIGNAL_PERIOD sweep: 7 / 10 / 14
- EOM_PERIOD sweep: 10 / 14 / 21
