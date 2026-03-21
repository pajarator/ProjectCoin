# RUN165 — ADR% Volatility Band Breakout: Intraday Range Expansion as Scalping Signal

## Hypothesis

**Mechanism**: When price breaks above the ADR% upper band (price > adr_lo + range * 0.80) with volume confirmation, it's a volatility expansion — momentum continuation trade. When it rejects from the band, it's a mean-reversion. COINCLAW uses ADR for regime but not for intraday volatility bands. Adding ADR% bands on the 15m chart provides breakout and reversal signals at the intraday timeframe.

**Why not duplicate**: RUN46 (Partial Reversion Signal Exit) uses Z-score for exits. RUN110 (BB Width Compression Entry) uses BB compression. RUN124 (Choppiness Index) confirms range-bound state. None use ADR% bands for both breakout and mean-reversion on the same timeframe.

## Proposed Config Changes (config.rs)

```rust
// ── RUN165: ADR% Volatility Band Breakout ──────────────────────────────
// ADR% upper band = adr_lo + (adr_hi - adr_lo) * ADR_BAND_PCT
// Break above upper band + vol > 1.5x → momentum LONG
// Rejection from upper band → mean-reversion SHORT
// Same for lower band

pub const ADRBAND_ENABLED: bool = true;
pub const ADRBAND_PCT: f64 = 0.80;       // 80% of range from adr_lo
pub const ADRBAND_VOL_MULT: f64 = 1.5;    // volume must be 1.5x average
pub const ADRBAND_SL: f64 = 0.004;        // 0.4% stop
pub const ADRBAND_TP: f64 = 0.003;        // 0.3% take profit
pub const ADRBAND_MAX_HOLD: u32 = 12;      // ~3 hours
```

Add entry logic in `engine.rs`:

```rust
fn check_adrband_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::ADRBAND_ENABLED { return None; }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }
    if ind.adr_hi.is_nan() || ind.adr_lo.is_nan() { return None; }

    let range = ind.adr_hi - ind.adr_lo;
    if range <= 0.0 { return None; }

    let vol_r = ind.vol / ind.vol_ma;
    if vol_r < config::ADRBAND_VOL_MULT { return None; }

    let upper_band = ind.adr_lo + range * config::ADRBAND_PCT;
    let lower_band = ind.adr_hi - range * config::ADRBAND_PCT;

    // Break above upper band → momentum LONG
    if ind.p > upper_band && ind.p > ind.sma20 {
        return Some((Direction::Long, "adrband_break"));
    }
    // Rejection from upper band → SHORT
    if ind.p < upper_band && ind.p > upper_band * 0.98 {
        return Some((Direction::Short, "adrband_reject"));
    }
    // Break below lower band → momentum SHORT
    if ind.p < lower_band && ind.p < ind.sma20 {
        return Some((Direction::Short, "adrband_break"));
    }
    // Rejection from lower band → LONG
    if ind.p > lower_band && ind.p < lower_band * 1.02 {
        return Some((Direction::Long, "adrband_reject"));
    }
    None
}
```

---

## Validation Method

1. **Historical backtest** (run165_1_adrband_backtest.py): 18 coins, sweep ADRBAND_PCT
2. **Walk-forward** (run165_2_adrband_wf.py): 3-window walk-forward
3. **Combined** (run165_3_combined.py): vs baseline

## Out-of-Sample Testing

- PCT sweep: 0.70 / 0.80 / 0.90
- VOL_MULT sweep: 1.2 / 1.5 / 2.0
