# RUN201 — Demand Index (DI): Volume-Price Synthesis for Institutional Flow

## Hypothesis

**Mechanism**: The Demand Index = synthetic ratio of volume to price change. When price rises on decreasing volume → divergence (weakness, distribution). When price falls on decreasing volume → divergence (strength, accumulation). High positive DI with rising price = institutional accumulation. Negative DI = institutional distribution. Zero-line crossover is the primary signal.

**Why not duplicate**: No prior RUN uses the Demand Index. All volume-based indicators (MFI, OBV, EOM) are directional. The Demand Index is unique because it specifically identifies *non-confirmed* price moves — situations where price moves but volume doesn't confirm, signaling potential reversal.

## Proposed Config Changes (config.rs)

```rust
// ── RUN201: Demand Index ─────────────────────────────────────────────────
// demand_index = (volume × price_change) / (price_change + volume_offset)
// Simplified: DI = rolling_corr(price, volume) × volume_trend
// Positive DI + price rising = institutional buying
// Negative DI + price falling = institutional selling
// Zero-line crossover as entry signal

pub const DEMAND_INDEX_ENABLED: bool = true;
pub const DEMAND_INDEX_PERIOD: usize = 20;   // smoothing period
pub const DEMAND_INDEX_SL: f64 = 0.005;
pub const DEMAND_INDEX_TP: f64 = 0.004;
pub const DEMAND_INDEX_MAX_HOLD: u32 = 36;
```

---

## Validation Method

1. **Historical backtest** (run201_1_di_backtest.py)
2. **Walk-forward** (run201_2_di_wf.py)
3. **Combined** (run201_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 14 / 20 / 30
