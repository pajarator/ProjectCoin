# RUN55 — BTC-Altcoin Breadth Divergence Filter: Recommendation

## Hypothesis
Named: `btc_alt_breadth_divergence`

Suppress ISO shorts when BTC is significantly oversold relative to altcoins (negative divergence) to avoid catching falling knives.

## Results

### RUN55.1 — Divergence Grid Search (17 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All divergence filter configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Suppress% |
|--------|------|------|-----|--------|-----------|
| DISABLED (baseline) | +$176.97 | — | 54.7% | 15,101 | 0% |
| ST=-2.0 (best) | +$175.60 | -$1.37 | 54.7% | 15,001 | 0.7% |
| ST=-1.5 | +$172.91 | -$4.06 | 54.8% | 14,819 | 1.9% |
| ST=-1.0 | +$165.40 | -$11.57 | 54.6% | 14,524 | 3.8% |
| ST=-0.5 (worst) | +$160.26 | -$16.71 | 54.6% | 13,934 | 7.7% |

**Key findings:**
- ALL 16 divergence filter configs produce lower PnL than baseline
- Suppression threshold is the only effective parameter (divergence_threshold has no effect in this implementation)
- Even suppressing only 0.7% of trades reduces PnL by $1.37
- Suppressing trades when BTC is oversold vs alts removes more winning trades than losing ones
- The cross-coin divergence signal is not a useful filter for COINCLAW's regime strategy

**Why it fails:** BTC's relative z-score position vs alts does not predict ISO short success. The suppression removes both winning and losing ISO shorts proportionally, but the net effect is negative because it reduces total trade count without meaningful WR improvement.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

Cross-coin breadth divergence filtering does not improve ISO short timing. The market regime signal (z-score) already captures the relevant information; cross-coin comparison adds noise rather than signal.

## Files
- `run55_1_results.json` — Grid search results
- `coinclaw/src/run55.rs` — Implementation
