# RUN308 — PPO Histogram Divergence: Normalized MACD Divergence

## Hypothesis

**Mechanism**: PPO (Percentage Price Oscillator) = (EMA12 - EMA26) / EMA26 × 100. Unlike MACD (absolute values), PPO is normalized — the histogram value is a percentage of the EMA. This makes divergences more comparable across coins with different price levels. When price makes a new high but PPO histogram makes a lower high → bearish divergence → SHORT. When price makes a new low but PPO histogram makes a higher low → bullish divergence → LONG.

**Why not duplicate**: RUN249 uses MACD histogram ROC. RUN277 uses MACD histogram slope. RUN289 uses MACD zero-line rejection. RUN203 uses VW-MACD. No RUN uses PPO specifically — the normalization is the key distinction. PPO divergence is more reliable than MACD divergence for cross-coin comparison because it's scale-invariant.

## Proposed Config Changes (config.rs)

```rust
// ── RUN308: PPO Histogram Divergence ─────────────────────────────────────────
// ppo = (EMA(fast) - EMA(slow)) / EMA(slow) * 100
// ppo_signal = EMA(ppo, signal_period)
// ppo_hist = ppo - ppo_signal
// LONG: price.new_low AND ppo_hist.higher_low (bullish divergence)
// SHORT: price.new_high AND ppo_hist.lower_high (bearish divergence)
// Confluence: require PPO histogram also crossing its signal line

pub const PPO_DIVERG_ENABLED: bool = true;
pub const PPO_FAST: usize = 12;
pub const PPO_SLOW: usize = 26;
pub const PPO_SIGNAL: usize = 9;
pub const PPO_DIVERG_LOOKBACK: usize = 20;   // bars to look back for swing high/low
pub const PPO_SL: f64 = 0.005;
pub const PPO_TP: f64 = 0.004;
pub const PPO_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run308_1_ppo_div_backtest.py)
2. **Walk-forward** (run308_2_ppo_div_wf.py)
3. **Combined** (run308_3_combined.py)

## Out-of-Sample Testing

- FAST sweep: 8 / 12 / 16
- SLOW sweep: 20 / 26 / 34
- SIGNAL sweep: 6 / 9 / 12
- LOOKBACK sweep: 14 / 20 / 30
