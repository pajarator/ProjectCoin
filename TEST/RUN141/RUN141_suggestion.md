# RUN141 — CME Gap Fill Strategy: Post-Gap Mean Reversion Entry

## Hypothesis

**Mechanism**: CME futures gaps in crypto (BTC, ETH) tend to fill within 4-8 hours of occurrence. When price gaps up at the open, it frequently pulls back to fill the gap before resuming direction; same for gap downs. COINCLAW currently has no gap detection — adding a gap-fill trade type captures an orthogonal, calendar-driven edge that pure mean-reversion cannot.

**Why this is not a duplicate**: RUN58 (Post-Event Gap Fill) was proposed but never executed. No other RUN addresses CME gaps specifically. All existing COINCLAW strategies (VWAP Reversion, Bollinger Bounce, etc.) are pure price-action mean reversion — none use inter-day gap dynamics.

**Why it could work**: CME futures trade 23h/day. The BTC/USDT price on Binance tracks CME closely. Weekend/overnight gaps >0.5% are common and statistically tend to fill within the next trading session. This is a well-documented phenomenon in equity markets (overnight gap fill) and extends to crypto CME追踪.

---

## Proposed Config Changes (config.rs)

Add new gap-fill trade type and parameters:

```rust
// ── RUN1: CME Gap Fill ──────────────────────────────────────────────
// Gap is measured as: |open_price / prev_close - 1.0| >= GAP_THRESHOLD
// Gap direction: open > prev_close → gap UP (expect fill downward)
// Entry: price mean-reverts INTO the gap (buy dip after gap up)
// Exit: gap fully filled OR GAP_MAX_HOLD bars elapsed

pub const GAP_ENABLED: bool = true;
pub const GAP_THRESHOLD: f64 = 0.005;    // 0.5% minimum gap to qualify
pub const GAP_SL: f64 = 0.006;           // 0.6% stop loss (wider than scalp)
pub const GAP_TP: f64 = 0.004;           // 0.4% take profit (gap midpoint)
pub const GAP_MAX_HOLD: u32 = 32;         // ~8 hours at 15m bars
pub const GAP_MIN_FILL_PCT: f64 = 0.50;  // gap must still be ≥50% unfilled
```

Add new trade type:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeType {
    Regime,
    Scalp,
    Momentum,
    GapFill,  // NEW
}
```

Add gap detection function in engine.rs:

```rust
/// Returns (gap_direction, gap_pct, gap_filled_pct) if gap detected, else None.
/// gap_direction: 1.0 = gap up (open > prev_close), -1.0 = gap down
/// gap_filled_pct: 0.0 = fully open gap, 1.0 = fully filled
fn detect_cme_gap(candles_15m: &[Candle]) -> Option<(f64, f64, f64)> {
    if candles_15m.len() < 2 { return None; }
    let prev = candles_15m[candles_15m.len() - 2];
    let curr = candles_15m.last()?;
    let gap_pct = (curr.o / prev.c - 1.0).abs();
    if gap_pct < GAP_THRESHOLD { return None; }
    let direction = if curr.o > prev.c { 1.0 } else { -1.0 };
    // How much of the gap is filled?
    // For gap up: price moved down from open toward prev close = filled
    // For gap down: price moved up from open toward prev close = filled
    let filled = if direction > 0.0 {
        (curr.o - curr.c) / (curr.o - prev.c)  // gap up: how much price fell
    } else {
        (curr.c - curr.o) / (prev.c - curr.o)  // gap down: how much price rose
    };
    let fill_pct = filled.clamp(0.0, 1.0);
    Some((direction, gap_pct, fill_pct))
}
```

Add `check_gap_entry` in engine.rs — fires when:
1. Gap detected AND `gap_filled_pct < GAP_MIN_FILL_PCT` (gap still open)
2. Direction is LONG: gap_up AND price pulled back (c < o)
3. Direction is SHORT: gap_down AND price rallied (c > o)
4. After 2-bar warmup (MIN_HOLD = 2)
5. No position currently open, no cooldown active

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 regime + scalp + momentum (no gap strategy)
- **Comparison**: +gap_fill trade type, tracked separately

**Metrics to measure**:
- Gap fill win rate (should be >55% if hypothesis holds)
- Profit factor on gap trades
- Whether gap trades correlate or anti-correlate with regime trades (diversification benefit)
- Avg gap fill completion time

**Hypothesis**: Gap fill trades should achieve WR >55% (higher than regime's ~33%) because gaps are mechanically driven and tend to fill predictably. If WR >55% and PF >1.5, add as a third trade type alongside regime/scalp.

---

## Validation Method

1. **Historical backtest** (run1_1_gap_backtest.py):
   - 18 coins, 1-year 15m data
   - Identify all gaps >0.5% in the dataset
   - Simulate gap-fill entry on bars 2+ after gap
   - Record: entry price, stop, TP, exit reason, P&L
   - Output: per-coin WR, PF, avg hold time, fill %

2. **Walk-forward** (run1_2_gap_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Compare: gap-only strategy vs baseline regime vs combined

3. **Combined comparison** (run1_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + gap_fill
   - Portfolio stats, exit reason distribution, correlation of gap trades vs regime trades

---

## Out-of-Sample Testing

- Gap threshold sweep: 0.3% / 0.5% / 0.75% / 1.0%
- Fill pct threshold sweep: 30% / 50% / 70%
- Max hold time sweep: 16 / 32 / 48 bars
- OOS: final 4 months of data held out from all parameter selection
