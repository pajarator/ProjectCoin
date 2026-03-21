# RUN435 — Williams %R with Bollinger Band Position Filter

## Hypothesis

**Mechanism**: Williams %R measures the current close relative to the high-low range, oscillating between 0 and -100. Bollinger Band Position (BBP) measures where the current price is within the Bollinger Bands — 0% means at the lower band, 100% means at the upper band. When Williams %R reaches extreme AND Bollinger Band Position is at an extreme (near 0% for longs or near 100% for shorts), the signal has both oscillator extreme and band position confirmation. This combination catches reversals at statistical extremes of both indicators.

**Why not duplicate**: RUN371 uses Williams %R with RSI Confluence. RUN407 uses Williams %R with ADX Trend Strength and EMA Alignment. This RUN specifically uses Bollinger Band Position as the confirmation filter — the distinct mechanism is requiring price to be at a specific position within the BB range at the time of the Williams %R extreme.

## Proposed Config Changes (config.rs)

```rust
// ── RUN435: Williams %R with Bollinger Band Position Filter ──────────────────────────────────────
// williams_r = (highest_high - close) / (highest_high - lowest_low) * -100
// wr_extreme: williams_r < WR_OVERSOLD or > WR_OVERBOUGHT
// bb_position = (close - bb_lower) / (bb_upper - bb_lower)  [0-100%]
// bb_position_extreme: bb_position < BBP_LOWER for longs, bb_position > BBP_UPPER for shorts
// LONG: williams_r < WR_OVERSOLD AND bb_position < BBP_LOWER
// SHORT: williams_r > WR_OVERBOUGHT AND bb_position > BBP_UPPER

pub const WR_BBP_ENABLED: bool = true;
pub const WR_BBP_WR_PERIOD: usize = 14;
pub const WR_BBP_WR_OVERSOLD: f64 = -80.0;
pub const WR_BBP_WR_OVERBOUGHT: f64 = -20.0;
pub const WR_BBP_BB_PERIOD: usize = 20;
pub const WR_BBP_BB_STD: f64 = 2.0;
pub const WR_BBP_BBP_LOWER: f64 = 0.15;   // price in bottom 15% of BB
pub const WR_BBP_BBP_UPPER: f64 = 0.85;   // price in top 15% of BB
pub const WR_BBP_SL: f64 = 0.005;
pub const WR_BBP_TP: f64 = 0.004;
pub const WR_BBP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run435_1_wr_bbp_backtest.py)
2. **Walk-forward** (run435_2_wr_bbp_wf.py)
3. **Combined** (run435_3_combined.py)

## Out-of-Sample Testing

- WR_PERIOD sweep: 10 / 14 / 21
- WR_OVERSOLD sweep: -85 / -80 / -75
- WR_OVERBOUGHT sweep: -25 / -20 / -15
- BBP_LOWER sweep: 0.10 / 0.15 / 0.20
- BBP_UPPER sweep: 0.80 / 0.85 / 0.90
