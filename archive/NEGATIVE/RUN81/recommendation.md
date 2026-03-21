# RUN81 — Equity Curve Circuit Breaker: Recommendation

## Hypothesis
Named: `equity_circuit_breaker`

Halt regime entries when portfolio drawdown exceeds threshold for sustained bars (10-20 consecutive bars).

Grid: THRESHOLD [0.10, 0.15, 0.20] × BARS [5, 10, 20] × RECOVERY [0.05, 0.10, 0.15]
Total: 3 × 3 × 3 = 27 + baseline = 28 configs

## Results

### RUN81.1 — Equity Curve Circuit Breaker Grid Search (28 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL 27 configs produce IDENTICAL results to baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Breaker% | Blocked |
|--------|------|------|-----|--------|---------|---------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.0% | 0 |
| T+10_B5_R+5 (any) | +$292.85 | $0.00 | 25.9% | 9,716 | 0.0% | 0 |

**All 27 configs: breaker_active_pct = 0.0%, blocked = 0.**

**Key findings:**
- Circuit breaker NEVER activates — portfolio drawdown never reaches 10% from peak
- COINCLAW portfolio is profitable across the entire 5-month backtest (~$100/coin → ~$292/coin)
- Peak equity keeps increasing, so drawdown from peak never triggers any threshold
- The circuit breaker hypothesis assumed the portfolio would experience sustained drawdowns — but COINCLAW only goes up in this backtest period
- Results are identical across all 27 configs because none of them ever activate

**Why it fails:** The circuit breaker is designed for a losing portfolio that suffers sustained drawdowns. COINCLAW over 5 months is profitable (cumulative gains every month), so peak equity monotonically increases. The circuit breaker would need a prolonged losing period (3+ months of consecutive losses) to ever activate, which didn't occur in this dataset.

**Additional context:** This doesn't mean the circuit breaker is a bad idea — it just means the backtest period (Oct 2025 - Feb 2026) was consistently profitable for COINCLAW. In a prolonged bear market or extended drawdown period, the circuit breaker might activate. However, without any activation in 5 months of backtesting, there is no evidence it helps.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run81_1_results.json` — Grid search results
- `coinclaw/src/run81.rs` — Implementation
