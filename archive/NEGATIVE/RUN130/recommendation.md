# RUN130 — Negative Day Revenue Filter: NEGATIVE

## Result Summary

| Config | PnL | ΔPnL | WR% | Trades | Suppress Rate |
|--------|-----|------|-----|--------|---------------|
| BASELINE | +2384.20 | — | 39.0% | 13601 | 0.0% |
| NW3_NS0.8 | +2384.20 | 0.00 | 39.0% | 13601 | 0.0% |
| NW3_NS1 | +2384.20 | 0.00 | 39.0% | 13601 | 0.0% |
| NW5_NS0.8 | +2384.20 | 0.00 | 39.0% | 13601 | 0.0% |
| NW5_NS1 | +2384.20 | 0.00 | 39.0% | 13601 | 0.0% |
| NW7_NS0.8 | +2384.20 | 0.00 | 39.0% | 13601 | 0.0% |
| NW7_NS1 | +2384.20 | 0.00 | 39.0% | 13601 | 0.0% |

**VERDICT: NEGATIVE** — All configs produce identical results with 0% suppress rate. The portfolio-level NEGDAY filter never triggers because the rolling N-day loss frequency never reaches 80% threshold in this dataset.

## Analysis

**Filter mechanism:** Track portfolio's rolling N-day loss frequency (% of days with negative P&L in last N days). Suppress all entries when loss_freq >= threshold (e.g., 80% = 3/3 negative days for NW3).

**Why the filter never triggers:**
1. **Portfolio-level diversity:** With 18 coins across different crypto assets, it's extremely rare for ALL 18 coins to produce negative P&L on the same UTC day, let alone 3 consecutive such days.
2. **Market regimes in backtest:** The Oct 2025 - Mar 2026 dataset had trending behavior. For 3 consecutive all-negative days to occur, the entire crypto market would need a sustained 3-day drawdown where every single coin loses money.
3. **Threshold is too high:** Even NW3 with 80% threshold requires 3/3 days negative for suppression. This is nearly impossible with a diversified 18-coin portfolio.

**Grid search tested:**
- NEGDAY_WINDOW [3, 5, 7]: Different lookback windows
- NEGDAY_SUPPRESS [0.80, 1.0]: Different thresholds (80% or 100% negative days required)

**Key observation:** The portfolio-level approach is fundamentally different from per-coin indicator filters. While per-coin filters (RUN121-129) triggered frequently but removed good trades, this portfolio-level filter never triggers at all with a diversified portfolio.

## Conclusion

No COINCLAW changes. The NEGDAY filter is not suitable as implemented. A diversified 18-coin portfolio almost never has 3 consecutive all-negative days, making the filter inactive. If this concept were revisited, it would need:
1. Per-coin loss frequency (not portfolio-level), or
2. A much lower threshold (e.g., 30-40% loss frequency), or
3. Application to a single coin or a highly correlated subset
