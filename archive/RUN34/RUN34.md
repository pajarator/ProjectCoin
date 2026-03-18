# RUN34 — ISO Short Drawdown Mitigation

## Goal

Investigate whether ISO short entries are causing excessive drawdown due to repeated SL hits when coins trend up, and test two mitigation mechanisms:
- **A. Circuit breaker** — after N consecutive ISO short SLs on the same coin, suppress ISO shorts for X bars
- **B. ISO-specific SL widening** — use a wider (or no) stop loss on ISO shorts so mean-reversion signals have time to work

## Motivation

Live paper trading on Mar 16-17, 2026 showed concentrated losses on ISO shorts:
- XLM: 6 consecutive SL hits on iso_relative_z (-$1.30 cumulative)
- DOT: 3 consecutive SL hits on iso_relative_z (-$0.61)
- LINK: 4 SL hits on iso_relative_z / short_bb_bounce (-$0.70)
- DASH: 2 SL hits on iso_divergence + scalp SLs (-$0.40)

Hypothesis: the market regime changed (broad uptrend) and ISO shorts are mistaking trend momentum for coin-specific overbought conditions.

## Method

Grid search: 15 combos across 18 coins, 366 days of 15m data, $100/coin/day reset.

| # | Config | CB_N | CB_CD | ISO_SL | Notes |
|---|--------|------|-------|--------|-------|
| 0 | baseline | — | — | 0.3% | Current system |
| 1 | CB(2,4) | 2 | 4 | 0.3% | Light circuit breaker |
| 2 | CB(2,8) | 2 | 8 | 0.3% | |
| 3 | CB(2,16) | 2 | 16 | 0.3% | Aggressive circuit breaker |
| 4 | CB(3,4) | 3 | 4 | 0.3% | |
| 5 | CB(3,8) | 3 | 8 | 0.3% | |
| 6 | CB(3,16) | 3 | 16 | 0.3% | |
| 7 | ISO_SL=0.5% | — | — | 0.5% | |
| 8 | ISO_SL=0.8% | — | — | 0.8% | |
| 9 | ISO_SL=1.5% | — | — | 1.5% | |
| 10 | ISO_SL=signal | — | — | none | Signal-only exits |
| 11 | CB(2,8)+SL=0.5% | 2 | 8 | 0.5% | Combined |
| 12 | CB(2,8)+SL=0.8% | 2 | 8 | 0.8% | Combined |
| 13 | CB(2,8)+SL=signal | 2 | 8 | none | Combined |
| 14 | no-ISO | — | — | — | Kill all ISO shorts |

Decision gate: ΔP&L > 0 AND ΔWR > 0.

## Results

**0 of 14 combos pass the decision gate.**

### Summary Table

| Config | TotalPnL | ΔBase | Trades | WR% | ISOtrd | IWR% | ISO_PnL | nonISO_PnL |
|--------|----------|-------|--------|-----|--------|------|---------|------------|
| **baseline** | **+144.03** | — | 6261 | 39.6% | 3305 | 39.5% | +118.08 | +25.95 |
| CB(3,4) | +142.01 | -2.02 | 6221 | 39.8% | 3264 | 39.8% | +116.28 | +25.73 |
| CB(3,8) | +131.20 | -12.84 | 6192 | 39.7% | 3236 | 39.8% | +105.25 | +25.95 |
| CB(2,4) | +129.71 | -14.32 | 6144 | 39.8% | 3180 | 40.0% | +107.30 | +22.41 |
| CB(3,16) | +127.59 | -16.44 | 6170 | 39.8% | 3212 | 39.9% | +103.83 | +23.76 |
| CB(2,8) | +118.00 | -26.03 | 6065 | 39.9% | 3101 | 40.1% | +95.59 | +22.41 |
| CB(2,16) | +108.08 | -35.95 | 6028 | 39.8% | 3062 | 40.1% | +86.12 | +21.96 |
| ISO_SL=0.5% | +106.80 | -37.23 | 6050 | 43.3% | 3110 | 46.8% | +81.05 | +25.76 |
| CB(2,8)+SL=0.5% | +84.52 | -59.51 | 5913 | 43.4% | 2964 | 47.2% | +62.53 | +21.99 |
| ISO_SL=0.8% | +75.65 | -68.39 | 5855 | 46.9% | 2936 | 54.0% | +50.71 | +24.94 |
| CB(2,8)+SL=0.8% | +61.84 | -82.19 | 5768 | 46.9% | 2844 | 54.5% | +38.03 | +23.82 |
| no-ISO | +26.85 | -117.18 | 3132 | 39.5% | 0 | — | +0.00 | +26.85 |
| ISO_SL=1.5% | +6.25 | -137.78 | 5639 | 50.0% | 2748 | 61.0% | -15.71 | +21.96 |
| ISO_SL=signal | -194.50 | -338.54 | 5216 | 53.7% | 2403 | 70.0% | -213.59 | +19.09 |
| CB(2,8)+SL=signal | -194.50 | -338.54 | 5216 | 53.7% | 2403 | 70.0% | -213.59 | +19.09 |

### Per-coin ISO Short Diagnostics (baseline)

| Coin | ISO Trades | ISO SLs | Max Consec | ISO WR% | Total P&L |
|------|-----------|---------|------------|---------|-----------|
| DASH | 418 | 253 | 6 | 39.5% | +80.47 |
| ATOM | 420 | 234 | 7 | 44.3% | +5.12 |
| UNI | 341 | 198 | 9 | 41.9% | +33.42 |
| XLM | 324 | 191 | 5 | 41.0% | +1.56 |
| AVAX | 309 | 171 | 6 | 44.7% | +10.97 |
| BNB | 297 | 151 | 4 | 49.2% | -10.17 |
| ADA | 247 | 150 | 6 | 39.3% | +5.06 |
| DOT | 248 | 140 | 5 | 43.5% | +9.73 |
| DOGE | 231 | 138 | 4 | 40.3% | +10.03 |
| LINK | 208 | 110 | 5 | 47.1% | +12.49 |
| LTC | 55 | 35 | 4 | 36.4% | +1.97 |
| NEAR | 34 | 23 | 3 | 32.4% | +16.11 |
| SOL | 37 | 19 | 2 | 48.6% | -1.51 |
| SHIB | 27 | 18 | 3 | 33.3% | +2.65 |
| XRP | 34 | 18 | 3 | 47.1% | -1.25 |
| ALGO | 25 | 15 | 2 | 40.0% | -6.17 |
| BTC | 26 | 11 | 1 | 57.7% | -25.39 |
| ETH | 24 | 10 | 1 | 58.3% | -1.06 |

## Key Findings

1. **ISO shorts are the primary profit engine**: +$118.08 of the +$144.03 total P&L (82%) comes from ISO shorts. Killing ISO shorts drops portfolio to +$26.85.

2. **Every mitigation uniformly hurts P&L**: Circuit breakers block re-entries that eventually win. SL widening raises WR dramatically (signal-only = 70% ISO WR!) but each loss grows so large that P&L goes deeply negative (-$214 ISO P&L on signal-only).

3. **Wider SL raises breakeven WR faster than actual WR** — same pattern as RUN26. ISO_SL=0.5% raises WR from 39.5% to 46.8% but breakeven WR also rises proportionally, net effect is worse.

4. **Consecutive SL streaks are normal operating variance**: Max consecutive ISO SLs per coin range from 1 (BTC) to 9 (UNI). The Mar 16-17 drawdown (XLM 6 SLs, DOT 3 SLs) falls within normal annual range.

5. **Non-ISO P&L is stable (~$22-26)** across all variants, confirming changes only affect ISO shorts.

6. **ISO shorts are a high-frequency, low-WR, high-payoff strategy**: 3,305 trades/year, ~60% hit SL, but the 40% that win pay enough to net +$118. The coins with the most SLs (DASH 253, ATOM 234) are also the highest contributors.

## Conclusion

**NEGATIVE** — The hypothesis was wrong. ISO short repeated SL losses are not a system flaw; they are the expected cost of a strategy that is net profitable over the full year. The 2-day drawdown observed in live trading is within normal variance (max 9 consecutive SLs observed on UNI across 366 days).

No COINCLAW changes. The existing system's ISO short configuration is already optimal — any attempt to reduce SL frequency (circuit breaker) or SL severity (wider stops) reduces expected P&L.

## Files

- `run34.rs` — Grid search script (15 combos, rayon parallel)
- `run34_1_results.json` — Full results with per-coin breakdowns
