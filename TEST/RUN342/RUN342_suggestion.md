# RUN342 — KST Percentile Rank: Adaptive Momentum Strength

## Hypothesis

**Mechanism**: KST (Know Sure Thing) measures momentum across multiple ROC windows. Instead of fixed thresholds, rank current KST against its own historical distribution. When KST rank is at extreme highs (top 90%) → momentum is historically stretched → expect mean-reversion. When KST rank is at extreme lows (bottom 10%) → momentum historically weak → expect bounce. Percentile rank makes the signal adaptive to each coin's own KST range.

**Why not duplicate**: RUN13 uses KST as a complement signal (raw crossover). This RUN uses KST percentile rank — the percentile framing is distinct. It makes KST adaptive rather than using fixed thresholds.

## Proposed Config Changes (config.rs)

```rust
// ── RUN342: KST Percentile Rank ───────────────────────────────────────────────
// kst = weighted ROC across 4 windows
// kst_rank = percentile_rank(kst, lookback)
// LONG: kst_rank < KST_OVERSOLD (momentum historically weak = expect bounce)
// SHORT: kst_rank > KST_OVERBOUGHT (momentum historically strong = expect reversal)
// Wait for KST to cross its signal line to confirm the turn

pub const KST_PCT_ENABLED: bool = true;
pub const KST_PCT_ROC1: usize = 8;
pub const KST_PCT_ROC2: usize = 16;
pub const KST_PCT_ROC3: usize = 24;
pub const KST_PCT_ROC4: usize = 32;
pub const KST_PCT_SIGNAL: usize = 8;
pub const KST_PCT_LOOKBACK: usize = 100;
pub const KST_PCT_OVERSOLD: f64 = 15.0;    // bottom 15th percentile
pub const KST_PCT_OVERBOUGHT: f64 = 85.0;  // top 85th percentile
pub const KST_PCT_SL: f64 = 0.005;
pub const KST_PCT_TP: f64 = 0.004;
pub const KST_PCT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run342_1_kst_pct_backtest.py)
2. **Walk-forward** (run342_2_kst_pct_wf.py)
3. **Combined** (run342_3_combined.py)

## Out-of-Sample Testing

- OVERSOLD sweep: 10 / 15 / 20
- OVERBOUGHT sweep: 80 / 85 / 90
- LOOKBACK sweep: 50 / 100 / 200
