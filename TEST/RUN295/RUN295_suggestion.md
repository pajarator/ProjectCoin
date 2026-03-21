# RUN295 — Trend Exhaustion Index: Consecutive Up/Down Days Counter

## Hypothesis

**Mechanism**: Count consecutive bars where close > prior close (up bars) vs close < prior close (down bars). When up_bar_count reaches 8+ → market is exhausted on the upside → expect mean reversion down. When down_bar_count reaches 8+ → expect mean reversion up. The higher the count, the stronger the exhaustion.

**Why not duplicate**: RUN228 uses consecutive candle patterns. This RUN counts *consecutive directional closes* specifically, not just candlestick patterns. It's simpler and more direct.

## Proposed Config Changes (config.rs)

```rust
// ── RUN295: Trend Exhaustion Index ──────────────────────────────────────
// consecutive_up = count of bars where close > close[1]
// consecutive_down = count of bars where close < close[1]
// LONG: consecutive_down >= 8 (exhausted sellers)
// SHORT: consecutive_up >= 8 (exhausted buyers)

pub const EXHAUSTION_ENABLED: bool = true;
pub const EXHAUSTION_THRESH: u32 = 8;
pub const EXHAUSTION_SL: f64 = 0.005;
pub const EXHAUSTION_TP: f64 = 0.004;
pub const EXHAUSTION_MAX_HOLD: u32 = 24;
```

---

## Validation Method

1. **Historical backtest** (run295_1_exhaustion_backtest.py)
2. **Walk-forward** (run295_2_exhaustion_wf.py)
3. **Combined** (run295_3_combined.py)

## Out-of-Sample Testing

- THRESH sweep: 6 / 8 / 10
