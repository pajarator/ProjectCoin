# RUN363 — RSI Gap Momentum: Gap Detection and Fill Strategy

## Hypothesis

**Mechanism**: Intraday gaps (price opening significantly above or below the prior close) tend to fill — the price closes the gap back to the prior close. Measure the "gap size" as a percentage of the prior close. When gap > GAP_THRESH% and price moves against the gap direction → the fill is likely. LONG when there's a downside gap and price starts recovering. SHORT when there's an upside gap and price starts declining. Volume confirms whether the fill will materialize.

**Why not duplicate**: No prior RUN uses gap detection. Gaps are distinct from regular price moves because they represent overnight sentiment changes. Gap fill strategies are a well-known pattern that haven't been tested in COINCLAW yet.

## Proposed Config Changes (config.rs)

```rust
// ── RUN363: RSI Gap Momentum ─────────────────────────────────────────────────
// gap_size = (open - prior_close) / prior_close * 100
// downside_gap = gap_size < -GAP_THRESH (price gapped down)
// upside_gap = gap_size > GAP_THRESH (price gapped up)
// LONG: downside_gap AND price recovered above open AND RSI recovering
// SHORT: upside_gap AND price fell below open AND RSI declining
// Exit: price reaches prior_close (gap filled)

pub const RSI_GAP_ENABLED: bool = true;
pub const RSI_GAP_GAP_THRESH: f64 = 0.5;    // 0.5% = significant gap
pub const RSI_GAP_RSI_PERIOD: usize = 14;
pub const RSI_GAP_RSI_RECOVER_THRESH: f64 = 45.0;  // RSI must be recovering above this
pub const RSI_GAP_SL: f64 = 0.005;
pub const RSI_GAP_TP: f64 = 0.004;
pub const RSI_GAP_MAX_HOLD: u32 = 24;
```

---

## Validation Method

1. **Historical backtest** (run363_1_rsi_gap_backtest.py)
2. **Walk-forward** (run363_2_rsi_gap_wf.py)
3. **Combined** (run363_3_combined.py)

## Out-of-Sample Testing

- GAP_THRESH sweep: 0.3 / 0.5 / 0.8
- RSI_RECOVER_THRESH sweep: 40 / 45 / 50
