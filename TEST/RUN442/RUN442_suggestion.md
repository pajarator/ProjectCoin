# RUN442 — Market Compression Ratio with DMI Trend Direction

## Hypothesis

**Mechanism**: Market Compression Ratio measures the ratio of true range to the full high-low range over a period. When the ratio is low, price is compressed (closing near the high-low midpoint); when high, price is ranging widely. Low compression often precedes breakouts. DMI (Directional Movement Index) provides trend direction: when compression is low AND DMI shows a clear trend direction, the compression is building energy for a directional breakout.

**Why not duplicate**: RUN338 uses Market Compression Ratio standalone. RUN376 uses DMI Smoothed Oscillator with Volume. This RUN specifically combines Market Compression Ratio (low compression = coiled energy) with DMI trend direction (clear directional bias for the breakout) — the distinct mechanism is using compression ratio to time DMI signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN442: Market Compression Ratio with DMI Trend Direction ─────────────────────────────────────
// compression_ratio = true_range / (high - low)  // 0 = compressed, 1 = full range
// low_compression: compression_ratio < COMP_THRESH (compressed = coiled)
// dmi_plus = directional movement positive
// dmi_minus = directional movement negative
// dmi_trend: dmi_plus > dmi_minus for uptrend
// LONG: low_compression AND dmi_trend bullish (compressed for upside breakout)
// SHORT: low_compression AND dmi_trend bearish (compressed for downside breakout)

pub const COMP_DMI_ENABLED: bool = true;
pub const COMP_DMI_COMP_PERIOD: usize = 14;
pub const COMP_DMI_COMP_THRESH: f64 = 0.3;   // below = compressed
pub const COMP_DMI_DMI_PERIOD: usize = 14;
pub const COMP_DMI_SL: f64 = 0.005;
pub const COMP_DMI_TP: f64 = 0.004;
pub const COMP_DMI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run442_1_comp_dmi_backtest.py)
2. **Walk-forward** (run442_2_comp_dmi_wf.py)
3. **Combined** (run442_3_combined.py)

## Out-of-Sample Testing

- COMP_PERIOD sweep: 10 / 14 / 21
- COMP_THRESH sweep: 0.2 / 0.3 / 0.4
- DMI_PERIOD sweep: 10 / 14 / 21
