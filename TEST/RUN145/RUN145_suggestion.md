# RUN145 — Intraday Volatility Cycle: Compression-Expansion Position Scaling

## Hypothesis

**Mechanism**: Crypto markets exhibit predictable intraday volatility cycles. When 4h ATR is below its 20-bar percentile (volatility compression), the market is coiled — a directional breakout is imminent. COINCLAW currently treats all regime entries equally regardless of where we are in the volatility cycle. This RUN adds a `VOL_CYCLE_MULTIPLIER` that scales position size up during compression and down during expansion.

**Why this is not a duplicate**: RUN42 (Dynamic Leverage) scales leverage by volatility but doesn't distinguish compression from expansion phases. RUN54 (Volatility Regime Entry Filter) blocks entries during high-vol environments but doesn't rescale size during compression. RUN72 (Scalp Choppy Mode) suppresses scalp only. None add a continuous, rolling percentile-based position scaler that modulates risk dynamically based on volatility cycle phase.

**Why it could work**: Low-vol compression phases precede the highest-quality mean-reversion setups (tight ranges, predictable reversions). By increasing position size during these phases, we amplify the edge on our highest-quality trades. Conversely, entries during volatility expansion (post-breakout) are lower-quality — reducing size limits losses. If avg trade quality (measured by WR × avg_win / avg_loss) improves >15% during compressed-phase entries, this is a direct P&L improvement.

---

## Proposed Config Changes (config.rs)

```rust
// ── RUN5: Intraday Volatility Cycle Position Scaling ─────────────────
// Phase 1: compute 4h ATR percentile over rolling 20-bar window
// Phase 2: if percentile < 25th pct → COMPRESSED (increase risk)
// Phase 3: if percentile > 75th pct → EXPANDED (decrease risk)
// Applied to: RISK for regime and scalp entries (not momentum)

pub const VOL_CYCLE_ENABLED: bool = true;
pub const VOL_CYCLE_WINDOW: usize = 20;              // rolling 20-bar window
pub const VOL_CYCLE_COMPRESS_PCT: f64 = 0.25;        // < 25th pct = compressed
pub const VOL_CYCLE_EXPAND_PCT: f64 = 0.75;         // > 75th pct = expanded
pub const VOL_CYCLE_COMPRESS_MULT: f64 = 1.5;       // scale up 1.5x during compression
pub const VOL_CYCLE_EXPAND_MULT: f64 = 0.5;         // scale down 0.5x during expansion
```

Add to `Ind15m` in `indicators.rs`:
```rust
pub atr14_percentile: f64,   // current ATR(14) rank over VOL_CYCLE_WINDOW bars (0.0-1.0)
pub atr4h: f64,              // 4-bar (4h) ATR(14) for intraday cycle
pub atr4h_percentile: f64,   // atr4h rank over rolling window
```

Add calculation in `indicators.rs`:
```rust
/// Returns percentile rank of current atr4h over its rolling window
fn calc_atr_percentile(candles: &[Candle], window: usize) -> f64 {
    if candles.len() < window + 1 { return 0.5; }  // default to mid-rank
    let current_atr = calculate_atr14(&candles[candles.len()-1]);
    let mut past_atrs: Vec<f64> = (1..window)
        .filter_map(|i| candles.len().checked_sub(i))
        .map(|i| calculate_atr14(&candles[i]))
        .collect();
    past_atrs.push(current_atr);
    past_atrs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let rank = past_atrs.iter().position(|x| *x >= current_atr).unwrap_or(0);
    rank as f64 / past_atrs.len() as f64
}
```

Modify `open_position` in engine.rs to compute effective risk:
```rust
fn effective_risk(trade_type: TradeType, state: &SharedState, ci: usize) -> f64 {
    let base = match trade_type {
        TradeType::Regime | TradeType::Momentum => config::RISK,
        TradeType::Scalp => config::SCALP_RISK,
        TradeType::CoinSpecific | TradeType::VolCycle => config::RISK,
        _ => config::RISK,
    };
    if !config::VOL_CYCLE_ENABLED { return base; }

    let ind = state.coins[ci].ind_15m.as_ref()?;
    if ind.atr4h_percentile.is_nan() { return base; }

    let pct = ind.atr4h_percentile;
    if pct <= config::VOL_CYCLE_COMPRESS_PCT {
        base * config::VOL_CYCLE_COMPRESS_MULT
    } else if pct >= config::VOL_CYCLE_EXPAND_PCT {
        base * config::VOL_CYCLE_EXPAND_MULT
    } else {
        base  // middle zone: no scaling
    }
}
```

Note: This RUN adds a position-size scaler, not a new trade type. All existing trade types (Regime, Scalp, Momentum, CoinSpecific) get their risk scaled by the vol-cycle multiplier.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 with fixed RISK=0.10
- **Comparison**: same trades, same entries, but with vol-cycle scaled position size

**Metrics to measure**:
- Portfolio-level Sharpe ratio improvement
- Avg trade quality (WR × avg_win / avg_loss) in compressed vs expanded phases
- Overall P&L improvement (% change)
- Whether compression-phase entries actually have higher WR than expansion-phase

**Hypothesis**: Compressed-phase entries (atr4h_percentile < 0.25) should have WR >5pp higher than expansion-phase entries, justifying the 1.5x size increase. Overall portfolio Sharpe should improve >15%.

---

## Validation Method

1. **Historical backtest** (run5_1_volcycle_backtest.py):
   - 18 coins, 1-year 15m data
   - Simulate COINCLAW v16 with fixed risk
   - Re-simulate with vol-cycle scaling applied to every entry
   - Record: phase (compressed/expanded/normal), position size used, P&L
   - Output: per-phase WR, PF, avg P&L; overall portfolio comparison

2. **Walk-forward** (run5_2_volcycle_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep VOL_CYCLE_COMPRESS_MULT: 1.25 / 1.5 / 2.0
   - Sweep VOL_CYCLE_EXPAND_MULT: 0.5 / 0.75

3. **Combined comparison** (run5_3_combined.py):
   - Side-by-side: COINCLAW v16 (fixed risk) vs COINCLAW v16 + vol_cycle_scaled
   - Portfolio stats, per-phase breakdown, Sharpe comparison

---

## Out-of-Sample Testing

- COMPRESS_MULT sweep: 1.25 / 1.5 / 2.0
- EXPAND_MULT sweep: 0.5 / 0.75
- Threshold sweep (COMPRESS_PCT): 0.20 / 0.25 / 0.30
- OOS: final 4 months held out from all parameter selection
