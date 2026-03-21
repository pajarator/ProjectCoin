# RUN198 — Vortex Indicator (VI): Upward vs Downward Movement Trend Reversal

## Hypothesis

**Mechanism**: The Vortex Indicator separates upward price movement (VM+) from downward movement (VM-) over N periods. VM+ = |high - low[prev]|, VM-d = |low - high[prev]|. VI+ (positive trend) above VI- (negative trend) = bullish. When VI+ crosses above VI- → LONG. When VI- crosses above VI+ → SHORT. It was designed specifically to identify trend reversals.

**Why not duplicate**: No prior RUN uses the Vortex Indicator. All prior directional indicators use EMA crosses, RSI, or MACD. VI is unique because it directly compares consecutive bar movements (high-low vs low-high), making it sensitive to the actual price oscillation pattern.

## Proposed Config Changes (config.rs)

```rust
// ── RUN198: Vortex Indicator (VI) ─────────────────────────────────────
// VM+ = |high - low[prev]|
// VM- = |low - high[prev]|
// VI+ = sum(VM+) / sum(TR) over period
// VI- = sum(VM-) / sum(TR) over period
// TR = max(H-L, H-PC, L-PC) where PC = prior close
// LONG: VI+ crosses above VI-
// SHORT: VI- crosses above VI+

pub const VORTEX_ENABLED: bool = true;
pub const VORTEX_PERIOD: usize = 14;       // lookback period
pub const VORTEX_SL: f64 = 0.005;
pub const VORTEX_TP: f64 = 0.004;
pub const VORTEX_MAX_HOLD: u32 = 48;
```

Add in `indicators.rs`:

```rust
pub fn vortex(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> (f64, f64) {
    let n = highs.len().min(lows.len()).min(closes.len());
    if n <= period + 1 {
        return (0.0, 0.0);
    }

    let mut vm_plus_sum = 0.0;
    let mut vm_minus_sum = 0.0;
    let mut tr_sum = 0.0;

    for i in 1..n {
        let tr = (highs[i] - lows[i])
            .max((highs[i] - closes[i-1]).abs())
            .max((lows[i] - closes[i-1]).abs());

        let vm_plus = (highs[i] - lows[i-1]).abs();
        let vm_minus = (lows[i] - highs[i-1]).abs();

        tr_sum += tr;
        vm_plus_sum += vm_plus;
        vm_minus_sum += vm_minus;
    }

    let vi_plus = if tr_sum > 0.0 { vm_plus_sum / tr_sum } else { 0.0 };
    let vi_minus = if tr_sum > 0.0 { vm_minus_sum / tr_sum } else { 0.0 };

    (vi_plus, vi_minus)
}
```

---

## Validation Method

1. **Historical backtest** (run198_1_vortex_backtest.py)
2. **Walk-forward** (run198_2_vortex_wf.py)
3. **Combined** (run198_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21 / 30
