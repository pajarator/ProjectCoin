# RUN20 — Momentum Crowding Filter

## Goal

Test whether avoiding entry after strong recent price run-ups improves COINCLAW v13 mean reversion quality. Originally planned as a funding rate / open interest filter — reframed as an OHLCV momentum proxy since no derivatives data was available in cache.

## Background

Funding rate (the periodic payment from longs to shorts in perpetual futures) is a measure of how "crowded" the long side is. High positive funding = overextended longs = higher probability that a mean reversion will be stopped out before completing.

Without funding data, the closest OHLCV proxy is prior 3-day momentum: a big run-up in the last 3 days attracts late buyers who are now trapped when price dips, creating a crowded-long situation that tends to generate continued downward pressure rather than a clean reversion.

## Hypothesis

COINCLAW entries (vwap_rev, bb_bounce, etc.) that follow a strong 3-day price run-up (>+1–3%) fail more often than entries after flat or down periods. Filtering these out improves win rate and reduces drawdown.

## Method

- **Data:** 18 coins, 15m 1-year OHLCV, COINCLAW v13 strategy per coin
- **Split:** 50/50 chronological — evaluated on OOS test half only
- **3-day return:** `close[i] / close[i - 288] - 1` (288 = 96 bars/day × 3)
- **Volume crowding ratio:** 3-day avg volume / 15-day avg volume
- **Trade sim:** SL=0.3%, no TP, fee=0.1%/side, slip=0.05%/side

**Filters:**

| Filter | Rule | Trades Kept |
|--------|------|-------------|
| Baseline | All signals | 100% |
| Mom > +2% | Skip if 3d return > +2% | 25.2% |
| Mom > +1% | Skip if 3d return > +1% | 23.7% |
| Mom > +3% | Skip if 3d return > +3% | 26.6% |
| VolCrowd | Skip if 3d vol / 15d vol > 2.0 | 31.1% |
| Combined | Mom > +2% AND VolCrowd | 24.7% |
| AntiMom < -3% | Skip if 3d return < -3% (falling knife guard) | 17.5% |

## Results (OOS Test Half)

### Portfolio Summary (18-coin average)

| Filter | Avg WR% | Avg P&L% | Avg MaxDD% | Trades Kept |
|--------|---------|---------|-----------|-------------|
| Baseline | 33.6% | -50.84% | 67.85% | 100% |
| Mom > +2% | 34.2% | -46.46% | 56.68% | 25.2% |
| **Mom > +1%** | **34.3%** | **-42.43%** | **53.64%** | **23.7%** |
| Mom > +3% | 34.1% | -50.85% | 59.86% | 26.6% |
| VolCrowd | 33.5% | -63.02% | 69.00% | 31.1% |
| Combined | 34.1% | -48.31% | 57.67% | 24.7% |
| AntiMom < -3% | 31.3% | -60.12% | 64.79% | 17.5% |

### Per-Coin P&L (OOS Test Half)

| Coin | Base | Mom2% | Mom1% | Mom3% | VolCrwd | Combined | AntiMom |
|------|------|-------|-------|-------|---------|----------|---------|
| ADA  | -85.3% | -70.7% | -62.5% | -74.4% | -85.9% | -72.2% | -80.9% |
| ALGO | -25.5% | -22.9% | -24.8% | -23.1% | -25.5% | -22.9% | -19.7% |
| ATOM | -84.5% | -76.6% | -73.8% | -79.5% | -85.6% | -78.2% | -78.3% |
| AVAX | -31.8% | -27.1% | -22.6% | -30.5% | -31.8% | -27.1% | -28.9% |
| BNB  | -85.1% | -70.1% | -65.2% | -77.6% | -85.2% | -70.6% | -83.8% |
| BTC  | -39.7% | -31.0% | -28.4% | -34.3% | -42.4% | -34.0% | -39.8% |
| DASH | +270.8% | +127.1% | +132.4% | +117.5% | +75.2% | +117.1% | +57.5% |
| DOGE | -37.8% | -11.5% | -13.6% | -20.3% | -40.6% | -15.0% | -47.0% |
| DOT  | -75.6% | -63.1% | -58.4% | -67.0% | -78.8% | -66.8% | -73.8% |
| ETH  | -86.8% | -71.7% | -68.0% | -75.0% | -87.4% | -72.8% | -81.6% |
| LINK | -77.6% | -57.0% | -53.3% | -61.6% | -80.7% | -62.9% | -75.3% |
| LTC  | -83.0% | -68.5% | -59.7% | -72.1% | -82.8% | -68.9% | -81.1% |
| NEAR | -67.1% | -53.9% | -49.7% | -59.9% | -75.7% | -54.0% | -67.2% |
| SHIB | -90.1% | -81.7% | -77.9% | -85.0% | -89.9% | -82.4% | -82.8% |
| SOL  | -87.3% | -71.9% | -66.2% | -75.8% | -87.3% | -71.9% | -83.2% |
| UNI  | -77.3% | -55.4% | -46.0% | -59.0% | -78.1% | -56.2% | -78.5% |
| XLM  | -59.8% | -46.8% | -45.4% | -52.1% | -60.0% | -46.8% | -52.9% |
| XRP  | -91.5% | -83.4% | -80.7% | -85.6% | -91.6% | -84.0% | -84.9% |

## Conclusions

### The mechanism is real but insufficient

**Mom > +1% is the best filter** — improves portfolio P&L by 8.4 percentage points (-50.8% → -42.4%) and reduces max drawdown from 67.9% to 53.6%. The WR improvement (33.6% → 34.3%) confirms the hypothesis: entries after large run-ups do fail more often.

**However, 34.3% WR is still 10 points below the 44% breakeven.** No filter makes the strategies profitable in the OOS test half.

### Why the mechanism doesn't rescue the strategies

The filter keeps only 23.7% of signals (Mom>1%). This means entries after 3d returns below +1% are the "better" ones — yet they still lose. The problem is not just crowded-long timing; it's the fundamental TP/SL structure: SL=0.3%, no TP, 33-35% WR in this period.

With an 8:1 loss/win ratio in terms of frequency (65% loss rate), average wins would need to be >6× the average loss for breakeven — which requires much longer-holding reverting entries than the SMA20 crossback / z-score exit provides.

### Volume crowding is counterproductive

VolCrowd filter (-63.0% vs -50.8% baseline) is the worst filter. High-volume sessions apparently include many valid reversions (e.g., panic selling → quick bounce). Excluding high-volume entries removes both noise AND signal, net negative.

### Anti-momentum filter is the worst

AntiMom<-3% (-60.1% P&L, 31.3% WR). Falling knives (3d return < -3%) are actually valid mean reversion setups — the strategy is designed to catch these. Filtering them out removes some of the best entries.

### DASH exception

DASH (mean_rev) returns +270.8% baseline. Mom>1% cuts this to +132.4%. The DASH strategy catches large 3-day momentum events AND their reversals — the momentum filter removes valid DASH trades.

### Implications

1. **The crowding hypothesis is partially confirmed**: post-runup entries have ~0.7% lower WR
2. **Effect size too small to be useful**: 10 points below breakeven cannot be closed by a 0.7% WR improvement
3. **Not recommended for COINCLAW**: removing 76% of signals while remaining unprofitable defeats the purpose
4. **The real fix** is either a higher-quality signal baseline OR a different TP/SL structure (e.g., smaller fixed TP for higher WR)

## Decision

**NEGATIVE** — Momentum/crowding filter reduces losses but does not make strategies profitable. No COINCLAW changes.

Note: Actual funding rate / OI data may produce different (potentially stronger) filtering. This test establishes the OHLCV proxy is insufficient. If derivatives data is fetched via `fetch_derivatives.py`, a re-run with real funding rate would be the proper follow-up.

## Files

| File | Description |
|------|-------------|
| `run20_results.json` | Full per-coin and portfolio results (Rust) |
| `run20_1_derivatives_features.py` | Original Python stub (ML approach, blocked on data) |
| `RUN20.md` | This file |

Source: `tools/src/run20.rs`
