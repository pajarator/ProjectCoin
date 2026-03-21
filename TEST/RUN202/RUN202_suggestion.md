# RUN202 — Choppiness Index (CI): Strategy Selector for Trendy vs Range-Bound Markets

## Hypothesis

**Mechanism**: The Choppiness Index = 100 × log(sum(ATR, period) / (max(HH, period) - min(LL, period))) / log(period). CI > 61.8 = choppy market (mean-reversion strategies win). CI < 38.2 = trending market (momentum strategies win). Use CI as a meta-filter: switch between mean-reversion and momentum strategies based on CI regime.

**Why not duplicate**: No prior RUN adapts strategy selection based on market choppiness. All prior RUNs use static strategy assignments. CI is a meta-regime indicator that determines *which* strategy type to use at any given time.

## Proposed Config Changes (config.rs)

```rust
// ── RUN202: Choppiness Index Strategy Selector ──────────────────────────
// CI > 61.8 → choppy → favor mean-reversion (current COINCLAW long/short regime)
// CI < 38.2 → trending → favor momentum breakout (momentum layer)
// CI 38.2-61.8 → neutral → current COINCLAW regime trades normally

pub const CI_ENABLED: bool = true;
pub const CI_PERIOD: usize = 14;              // ATR/lookback period
pub const CI_CHOPPY_THRESH: f64 = 61.8;        // above this = choppy
pub const CI_TRENDY_THRESH: f64 = 38.2;        // below this = trendy
pub const CI_LOOKBACK: usize = 14;             // bars for HH/LL
```

Modify engine.rs to check CI before entry and favor momentum when CI < 38.2.

---

## Validation Method

1. **Historical backtest** (run202_1_ci_backtest.py)
2. **Walk-forward** (run202_2_ci_wf.py)
3. **Combined** (run202_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- CHOPPY_THRESH sweep: 60 / 61.8 / 63
- TRENDY_THRESH sweep: 36 / 38.2 / 40
