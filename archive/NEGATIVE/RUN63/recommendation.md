# RUN63 — BTC Trend Confirmation for Regime Entries: Recommendation

## Hypothesis
Named: `btc_trend_regime_filter`

Block regime LONG entries when BTC is in a confirmed short-term downtrend (SMA9 < SMA20 AND 16-bar return < threshold), and vice versa for SHORT.

## Results

### RUN63.1 — BTC Trend Grid Search (7 configs × 17 altcoins, 5-month 15m data)

**NEGATIVE — All BTC trend filter configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Block% |
|--------|------|------|-----|--------|--------|
| DISABLED (baseline) | +$172.51 | — | 54.4% | 14,251 | 0% |
| RET0.015_SMAT (best) | +$117.34 | -$55.16 | 54.1% | 11,920 | 16% |
| RET0.005_SMAF (worst) | +$65.49 | -$107.02 | 54.1% | 7,226 | 49% |

**Key findings:**
- ALL 6 BTC trend configs produce lower PnL than baseline
- BTC trend filter removes ~16-49% of entries depending on threshold
- WR barely changes (+/- 0.3pp) — BTC trend is not a useful directional filter
- Blocking BTC-downtrend entries removes both winners and losers proportionally
- The BTC trend signal does not predict altcoin regime trade success

**Why it fails:** BTC's short-term trend does not systematically predict altcoin mean-reversion success or failure. The filter removes trades without improving win rate, resulting in net PnL loss proportional to trades removed.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

BTC short-term trend filtering does not improve regime entry quality for altcoins. Trade count reduction dominates any marginal WR improvement.

## Files
- `run63_1_results.json` — Grid search results
- `coinclaw/src/run63.rs` — Implementation
