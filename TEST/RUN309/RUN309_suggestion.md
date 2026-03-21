# RUN309 — Ulcer Index Compression: Low-Distress Mean Reversion

## Hypothesis

**Mechanism**: Ulcer Index (UI) measures downside volatility — specifically the depth and duration of drawdowns from recent highs. Unlike ATR (measures total volatility), UI measures only downside pain. When UI drops to historical lows → market complacency → mean-reversion likely to snap back up. When UI spikes → market distress → potential trend continuation. Trade the compression: low UI followed by a sharp price move away from the mean.

**Why not duplicate**: No prior RUN uses Ulcer Index. ATR percent rank (RUN250) and ADX percentile rank (RUN266) measure directional/volatility. Ulcer is distinct because it specifically measures drawdown depth — the "pain" dimension of volatility, not the range width. It's more psychologically relevant than pure volatility.

## Proposed Config Changes (config.rs)

```rust
// ── RUN309: Ulcer Index Compression ───────────────────────────────────────────
// ui = sqrt(sum((drawdown_percent^2)) / n)   — RMS of drawdown percentages
// ui_percentile = percentile_rank(ui, lookback=100)
// LONG: ui_percentile < ui_compression AND price below SMA(20)
// SHORT: ui_percentile < ui_compression AND price above SMA(20)
// Compression = low UI (market is quiet), then price move triggers reversal

pub const UI_COMPRESS_ENABLED: bool = true;
pub const UI_PERIOD: usize = 14;
pub const UI_LOOKBACK: usize = 100;
pub const UI_COMPRESSION: f64 = 20.0;        // bottom 20th percentile = compressed
pub const UI_CONFIRM_SMA: usize = 20;        // SMA period for direction
pub const UI_SL: f64 = 0.005;
pub const UI_TP: f64 = 0.004;
pub const UI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run309_1_ui_compress_backtest.py)
2. **Walk-forward** (run309_2_ui_compress_wf.py)
3. **Combined** (run309_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- COMPRESSION sweep: 10 / 20 / 30
- LOOKBACK sweep: 50 / 100 / 200
