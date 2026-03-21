# RUN13 — STRAT1000 Final Sweep (Complementary Signals)

## Goal

Test the 9 remaining STRAT1000 strategies not covered in RUN11. Key insight: instead of replacing existing strategies, find complementary signals that fire at different times than the primary long entry, capturing oversold conditions the primary misses.

## Strategies Tested (11 entry functions)

| Category | Strategy | Description |
|----------|----------|-------------|
| Candle | pin_bar, hammer, engulfing | Wick/body ratio patterns |
| Oscillator | qqe, laguerre_rsi, mass_index, kst_cross | Alternative oversold detection |
| Trend | dema_tema, parabolic_sar, ichimoku | Adaptive MA / trend-following |
| Adaptive | kalman_filter | State-space model mean reversion |
| Control | ctrl_mean_rev | z < -1.5 baseline |

108 configs (27 base x 4 z-filters) x 7 exit modes x 19 coins = 14,364 backtests.

## Results

### RUN13.1 — Grid Search
Most strategies failed standalone (same finding as RUN11). ctrl_mean_rev remained dominant.

### RUN13.2 — Walk-Forward (Standard)
All new strategies failed walk-forward as standalone replacements.

### Key Pivot: Overlap Analysis
Instead of standalone comparison, analyzed which strategies fire at **different times** than ctrl_mean_rev. Found 3 strategies with >50% unique signals and positive edge:

| Strategy | Unique WR | Unique PF | % Unique |
|----------|-----------|-----------|----------|
| laguerre_rsi | 55.8% | 1.37 | 62% |
| kalman_filter | 53.9% | 1.54 | 71% |
| kst_cross | 52.6% | 1.22 | 58% |

### Walk-Forward on Unique Signals
14/19 coins PASS with unique complement signals. 3 coins excluded (DOGE=WORSE, TRX/XRP=NEUTRAL).

### RUN13.3 — Comparison (Baseline vs Baseline+Complement)

| COIN | BASE P&L | +COMP P&L | Comp Trades | Comp WR | Comp PF | Delta |
|------|----------|-----------|-------------|---------|---------|-------|
| ADA | -0.93% | +0.53% | 77 | 61.0% | 1.76 | +1.45% |
| ALGO | -3.47% | -2.73% | 15 | 66.7% | 3.93 | +0.73% |
| ATOM | -3.47% | -3.05% | 28 | 57.1% | 1.73 | +0.42% |
| AVAX | -3.46% | -2.48% | 36 | 66.7% | 1.93 | +0.98% |
| BNB | -5.18% | -5.09% | 5 | 80.0% | 2.58 | +0.09% |
| BTC | -6.71% | -6.46% | 26 | 53.8% | 1.41 | +0.25% |
| DASH | +4.64% | +6.28% | 34 | 61.8% | 2.50 | +1.64% |
| DOGE | -3.81% | -4.35% | 30 | 30.0% | 0.51 | -0.53% |
| DOT | -1.49% | -1.40% | 14 | 57.1% | 1.33 | +0.09% |
| ETH | -4.24% | -3.85% | 48 | 64.6% | 1.56 | +0.40% |
| LINK | -0.39% | +1.42% | 49 | 69.4% | 3.18 | +1.81% |
| LTC | -2.04% | -1.14% | 66 | 69.7% | 2.27 | +0.90% |
| NEAR | +0.90% | +1.43% | 33 | 57.6% | 1.91 | +0.53% |
| SHIB | -2.87% | -2.02% | 25 | 80.0% | 4.50 | +0.84% |
| SOL | -2.03% | +0.38% | 71 | 71.8% | 3.26 | +2.41% |
| TRX | -9.07% | -9.08% | 5 | 40.0% | 0.96 | -0.00% |
| UNI | -1.17% | +0.42% | 22 | 63.6% | 3.97 | +1.60% |
| XLM | -4.01% | -3.32% | 21 | 71.4% | 2.85 | +0.68% |
| XRP | -3.29% | -3.41% | 35 | 45.7% | 1.03 | -0.12% |
| **TOTAL** | **-52.09%** | **-37.91%** | **640** | **62.7%** | | **+14.18%** |

**16/19 coins improve. 640 complement-only trades at 62.7% WR.**

## Per-Coin Complement Assignments (applied to COINCLAW v13)

| Coin | Complement Strategy | Z-Filter |
|------|-------------------|----------|
| ADA | LaguerreRsi(γ=0.5) | -1.0 |
| ALGO | LaguerreRsi(γ=0.8) | -1.5 |
| ATOM | LaguerreRsi(γ=0.6) | -1.5 |
| AVAX | KalmanFilter(Q=0.0001) | -1.5 |
| BNB | KstCross | -1.0 |
| BTC | KstCross | -0.5 |
| DASH | KstCross | -0.5 |
| DOGE | None (WORSE) | - |
| DOT | LaguerreRsi(γ=0.7) | -1.5 |
| ETH | KalmanFilter(Q=0.0001) | -1.5 |
| LINK | KalmanFilter(Q=0.0001) | -1.5 |
| LTC | KalmanFilter(Q=0.0001) | -1.5 |
| NEAR | LaguerreRsi(γ=0.8) | -0.5 |
| SHIB | LaguerreRsi(γ=0.8) | -1.0 |
| SOL | KalmanFilter(Q=0.0001) | -1.5 |
| TRX | None (NEUTRAL) | - |
| UNI | KalmanFilter(Q=0.0001) | -1.5 |
| XLM | LaguerreRsi(γ=0.6) | -1.5 |
| XRP | None (NEUTRAL) | - |

## Conclusion

Complementary signals add +14.2% to portfolio P&L by capturing oversold conditions the primary strategy misses. The three complement types (Laguerre RSI, Kalman Filter, KST Cross) are mathematically distinct from each other and from z-score mean reversion, ensuring diverse signal sources.

Applied to COINCLAW v13: complement_entry() tries after primary long_entry() fails, before ISO short fallback. Same exit rules (SL 0.3%, SMA20 cross, z-score reversion).
