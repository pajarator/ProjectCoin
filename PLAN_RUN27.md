# PLAN_RUN27 — Breakout Momentum Rider

## Hypothesis

Every prior RUN (RUN14–RUN26) tested variants of the same assumption:
**buy oversold, exit at mean**. All produced OOS WR ≤ 40% on the test half.
The COINCLAW strategy structure may simply not work when the market is
trending hard — because mean-reversion entries keep getting stopped out
by continuation moves.

RUN27 tests the opposite: **ride the hard move, don't fight it**.

When the market makes a sudden large directional move with volume
confirmation and trend structure, the trade goes *with* the breakout
rather than fading it. The expected WR is lower (~45–55%), but with
a better avg_win / avg_loss ratio (momentum trades can run 1–3% before
exhausting, while losers are cut at ≈0.5× ATR).

---

## Signal Definition

### Entry Conditions — HARD UP (long)

All of the following must be true at bar `i`:

| # | Condition | Rationale |
|---|-----------|-----------|
| 1 | `sum(close[i-15..i] / close[i-16..i-1] - 1) > +1.5%` | 4h-equivalent candle up >1.5% (16×15m compounded) |
| 2 | `vol[i] > rolling_mean(vol, 20)[i] × 2.0` | Volume spike — institutional participation |
| 3 | `ADX(14)[i] > 25` | Trend has sufficient strength |
| 4 | `ADX(14)[i] > ADX(14)[i-3]` | ADX is rising (trend accelerating) |
| 5 | `close[i] > SMA(50)[i]` | Price above 50-bar SMA (uptrend confirmed) |
| 6 | `50 ≤ RSI(14)[i] ≤ 75` | Momentum present but not yet overbought |

### Entry Conditions — HARD DOWN (short)

Mirror conditions:
- 4h equivalent candle down <−1.5%
- Volume spike ×2.0
- ADX(14) > 25 AND rising
- `close[i] < SMA(50)[i]`
- `25 ≤ RSI(14)[i] ≤ 50`

### Exit Conditions

| Condition | Long exit | Short exit |
|-----------|-----------|------------|
| Momentum exhausted | RSI(14) > 78 | RSI(14) < 22 |
| Trend reversal | close < SMA(20) | close > SMA(20) |
| Stop loss | ATR(14) × 1.0 from entry (dynamic) | same |
| Optional trail | Trail at peak − ATR(14) × 0.75 after +0.8% profit | peak + ATR × 0.75 after −0.8% |

The ATR-based stop is used here — *not* 0.3% fixed — because momentum
trades operate in a different volatility regime and need room to breathe
before the continuation fires.

---

## What Makes This Different From All Prior RUNs

| Property | COINCLAW mean-rev (RUN1–26) | RUN27 momentum |
|----------|----------------------------|----------------|
| Market regime | Any (typically works in ranging) | Requires hard directional move |
| Entry trigger | Oversold indicator | Breakout + volume confirmation |
| Expected WR | ~33% OOS (empirical) | Target 45–55% |
| Expected R:R | avg_win ≈ avg_loss (bad) | avg_win > avg_loss (trend continuation) |
| Stop type | Fixed 0.3% SL | ATR-based ×1.0 |
| Exit type | Mean reversion (SMA cross, z-score) | Momentum exhaustion (RSI extreme, SMA(20)) |
| Signal frequency | High (entries on any oversold dip) | Low (only on confirmed hard moves) |
| COINCLAW overlap | Cannot coexist (different regime) | Run as separate overlay or replacement for specific coins |

The key theoretical advantage: RUN26 showed trailing stops *hurt* mean-rev
because tight trails chop wins. For momentum, the opposite is true — the
trade has directional energy so a 0.75×ATR trail from the peak captures
the bulk of the move while cutting fast reversals.

---

## Variant Grid to Test

### Hard move threshold: `[+1.0%, +1.5%, +2.0%, +2.5%]`
Controls how "hard" the move must be. Lower = more signals, lower quality.
Higher = fewer signals, potentially cleaner.

### Volume multiplier: `[1.5×, 2.0×, 2.5×]`
Volume spike required. Lower = more signals.

### ADX threshold: `[20, 25, 30]`
Trend strength gate.

### ATR stop mult: `[0.75, 1.0, 1.5]`
Width of the initial stop loss.

### Trailing params: `[(none), (trail=0.75×ATR, act=0.5%), (trail=1.0×ATR, act=0.8%)]`

**Total combos:** 4 × 3 × 3 × 3 × 3 = 324 per coin. Fine for Rust/Rayon.

---

## Implementation Plan

### Data
18 coins, 15m 1-year OHLCV from `data_cache/`. Same loader as all prior RUNs.

### Indicators needed (all in `tools/src/indicators.rs`)
- `sma(close, 20)`, `sma(close, 50)` — already available
- `rsi(close, 14)` — already available
- `atr(high, low, close, 14)` — already available
- `rolling_mean(vol, 20)` — already available
- ADX(14): need to implement OR approximate using existing `atr` + directional movement
  - DM+[i] = max(high[i]-high[i-1], 0) if > max(low[i-1]-low[i], 0) else 0
  - DM-[i] = max(low[i-1]-low[i], 0) if > max(high[i]-high[i-1], 0) else 0
  - Use Wilder's RMA (already in use for ATR)
  - ADX = 100 × RMA(|DI+ - DI-| / (DI+ + DI+), 14)
- 4h return proxy: rolling 16-bar product of 1-bar returns (pure close prices)

### ADX implementation note
ADX is needed in run22.rs already as a signal feature. However it was computed
inline there. For run27, implement `adx(high, low, close, period) -> Vec<f64>`
as a proper function in `indicators.rs` for reuse.

### Rust file: `tools/src/run27.rs`

**Structure:**
```rust
fn sig_breakout_long(close, high, low, vol, params) -> (Vec<bool>, Vec<bool>)
fn sig_breakout_short(close, high, low, vol, params) -> (Vec<bool>, Vec<bool>)
fn sim_momentum(close, entry, exit, atr, atr_stop_mult, trail_mult, trail_act) -> Stats
fn process_coin(coin) -> Value  // grid search train, eval OOS
pub fn run(shutdown)             // parallel 18 coins
```

**Split:** 67% train / 33% OOS (consistent with all prior RUNs).

**Fitness on train:** `sharpe × sqrt(trades)` (penalizes few-trade overfitting).

**OOS report per coin:**
- Best long config: trades, WR%, PF, avg_win, avg_loss, breakeven WR, P&L%
- Best short config: same
- Combined (long + short signals merged): same
- Baseline comparison: COINCLAW v13 primary strategy on same OOS period

### Coin scope
Test all 18 COINCLAW coins. Long strategy applies to all 18. Short strategy
applies to coins where COINCLAW already uses short entries (BNB, BTC, etc.)
or on ALL 18 as a standalone test (for completeness).

---

## Success Criteria

A config is **POSITIVE** for a coin if on the OOS test half:
- WR ≥ 44% with ≥ 20 trades (breakeven threshold), **OR**
- avg_win / avg_loss ≥ 1.5 with PF ≥ 1.2 with ≥ 20 trades (structural edge)

Portfolio-level **POSITIVE** if ≥ 6/18 coins meet either criterion.

If positive: the winning config per coin gets encoded in `coinclaw/src/main.rs`
as a new `momentum_rider` strategy alongside (not replacing) the existing
mean-reversion strategies. Applied when the breakout signal fires; mean-rev
signals remain active the rest of the time.

---

## What Could Go Wrong

1. **Signal frequency too low**: Hard moves (>1.5%, 2× vol, ADX>25) may fire
   only 5–15 times per coin per year → insufficient OOS sample to validate.
   Mitigation: test lower thresholds in the grid.

2. **False breakouts**: Crypto has frequent pump-and-dump patterns where a
   hard move reverses within 1-2 candles (the "fake breakout"). The ADX
   rising condition and volume gate are intended to filter these, but OOS
   may still show many reversals.
   Mitigation: require ADX > 25 AND rising over 3+ bars; add 2-bar confirmation.

3. **Correlation with COINCLAW shorting mode**: In SHORT market mode (≥50%
   coins down), COINCLAW already shorts. The HARD DOWN signal may overlap.
   Resolution: run27 is tested as a standalone — whether to merge with
   existing mode logic is decided post-result.

4. **Data survivorship in OOS half**: The test half (H2 of year) may have a
   specific macro regime (e.g., sideways consolidation) where breakouts are
   rare and tend to fail. This is the same regime risk as all prior RUNs.

---

## Files to Create

| File | Description |
|------|-------------|
| `tools/src/run27.rs` | Rust implementation |
| `archive/RUN27/run27_results.json` | Per-coin grid search results |
| `archive/RUN27/RUN27.md` | Results and conclusions |

Register in `tools/src/main.rs`: `mod run27;` + `"run27" => run27::run(shutdown)`.

---

## Notes on ADX Implementation

RMA (Wilder's Smoothing): `rma[i] = rma[i-1] × (1 - 1/period) + value[i] × (1/period)`

```
tr[i]  = max(high[i]-low[i], |high[i]-close[i-1]|, |low[i]-close[i-1]|)
dm_p[i] = if high[i]-high[i-1] > low[i-1]-low[i] and high[i]-high[i-1] > 0
              then high[i]-high[i-1] else 0
dm_n[i] = if low[i-1]-low[i] > high[i]-high[i-1] and low[i-1]-low[i] > 0
              then low[i-1]-low[i] else 0

atr14   = RMA(tr, 14)
di_p    = 100 × RMA(dm_p, 14) / atr14
di_n    = 100 × RMA(dm_n, 14) / atr14
dx      = 100 × |di_p - di_n| / (di_p + di_n)
adx     = RMA(dx, 14)
```

This is the Wilder standard ADX used in TradingView and COINCLAW run22.
