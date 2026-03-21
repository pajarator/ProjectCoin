# RUN231 — Random Walk Index (RWI): Trending vs Random Walk Market Classifier

## Hypothesis

**Mechanism**: RWI = |close - close[N bars ago]| / (ATR(N) × sqrt(N)). It compares actual price movement to what would be expected from a random walk. When RWI > 1 → market is trending (trade with trend). When RWI < 1 → random walk (use mean-reversion). Trade in the direction of the higher RWI.

**Why not duplicate**: No prior RUN uses Random Walk Index. All prior trend/range RUNs use ADX, Choppiness Index, or Bollinger Bandwidth. RWI is mathematically distinct — it directly compares actual price movement to the random walk null hypothesis.

## Proposed Config Changes (config.rs)

```rust
// ── RUN231: Random Walk Index (RWI) ─────────────────────────────────────
// rwi_high[N] = |high - close[N]| / (ATR(N) × sqrt(N))
// rwi_low[N] = |close - low[N]| / (ATR(N) × sqrt(N))
// RWI = max(rwi_high, rwi_low)
// RWI > 1.0 → trending, RWI < 1.0 → random walk
// LONG: rwi_low > rwi_high AND rwi_low > 1.0
// SHORT: rwi_high > rwi_low AND rwi_high > 1.0

pub const RWI_ENABLED: bool = true;
pub const RWI_PERIOD: usize = 14;            // lookback period
pub const RWI_THRESHOLD: f64 = 1.0;          // trending threshold
pub const RWI_SL: f64 = 0.005;
pub const RWI_TP: f64 = 0.004;
pub const RWI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run231_1_rwi_backtest.py)
2. **Walk-forward** (run231_2_rwi_wf.py)
3. **Combined** (run231_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- THRESHOLD sweep: 0.8 / 1.0 / 1.2
