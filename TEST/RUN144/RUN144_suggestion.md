# RUN144 — Volume Imbalance Mean Reversion: Tick-Wweighted Price Alignment

## Hypothesis

**Mechanism**: In each 15m bar, volume is split between up-bars (close > open) and down-bars (close < open). The **True Volume Weight** (TVWAP) — volume-weighted average price using only bars where price went up vs down — reveals directional conviction. When price is below VWAP but TVWAP is above VWAP (buyers are winning on volume but price hasn't caught up yet), it signals hidden accumulation. The inverse applies for distribution. This is a volume-profile concept that COINCLAW's existing indicators don't capture.

**Why this is not a duplicate**: RUN80 (Volume Imbalance Confirmation) uses OBV direction. RUN50 (Candle Composition) uses body-to-range ratio. RUN112 (MFI Confirmation) uses HLC-based money flow. RUN129 (VWAP Deviation Percentile) measures distance from VWAP without distinguishing volume direction. TVWAP imbalance is mechanically distinct — it isolates the price action of volume-heavy bars.

**Why it could work**: Volume is the only leading indicator in market microstructure. When aggressive buyers are consistently putting volume on up-bars but price lags, there's pent-up buying pressure. The signal fires when TVWAP and price disagree, with price lagging — a pure mean-reversion setup on a volume-weighted basis. If WR >55%, it's a clean, orthogonal signal.

---

## Proposed Config Changes (config.rs)

```rust
// ── RUN4: Volume Imbalance Mean Reversion ──────────────────────────
// TVWAP = sum(vol_i * price_i for up-bars) / sum(vol_i for up-bars)
// Imbalance = (TVWAP - VWAP) / VWAP
// Entry: imbalance exceeds threshold AND price agrees direction
// Exit: imbalance closes OR MAX_HOLD bars

pub const VOL_IMBALANCE_ENABLED: bool = true;
pub const VOL_IMBALANCE_THRESHOLD: f64 = 0.002;    // 0.2% TVWAP-VWAP deviation
pub const VOL_IMBALANCE_WINDOW: usize = 8;          // look back 8 bars for TVWAP
pub const VOL_IMBALANCE_SL: f64 = 0.004;           // 0.4% stop
pub const VOL_IMBALANCE_TP: f64 = 0.003;          // 0.3% take profit
pub const VOL_IMBALANCE_MAX_HOLD: u32 = 16;       // ~4 hours at 15m bars
```

Add to indicators.rs a new `Ind15m` field:
```rust
pub tvwap_deviation: f64,    // (TVWAP - VWAP) / VWAP, positive = up-volume dominant
pub tvwap_deviation_prev: f64,  // previous bar for crossover detection
```

Add TVWAP calculation:
```rust
/// Calculate True Volume Weighted Average Price (TVWAP) deviation from VWAP
/// Returns positive if up-bar volume dominates (bullish imbalance), negative if down-bar volume dominates
fn calc_tvwap_deviation(candles: &[Candle], window: usize, vwap: f64) -> f64 {
    if candles.len() < window || vwap <= 0.0 { return f64::NAN; }
    let window_candles = &candles[candles.len() - window..];
    let mut up_vol_sum = 0.0;
    let mut up_price_vol_sum = 0.0;
    for c in window_candles {
        if c.c > c.o {
            up_vol_sum += c.v;
            up_price_vol_sum += c.v * (c.h + c.l) / 2.0;  // typical price
        }
    }
    if up_vol_sum == 0.0 { return f64::NAN; }
    let tvwap = up_price_vol_sum / up_vol_sum;
    (tvwap - vwap) / vwap
}
```

Add entry logic in engine.rs:
```rust
/// Fires when TVWAP deviation exceeds threshold and crosses zero (was <0, now >0 = bullish entry)
/// LONG: tvwap_deviation crosses above 0 while > THRESHOLD (up-volume emerging)
/// SHORT: tvwap_deviation crosses below 0 while < -THRESHOLD (down-volume emerging)
fn check_vol_imbalance_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }
    if ind.tvwap_deviation.is_nan() || ind.tvwap_deviation_prev.is_nan() { return None; }

    let thresh = config::VOL_IMBALANCE_THRESHOLD;
    let dev = ind.tvwap_deviation;
    let prev = ind.tvwap_deviation_prev;

    // LONG: bullish crossover of threshold
    if prev < thresh && dev >= thresh && dev > 0.0 {
        return Some((Direction::Long, "vol_imbalance"));
    }
    // SHORT: bearish crossover below -threshold
    if prev > -thresh && dev <= -thresh && dev < 0.0 {
        return Some((Direction::Short, "vol_imbalance"));
    }
    None
}
```

Integration: Call from `check_entry` after regime and before momentum.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (no volume imbalance layer)
- **Comparison**: vol_imbalance trades tracked separately

**Metrics to measure**:
- Vol imbalance WR (hypothesis: >55%)
- PF on vol imbalance trades
- Correlation with regime and scalp trades
- Does TVWAP deviation lead price? (check if deviation peaks before price reversal)

**Hypothesis**: Volume-imbalance crossover should achieve WR >55% because it measures directional conviction, not just price level. If confirmed, it's a clean orthogonal signal with high interpretability.

---

## Validation Method

1. **Historical backtest** (run4_1_volimb_backtest.py):
   - 18 coins, 1-year 15m data
   - Identify all TVWAP deviation crossovers
   - Simulate entry on crossover bar
   - Record: deviation magnitude, direction, entry price, stop, TP, exit reason, P&L
   - Output: per-coin WR, PF, avg hold time

2. **Walk-forward** (run4_2_volimb_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep VOL_IMBALANCE_THRESHOLD: 0.001 / 0.002 / 0.003 / 0.004
   - Sweep VOL_IMBALANCE_WINDOW: 4 / 8 / 16 bars

3. **Combined comparison** (run4_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + vol_imbalance
   - Portfolio stats, trade type correlation matrix

---

## Out-of-Sample Testing

- Threshold sweep: 0.001 / 0.002 / 0.003 / 0.004
- Window sweep: 4 / 8 / 16 bars
- OOS: final 4 months held out from all parameter selection
