# RUN334 — ZigZag Breakout with Volume Surge: Structural Pivot Confirmation

## Hypothesis

**Mechanism**: ZigZag identifies structural price pivots (swing highs/lows) by filtering out moves smaller than a threshold percentage. Once ZigZag identifies a swing high/low, the next break above/below that level with volume confirmation is a high-probability continuation move. The ZigZag level acts as a price ceiling/floor; breaking it with volume surge indicates the trend has enough force to continue.

**Why not duplicate**: RUN274 uses ZigZag for trend detection. This RUN uses ZigZag specifically as a breakout confirmation tool — the pivot level is the resistance/support, and the breakout with volume is the entry trigger. The distinct mechanism is using ZigZag pivots as structural levels rather than trend direction signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN334: ZigZag Breakout with Volume Surge ─────────────────────────────────
// zig_zag(level) = pivot points where price reverses by at least level%
// swing_high = zig_zag point with prior high > adjacent highs
// swing_low = zig_zag point with prior low < adjacent lows
// breakout_up = close crosses above most recent swing_high AND volume > vol_avg * VOL_MULT
// breakout_down = close crosses below most recent swing_low AND volume > vol_avg * VOL_MULT

pub const ZIGZAG_BRK_ENABLED: bool = true;
pub const ZIGZAG_BRK_LEVEL: f64 = 0.02;     // 2% reversal threshold for pivot
pub const ZIGZAG_BRK_VOL_MULT: f64 = 2.0;   // volume must exceed 2x average
pub const ZIGZAG_BRK_SL: f64 = 0.005;
pub const ZIGZAG_BRK_TP: f64 = 0.004;
pub const ZIGZAG_BRK_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run334_1_zigzag_brk_backtest.py)
2. **Walk-forward** (run334_2_zigzag_brk_wf.py)
3. **Combined** (run334_3_combined.py)

## Out-of-Sample Testing

- LEVEL sweep: 0.01 / 0.02 / 0.03
- VOL_MULT sweep: 1.5 / 2.0 / 2.5
