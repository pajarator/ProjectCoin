# RUN490 — Opening Range Gap with RSI Extreme Filter

## Hypothesis

**Mechanism**: Opening Range Gap identifies price gaps at the market open that exceed a threshold percentage of the opening range. These gaps often occur due to overnight news or sentiment and can signal momentum continuation. RSI Extreme Filter ensures the gap occurs from an oversold/overbought base, making the gap a momentum expansion rather than an exhaustion gap. When a gap forms from extreme RSI AND the gap exceeds a meaningful threshold, entries have both momentum base and directional conviction.

**Why not duplicate**: RUN450 uses Opening Range Gap with VWAP Distance Confirmation. This RUN uses RSI Extreme instead — distinct mechanism is using RSI extremes as the confirming condition versus VWAP distance. RSI extremes indicate prior momentum exhaustion that can fuel the gap continuation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN490: Opening Range Gap with RSI Extreme Filter ─────────────────────────────────
// opening_range: high/low range established in first N minutes of trading
// gap_threshold: gap must exceed this % of opening range to qualify
// rsi_extreme: rsi < RSI_OVERSOLD or rsi > RSI_OVERBOUGHT
// LONG: price gaps up AND rsi > RSI_OVERBOUGHT (not overheated)
// SHORT: price gaps down AND rsi < RSI_OVERSOLD (not oversold exhausted)

pub const ORG_RSI_ENABLED: bool = true;
pub const ORG_RSI_OPEN_RANGE_PERIOD: usize = 15;
pub const ORG_RSI_GAP_THRESH: f64 = 0.005;
pub const ORG_RSI_RSI_PERIOD: usize = 14;
pub const ORG_RSI_RSI_OVERSOLD: f64 = 35.0;
pub const ORG_RSI_RSI_OVERBOUGHT: f64 = 65.0;
pub const ORG_RSI_SL: f64 = 0.005;
pub const ORG_RSI_TP: f64 = 0.004;
pub const ORG_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run490_1_org_rsi_backtest.py)
2. **Walk-forward** (run490_2_org_rsi_wf.py)
3. **Combined** (run490_3_combined.py)

## Out-of-Sample Testing

- OPEN_RANGE_PERIOD sweep: 10 / 15 / 20
- GAP_THRESH sweep: 0.003 / 0.005 / 0.008
- RSI_PERIOD sweep: 10 / 14 / 20
- RSI_OVERSOLD sweep: 30 / 35 / 40
- RSI_OVERBOUGHT sweep: 60 / 65 / 70
