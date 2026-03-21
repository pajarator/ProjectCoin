# RUN303 — MFI Percentile Rank: Volume-Weighted RSI at Historical Extremes

## Hypothesis

**Mechanism**: MFI (Money Flow Index) = volume-weighted RSI. Instead of fixed thresholds (e.g., MFI < 20 = oversold), rank current MFI against its own historical distribution. When MFI rank drops below 10% (historical extreme low) → strong mean-reversion LONG signal. When MFI rank rises above 90% (historical extreme high) → strong mean-reversion SHORT signal. Percentile rank makes the signal adaptive to each coin's own MFI distribution.

**Why not duplicate**: RUN187 uses basic MFI with fixed thresholds. RUN112 uses MFI confirmation. RUN262 uses RSI percentile rank. RUN273 uses StochRSI percentile. No RUN combines MFI with percentile ranking — this makes MFI adaptive to its own historical distribution rather than using fixed overbought/oversold levels.

## Proposed Config Changes (config.rs)

```rust
// ── RUN303: MFI Percentile Rank ─────────────────────────────────────────────
// mfi = money_flow_index(period=14)
// mfi_rank = percentile_rank(mfi, lookback=100)  — 0-100% where 100% = all-time high
// LONG: mfi_rank < MFI_OVERSOLD (extreme selling pressure)
// SHORT: mfi_rank > MFI_OVERBOUGHT (extreme buying pressure)
// Require volume confirmation: avg_volume(20) > base_volume to avoid low-liquidity signals

pub const MFI_RANK_ENABLED: bool = true;
pub const MFI_RANK_PERIOD: usize = 14;
pub const MFI_RANK_LOOKBACK: usize = 100;
pub const MFI_RANK_OVERSOLD: f64 = 10.0;     // bottom 10th percentile for LONG
pub const MFI_RANK_OVERBOUGHT: f64 = 90.0;   // top 10th percentile for SHORT
pub const MFI_RANK_VOL_CONFIRM: bool = true;
pub const MFI_RANK_SL: f64 = 0.005;
pub const MFI_RANK_TP: f64 = 0.004;
pub const MFI_RANK_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run303_1_mfi_rank_backtest.py)
2. **Walk-forward** (run303_2_mfi_rank_wf.py)
3. **Combined** (run303_3_combined.py)

## Out-of-Sample Testing

- OVERSOLD sweep: 5 / 10 / 15
- OVERBOUGHT sweep: 85 / 90 / 95
- LOOKBACK sweep: 50 / 100 / 200
- PERIOD sweep: 10 / 14 / 21
