# RUN479 — Pivot Point Zone with Bollinger Band Width Expansion

## Hypothesis

**Mechanism**: Pivot Points identify key support/resistance levels based on prior period's high/low/close. Price often bounces or breaks from these levels. Bollinger Band Width Expansion confirms volatility is increasing — narrow BB Width precedes breakouts, and expansion confirms the breakout has momentum behind it. Combining pivot zones with BB Width expansion ensures entries occur at structural levels with confirmed volatility expansion.

**Why not duplicate**: RUN415 uses Pivot Point Zone Detection with RSI Extreme Filter. This RUN uses BB Width expansion instead — distinct mechanism is volatility expansion confirmation versus RSI oscillator extremes. BB Width directly measures volatility state, not overbought/oversold conditions.

## Proposed Config Changes (config.rs)

```rust
// ── RUN479: Pivot Point Zone with Bollinger Band Width Expansion ─────────────────────────────────
// pivot_zone: price near pivot_point support or resistance level
// bb_width_expanding: bb_width > bb_width_sma indicating volatility expansion
// LONG: price near pivot_s AND bb_width_expanding AND bb_width > prev_bb_width
// SHORT: price near pivot_r AND bb_width_expanding AND bb_width > prev_bb_width

pub const PIVOT_BBW_ENABLED: bool = true;
pub const PIVOT_BBW_PIVOT_PERIOD: usize = 20;
pub const PIVOT_BBW_PIVOT_ZONE_PCT: f64 = 0.001;
pub const PIVOT_BBW_BB_PERIOD: usize = 20;
pub const PIVOT_BBW_BB_STD: f64 = 2.0;
pub const PIVOT_BBW_BBW_SMA_PERIOD: usize = 20;
pub const PIVOT_BBW_SL: f64 = 0.005;
pub const PIVOT_BBW_TP: f64 = 0.004;
pub const PIVOT_BBW_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run479_1_pivot_bbw_backtest.py)
2. **Walk-forward** (run479_2_pivot_bbw_wf.py)
3. **Combined** (run479_3_combined.py)

## Out-of-Sample Testing

- PIVOT_PERIOD sweep: 14 / 20 / 30
- PIVOT_ZONE_PCT sweep: 0.0005 / 0.001 / 0.002
- BB_PERIOD sweep: 15 / 20 / 25
- BBW_SMA_PERIOD sweep: 14 / 20 / 30
