# RUN27 — Breakout Momentum Rider (Hard Rally / Hard Crash)

## Goal

Every prior RUN (RUN14–RUN26) tested variants of mean-reversion and found OOS WR ≤ 40%. This RUN tests the opposite hypothesis: **ride hard directional moves instead of fading them**.

When the market makes a sudden large move (≥1.5–2.5% over 4h equivalent), with volume confirmation and a strengthening trend (ADX rising), the trade enters *with* the momentum rather than against it.

## Method

- **Data:** 18 coins, 15m 1-year OHLCV
- **Split:** 67% train / 33% OOS test

### Entry — HARD UP (long)
1. 16-bar compounded return ≥ `move_thresh` (≈ 4h candle)
2. `vol[i] ≥ rolling_mean(vol,20) × vol_mult`
3. ADX(14) ≥ `adx_thresh` AND ADX[i] > ADX[i−3] (rising)
4. `close > SMA(50)` (uptrend context)
5. `50 ≤ RSI(14) ≤ 75` (momentum, not yet overbought)

### Entry — HARD DOWN (short): mirror conditions

### Exit
- RSI > 78 (overbought exhaustion) for longs; RSI < 22 for shorts
- close < SMA(20) for longs; close > SMA(20) for shorts

### Stop / Trailing
- ATR(14) × `atr_mult` from entry (dynamic, adapts to volatility)
- Optional: trail at `peak − ATR × trail_atr` after profit ≥ `trail_act`

### Grid (324 combos per coin)
| Parameter | Values |
|-----------|--------|
| move_thresh | 1.0%, 1.5%, 2.0%, 2.5% |
| vol_mult | 1.5×, 2.0×, 2.5× |
| adx_thresh | 20, 25, 30 |
| atr_mult | 0.75, 1.0, 1.5 |
| trail config | none; 0.75×ATR@0.5%; 1.0×ATR@0.8% |

**Scoring on train:** Sharpe × √(trades). **OOS fitness:** independent evaluation.

**Success criteria:**
- WR ≥ 44% with ≥ 20 OOS trades, **OR**
- avg_win / avg_loss ≥ 1.5 AND PF ≥ 1.2 with ≥ 20 OOS trades

**Trade sim:** fee=0.1%/side, slip=0.05%/side.

---

## Results (OOS Test Half — 33% Hold-out)

### Full Per-Coin Table

| Coin | Lt | LWR% | LPF | LP&L% | LBEwr | LAW | LAL | St | SWR% | SPF | SP&L% | SBEwr | SAW | SAL | +? |
|------|----|----|-----|------|------|-----|-----|----|------|-----|------|------|-----|-----|----|
| DASH | 70 | 37.1 | **1.48** | +25.4 | 28.6 | 3.026 | 1.210 | 21 | 38.1 | 1.16 | +2.6 | 34.6 | 2.268 | 1.199 | **YES** |
| UNI  | 15 | 20.0 | 0.41 | -7.1 | 37.9 | 1.655 | 1.009 | 8 | 37.5 | 0.35 | -4.2 | 63.0 | 0.755 | 1.288 | no |
| **NEAR** | **31** | **48.4** | **1.60** | **+8.3** | **36.9** | 1.479 | 0.866 | **26** | **46.2** | **1.84** | **+10.7** | **31.8** | 1.959 | 0.914 | **YES** |
| ADA  | 19 | 36.8 | 0.85 | -1.1 | 40.7 | 0.882 | 0.606 | 10 | 40.0 | 0.95 | -0.3 | 41.3 | 1.225 | 0.861 | no |
| LTC  | 6 | 16.7 | 0.16 | -3.8 | 55.8 | 0.715 | 0.902 | 10 | 40.0 | 0.70 | -1.8 | 48.8 | 1.054 | 1.003 | no |
| SHIB | 19 | 42.1 | 1.42 | +3.6 | 33.9 | 1.513 | 0.776 | 5 | 40.0 | 0.43 | -3.5 | 60.9 | 1.300 | 2.024 | no |
| LINK | 12 | 16.7 | 0.12 | -8.6 | 62.8 | 0.579 | 0.976 | 18 | 38.9 | 0.87 | -1.1 | 42.3 | 1.071 | 0.785 | no |
| ETH  | 2 | 0.0 | 0.00 | -2.1 | 100 | 0.000 | 1.026 | 10 | 40.0 | 2.30 | +5.9 | 22.5 | 2.616 | 0.759 | no |
| DOT  | 11 | 18.2 | 0.64 | -2.0 | 25.6 | 1.765 | 0.609 | 13 | 23.1 | 0.75 | -1.8 | 28.5 | 1.868 | 0.745 | no |
| XRP  | 11 | 36.4 | 0.28 | -3.9 | 67.2 | 0.380 | 0.778 | **16** | **75.0** | **1.89** | **+7.2** | 61.4 | 1.278 | 2.029 | no* |
| ATOM | 22 | 36.4 | 0.32 | -8.1 | 64.3 | 0.471 | 0.849 | 2 | 50.0 | 0.14 | -2.3 | 87.5 | 0.383 | 2.692 | no |
| SOL  | 5 | 40.0 | 1.20 | +0.4 | 35.7 | 1.336 | 0.741 | 17 | 41.2 | 0.62 | -3.8 | 53.1 | 0.890 | 1.005 | no |
| DOGE | 16 | 62.5 | 0.93 | -0.6 | 64.1 | 0.835 | 1.490 | 8 | 50.0 | 0.29 | -5.1 | 77.8 | 0.511 | 1.787 | no |
| **XLM**  | **20** | **50.0** | **1.68** | **+5.1** | **37.3** | 1.260 | 0.750 | 15 | 46.7 | 1.32 | +2.9 | 39.9 | 1.707 | 1.134 | **YES** |
| AVAX | 12 | 58.3 | 0.89 | -0.4 | 61.1 | 0.528 | 0.828 | 5 | 20.0 | 0.04 | -5.7 | 87.0 | 0.221 | 1.475 | no |
| ALGO | 18 | 33.3 | 0.63 | -2.9 | 44.1 | 0.833 | 0.656 | 10 | 50.0 | 1.03 | +0.3 | 49.2 | 1.889 | 1.827 | no |
| BNB  | 2 | 0.0 | 0.00 | -0.9 | 100 | 0.000 | 0.447 | 18 | 27.8 | 0.42 | -8.2 | 48.0 | 1.173 | 1.085 | no |
| BTC  | 18 | 44.4 | 0.58 | -3.3 | 58.1 | 0.569 | 0.790 | 19 | 26.3 | 0.58 | -4.2 | 38.2 | 1.159 | 0.715 | no |

*XRP SHORT: WR=75% PF=1.89 but only 16 trades — below the ≥20 threshold.

**Avg LONG: WR=33.2%, PF=0.733 | Avg SHORT: WR=40.6%, PF=0.870**

### Positive Results Detail

| Coin | Dir | WR% | PF | t | Config |
|------|-----|-----|----|---|--------|
| DASH | LONG | 37.1% | 1.48 | 70 | move≥2.5%, vol≥2.0×, ADX≥20, ATR×1.0, trail 0.75×ATR@0.5% |
| NEAR | LONG | 48.4% | 1.60 | 31 | move≥2.5%, vol≥2.0×, ADX≥20, ATR×0.75, trail 1×ATR@0.8% |
| NEAR | SHORT | 46.2% | 1.84 | 26 | move≥2.0%, vol≥1.5×, ADX≥30, ATR×0.75, trail 1×ATR@0.8% |
| XLM | LONG | 50.0% | 1.68 | 20 | move≥2.0%, vol≥2.5×, ADX≥20, ATR×0.75, trail 1×ATR@0.8% |

---

## Conclusions

### First positive results since RUN17 — but with caveats

**4 coin-direction pairs meet the success criteria**, compared to 0/54 in RUN23 and 0/90 in RUN24. This is the first strategy type since the Monte Carlo validation in RUN17 to produce genuine OOS edge on multiple coins.

### NEAR is the standout: both long AND short positive

NEAR meets the criteria in both directions:
- **NEAR LONG**: 48.4% WR, PF=1.60, 31 trades. Config requires 2.5% hard move + 2× volume spike + ADX≥20 rising.
- **NEAR SHORT**: 46.2% WR, PF=1.84, 26 trades. Config slightly tighter on volume (1.5×) but requires ADX≥30.

NEAR having genuine breakout edge in both directions is consistent with it being a high-beta altcoin — when the market makes a hard move, NEAR amplifies it and continues. This is the opposite of what mean-reversion assumes.

### DASH LONG: structural R:R edge (avg_win/avg_loss = 2.5)

DASH LONG meets the R:R criterion: avg_win=3.026%, avg_loss=1.210%, ratio=2.5 ≥ 1.5 threshold, PF=1.48, 70 trades. The larger trade count (70) provides stronger statistical confidence than the other positives. WR=37.1% is below 44% but the outsized wins compensate — PF=1.48 with 70 OOS trades is meaningful.

Note: DASH also dominates the mean-reversion results across all prior RUNs due to its OuMeanRev structure. The momentum breakout strategy provides a complementary signal type.

### XLM LONG: WR=50% but borderline sample (20 trades)

XLM LONG at 50% WR, PF=1.68 with exactly 20 trades meets the threshold but is on the edge of statistical reliability. The config requires the strictest volume condition (2.5×) which explains the low trade count.

### XRP SHORT: 75% WR is tantalizing but only 16 trades

XRP SHORT produced WR=75%, PF=1.89 — the highest WR in the entire RUN. However 16 trades in 4 months is insufficient to confirm (6 wins out of 8 trades would already give 75% WR). This would need further validation before any COINCLAW changes.

### Why trailing stops work here but not in RUN26

RUN26 showed trailing stops *raise* breakeven WR for mean-reversion (wider stop grows avg_loss faster than trailing grows avg_win). For momentum breakout the dynamic is reversed:
- The 1×ATR trailing stop is placed *further* from current price than the 0.3% mean-rev fixed SL
- When the momentum trade works, it continues 2–3% before the trailing stop tightens
- When it fails, the ATR×0.75 stop keeps initial loss controlled
- Result: avg_win >> avg_loss (NEAR: 2.0% vs 0.9%; DASH: 3.0% vs 1.2%)

This confirms the RUN26 conclusion: trailing stops suit trend-following (momentum), not mean-reversion.

### Short signals are structurally better than long (avg WR 40.6% vs 33.2%)

The average OOS WR for short signals (40.6%) exceeds long signals (33.2%). This makes sense for the test period's market structure: hard crashes in crypto tend to continue (cascading liquidations, leveraged longs unwinding) while hard rallies sometimes reverse (euphoria spikes). Short momentum is more persistent than long momentum on 15m altcoin data.

### Sample size is the primary limitation

The median trade count for positives is ~26 OOS trades over 4 months. With ~26 trades, a 48% WR means ±10pp confidence interval at 90% confidence. This is enough to flag as a promising signal but not enough to commit to a COINCLAW change without further validation.

## Decision

**CONDITIONALLY POSITIVE** — 4/36 coin-direction pairs (NEAR long+short, DASH long, XLM long) meet success criteria. This is the first strategy type to show OOS profitability since RUN17 Monte Carlo validation.

**Recommended next step:** Walk-forward validation on a wider time window (multi-year data if available) or live paper-trading observation before COINCLAW integration. The top candidates are:
1. **NEAR** (both directions, consistent config, clear mechanism)
2. **DASH** (R:R criterion, 70 OOS trades, structural edge)
3. **XLM** (borderline sample, needs confirmation)

No immediate COINCLAW changes — validate sample size first.

## Files

| File | Description |
|------|-------------|
| `run27_results.json` | Per-coin grid search results, OOS evaluation |
| `PLAN_RUN27.md` | Implementation plan (root dir) |
| `RUN27.md` | This file |

Source: `tools/src/run27.rs`
