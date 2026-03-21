# RUN243 — VWAP Standard Deviation Bands: Historical Extension/Contraction

## Hypothesis

**Mechanism**: VWAP = volume-weighted average price since session start. Compute the rolling standard deviation of the price-VWAP spread over N bars. Create bands at VWAP ± 1σ, 2σ, 3σ. When price reaches 2σ+ bands → historically extended (95th+ percentile). When price reaches 3σ → extreme extension. Mean reversion to VWAP is likely from these extremes.

**Why not duplicate**: No prior RUN uses VWAP standard deviation bands. All prior VWAP RUNs use VWAP crossover. VWAP + stddev bands is a distinct mean-reversion system based on the *distribution* of price around VWAP.

## Proposed Config Changes (config.rs)

```rust
// ── RUN243: VWAP Standard Deviation Bands ────────────────────────────────
// vwap = session VWAP (reset each session)
// stddev = rolling std of (close - vwap) over period
// upper_band_2σ = vwap + 2 × stddev
// lower_band_2σ = vwap - 2 × stddev
// LONG: price touches lower_band_2σ (2 stddev below fair value)
// SHORT: price touches upper_band_2σ (2 stddev above fair value)

pub const VWAP_STD_ENABLED: bool = true;
pub const VWAP_STD_PERIOD: usize = 20;      // rolling stddev period
pub const VWAP_STD_ENTRY: f64 = 2.0;        // entry threshold in stddev units
pub const VWAP_STD_SL: f64 = 0.005;
pub const VWAP_STD_TP: f64 = 0.004;
pub const VWAP_STD_MAX_HOLD: u32 = 36;
```

---

## Validation Method

1. **Historical backtest** (run243_1_vwap_std_backtest.py)
2. **Walk-forward** (run243_2_vwap_std_wf.py)
3. **Combined** (run243_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
- ENTRY sweep: 1.5 / 2.0 / 2.5 / 3.0
