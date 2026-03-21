# RUN312 — Relative Vigor Index: Where Close Settles vs Open

## Hypothesis

**Mechanism**: RVI = (close - open) / (high - low). This measures where the close settles relative to the open and the bar's full range. In a bullish bar, close near the high = high RVI (close to open in strong up bar). In a bearish bar, close near the low = low RVI. Smooth RVI with an EMA and compare to its signal line. RVI crossing above signal = bullish vigor. RVI crossing below = bearish vigor. Use divergence between RVI and price for early reversal signals.

**Why not duplicate**: No prior RUN uses RVI. This is distinct from Intraday Intensity (RUN301) because RVI normalizes by the bar's full range rather than weighting by volume. It's also simpler than MFI or RSI — purely a close-open positioning indicator.

## Proposed Config Changes (config.rs)

```rust
// ── RUN312: Relative Vigor Index ─────────────────────────────────────────────
// rvi = (close - open) / (high - low)
// rvi_smooth = EMA(rvi, period)
// rvi_signal = EMA(rvi_smooth, signal_period)
// LONG: rvi_smooth crosses above rvi_signal AND rvi > 0
// SHORT: rvi_smooth crosses below rvi_signal AND rvi < 0
// Divergence: price.new_high but rvi.lower_high = bearish divergence

pub const RVI_ENABLED: bool = true;
pub const RVI_PERIOD: usize = 10;
pub const RVI_SIGNAL: usize = 4;
pub const RVI_SL: f64 = 0.005;
pub const RVI_TP: f64 = 0.004;
pub const RVI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run312_1_rvi_backtest.py)
2. **Walk-forward** (run312_2_rvi_wf.py)
3. **Combined** (run312_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 8 / 10 / 14
- SIGNAL sweep: 3 / 4 / 6
