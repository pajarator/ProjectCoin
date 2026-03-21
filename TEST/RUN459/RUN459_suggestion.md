# RUN459 — Aroon Oscillator with Bollinger Band Width

## Hypothesis

**Mechanism**: Aroon Oscillator measures trend strength via the time since last high/low within a period. Aroon Up > Aroon Down = bullish trend, and the crossing point indicates momentum shifts. Bollinger Band Width serves as a volatility expansion filter: only take Aroon crossover signals when BB Width is expanding (not contracting), ensuring entries occur during volatility expansion rather than during compression phases that often precede range-bound markets.

**Why not duplicate**: RUN420 uses Aroon Oscillator with Volume Confirmation. This RUN uses BB Width instead — the distinct mechanism is volatility expansion timing: Aroon confirms trend direction and BB Width confirms the trend is manifesting in volatility expansion rather than choppy compression.

## Proposed Config Changes (config.rs)

```rust
// ── RUN459: Aroon Oscillator with Bollinger Band Width ─────────────────────────────────
// aroon_oscillator: aroon_up - aroon_down
// aroon_cross: aroon_up crosses above/below aroon_down
// bb_width_expanding: bb_width > bb_width_sma (volatility expansion)
// LONG: aroon_cross bullish AND bb_width_expanding
// SHORT: aroon_cross bearish AND bb_width_expanding

pub const AROON_BBW_ENABLED: bool = true;
pub const AROON_BBW_AROON_PERIOD: usize = 14;
pub const AROON_BBW_AROON_UP_THRESH: f64 = 70.0;
pub const AROON_BBW_AROON_DOWN_THRESH: f64 = 30.0;
pub const AROON_BBW_BB_PERIOD: usize = 20;
pub const AROON_BBW_BB_STD: f64 = 2.0;
pub const AROON_BBW_BBW_SMA_PERIOD: usize = 20;
pub const AROON_BBW_SL: f64 = 0.005;
pub const AROON_BBW_TP: f64 = 0.004;
pub const AROON_BBW_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run459_1_aroon_bbw_backtest.py)
2. **Walk-forward** (run459_2_aroon_bbw_wf.py)
3. **Combined** (run459_3_combined.py)

## Out-of-Sample Testing

- AROON_PERIOD sweep: 10 / 14 / 20 / 25
- AROON_UP_THRESH sweep: 65 / 70 / 75
- BB_PERIOD sweep: 15 / 20 / 30
- BBW_SMA_PERIOD sweep: 14 / 20 / 30
