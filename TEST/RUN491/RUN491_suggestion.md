# RUN491 — Fisher Transform with RSI Momentum Confluence

## Hypothesis

**Mechanism**: Fisher Transform normalizes price data using a Gaussian distribution assumption, turning price into a bounded oscillator with sharp reversal signals. RSI Momentum Confluence provides a second momentum measurement: when Fisher Transform signals a reversal AND RSI confirms momentum is also reversing in the same direction, the reversal has both statistical normalization and dual oscillator timing.

**Why not duplicate**: RUN451 uses Fisher Transform with Bollinger Band Position Confirmation. This RUN uses RSI Momentum Confluence instead — distinct mechanism is RSI as the confirming oscillator versus BB Position as the spatial filter.

## Proposed Config Changes (config.rs)

```rust
// ── RUN491: Fisher Transform with RSI Momentum Confluence ─────────────────────────────────
// fisher_transform: gaussian_normalized price transform for sharp reversals
// fisher_cross: fisher crosses above/below signal line
// rsi_momentum: rsi direction and rate of change
// rsi_confluence: rsi is also reversing in same direction as fisher
// LONG: fisher_cross bullish AND rsi momentum also reversing upward
// SHORT: fisher_cross bearish AND rsi momentum also reversing downward

pub const FISHER_RSI_ENABLED: bool = true;
pub const FISHER_RSI_FISHER_PERIOD: usize = 10;
pub const FISHER_RSI_RSI_PERIOD: usize = 14;
pub const FISHER_RSI_RSI_MOM_PERIOD: usize = 5;
pub const FISHER_RSI_SL: f64 = 0.005;
pub const FISHER_RSI_TP: f64 = 0.004;
pub const FISHER_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run491_1_fisher_rsi_backtest.py)
2. **Walk-forward** (run491_2_fisher_rsi_wf.py)
3. **Combined** (run491_3_combined.py)

## Out-of-Sample Testing

- FISHER_PERIOD sweep: 7 / 10 / 14
- RSI_PERIOD sweep: 10 / 14 / 20
- RSI_MOM_PERIOD sweep: 3 / 5 / 7
