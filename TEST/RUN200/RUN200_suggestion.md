# RUN200 — DMI/ADX Trend Strength Filter: Directional Movement System

## Hypothesis

**Mechanism**: DMI (Directional Movement Index) consists of +DI (positive directional indicator) and -DI (negative directional indicator), plus ADX (Average Directional Index) which measures trend strength regardless of direction. When +DI > -DI AND ADX rising > 25 → strong bullish trend → LONG. When -DI > +DI AND ADX rising > 25 → strong bearish trend → SHORT. ADX falling < 20 → no trend → skip or exit.

**Why not duplicate**: No prior RUN uses DMI or ADX systematically. ADX was mentioned in RUN14 as part of the indicator library but not as a standalone trading system. DMI is distinct because it specifically separates directional from non-directional (range) market conditions.

## Proposed Config Changes (config.rs)

```rust
// ── RUN200: DMI/ADX Trend Strength System ───────────────────────────────
// +DM = max(high - high[prev], 0) if trending up
// -DM = max(low[prev] - low, 0) if trending down
// TR = max(H-L, H-PC, L-PC)
// +DI = 100 × EMA(+DM/TR, period)
// -DI = 100 × EMA(-DM/TR, period)
// ADX = 100 × EMA(|+DI - -DI| / (+DI + -DI), period)
// LONG: +DI > -DI AND ADX > 25 AND ADX rising
// SHORT: -DI > +DI AND ADX > 25 AND ADX rising
// NO TRADE: ADX < 20 (weak trend)

pub const DMI_ENABLED: bool = true;
pub const DMI_PERIOD: usize = 14;           // smoothing period
pub const DMI_ADX_THRESHOLD: f64 = 25.0;   // strong trend threshold
pub const DMI_WEAK_TREND: f64 = 20.0;      // no-trade threshold
pub const DMI_SL: f64 = 0.005;
pub const DMI_TP: f64 = 0.004;
pub const DMI_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn dmi_adx(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> (f64, f64, f64) {
    let n = highs.len().min(lows.len()).min(closes.len());
    if n <= period + 1 {
        return (25.0, 25.0, 25.0);
    }

    let mut plus_dm_sum = 0.0;
    let mut minus_dm_sum = 0.0;
    let mut tr_sum = 0.0;

    for i in 1..n {
        let tr = (highs[i] - lows[i])
            .max((highs[i] - closes[i-1]).abs())
            .max((lows[i] - closes[i-1]).abs());

        let plus_dm = if highs[i] > highs[i-1] { highs[i] - highs[i-1] } else { 0.0 };
        let minus_dm = if lows[i-1] > lows[i] { lows[i-1] - lows[i] } else { 0.0 };

        tr_sum += tr;
        plus_dm_sum += plus_dm;
        minus_dm_sum += minus_dm;
    }

    let plus_di = if tr_sum > 0.0 { 100.0 * plus_dm_sum / tr_sum } else { 0.0 };
    let minus_di = if tr_sum > 0.0 { 100.0 * minus_dm_sum / tr_sum } else { 0.0 };
    let dx = if (plus_di + minus_di) > 0.0 {
        100.0 * (plus_di - minus_di).abs() / (plus_di + minus_di)
    } else {
        0.0
    };

    // ADX is EMA of DX (simplified to DX as proxy)
    let adx = dx;

    (plus_di, minus_di, adx)
}
```

---

## Validation Method

1. **Historical backtest** (run200_1_dmi_backtest.py)
2. **Walk-forward** (run200_2_dmi_wf.py)
3. **Combined** (run200_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21 / 30
- ADX_THRESHOLD sweep: 20 / 25 / 30
- WEAK_TREND sweep: 15 / 20 / 25
