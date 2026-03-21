# RUN78 — Cross-Coin Z-Score Confirmation: Recommendation

## Hypothesis
Named: `cross_coin_z_confirm`

Mechanically: Require BTC z >= MIN_LONG for LONG, BTC z <= -MAX_SHORT for SHORT, block all if |btc_z| < SUPPRESS.

## Results

### RUN78.1 — Cross-Coin Z-Confirm Grid Search (28 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All configs catastrophically worse than baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Block% |
|--------|------|------|-----|--------|--------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.0% |
| ML0.5_MS0.5_SUP1.0 (best) | +$10.92 | -$281.93 | 21.3% | 394 | 98.7% |
| ML0.5_MS0.5_SUP2.0 (worst) | +$2.63 | -$290.22 | 17.6% | 51 | 99.8% |

**Key findings:**
- ALL 27 configs block 98-99.8% of entries — catastrophically over-filtering
- BTC z-score is between -1 and +1 ~95%+ of the time (it's mean-reverting around 0)
- The SUPPRESS condition `|btc_z| < threshold` blocks almost ALL bars
- Even the least restrictive config (SUP=1.0) blocks 98.7% of entries
- Remaining trades (1-2%) are in extreme BTC z conditions — too small a sample

**Why it fails:** BTC's z-score is mean-reverting, spending most of its time near 0. Using it as a regime-entry gate blocks almost all entries. The few that pass are in extreme non-representative market conditions.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run78_1_results.json` — Grid search results
- `coinclaw/src/run78.rs` — Implementation
