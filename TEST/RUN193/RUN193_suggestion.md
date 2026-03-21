# RUN193 — Mass Index Reversal Bulge: EMA Range Expansion as Trend Change Signal

## Hypothesis

**Mechanism**: The Mass Index analyzes the rate of change in price range (high-low). It uses a 9-period EMA of the high-low range, then a 9-period EMA of that EMA. When the ratio of these two EMAs rises above ~27 and then drops below ~26.5, it forms a "reversal bulge" — the market has exhausted its directional move and is about to reverse. This is not direction-specific: it signals the END of a move, not its direction.

**Why not duplicate**: No prior RUN uses Mass Index. All prior overbought/oversold RUNs use RSI/MFI (direction-specific oscillators). Mass Index is purely a reversal timing tool that doesn't predict direction — it must be combined with other signals for entry direction.

## Proposed Config Changes (config.rs)

```rust
// ── RUN193: Mass Index Reversal Bulge ───────────────────────────────────
// mass_ratio = EMA(EMA(high-low, 9), 9) / EMA(high-low, 9)
// mass_index = sum(mass_ratio, 25 periods)
// Reversal bulge: mass_index > 27.0 then drops below 26.5
// After bulge: use primary direction signal (regime) to determine LONG/SHORT
// The MASS signal provides timing; regime provides direction.

pub const MASS_ENABLED: bool = true;
pub const MASS_FAST: usize = 9;            // fast EMA period (range)
pub const MASS_SLOW: usize = 9;           // slow EMA period (EMA of EMA)
pub const MASS_SUM_PERIOD: usize = 25;     // summation period for index
pub const MASS_BULGE_UPPER: f64 = 27.0;   // bulge formation threshold
pub const MASS_BULGE_LOWER: f64 = 26.5;   // reversal trigger threshold
pub const MASS_SL: f64 = 0.005;
pub const MASS_TP: f64 = 0.004;
pub const MASS_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn mass_index(highs: &[f64], lows: &[f64],
                  fast_ema: usize, slow_ema: usize, sum_period: usize) -> f64 {
    let n = highs.len().min(lows.len());
    if n < sum_period + slow_ema {
        return 0.0;
    }

    let mut ranges = vec![0.0; n];
    for i in 0..n {
        ranges[i] = highs[i] - lows[i];
    }

    let ema_fast = ema(&ranges, fast_ema);
    let ema_slow = ema(&ranges, slow_ema);

    if ema_slow == 0.0 {
        return 0.0;
    }

    let mass_ratio = ema_fast / ema_slow;
    // Simplified: return mass_ratio as proxy for actual mass index calculation
    mass_ratio * (sum_period as f64)
}
```

---

## Validation Method

1. **Historical backtest** (run193_1_mass_backtest.py)
2. **Walk-forward** (run193_2_mass_wf.py)
3. **Combined** (run193_3_combined.py)

## Out-of-Sample Testing

- FAST_EMA sweep: 7 / 9 / 12
- SLOW_EMA sweep: 7 / 9 / 12
- BULGE_UPPER sweep: 25 / 27 / 29
- BULGE_LOWER sweep: 24 / 26.5 / 28
