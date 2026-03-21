# RUN72 — Scalp Choppy Mode: Recommendation

## Hypothesis
Named: `scalp_choppy_mode`

Suppress scalp entries when market-wide ATR is below threshold for sustained period, to avoid scalp losses during low-volatility choppy markets.

## Results

### RUN72.1 — Choppy Mode Grid Search (13 configs, portfolio-level 1m backtest)

**NEGATIVE — All choppy configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Choppy% | Blocked |
|--------|------|------|-----|--------|---------|---------|
| BASELINE | +$1177.66 | — | 23.8% | 22,251 | 0.0% | 0 |
| AT0.0010_CB4 (least harmful) | +$1176.90 | -$0.76 | 23.9% | 22,203 | 0.3% | 59 |
| AT0.0015_CB4 | +$990.64 | -$187.02 | 24.2% | 20,244 | 13.0% | 2,612 |
| AT0.0020_CB4 | +$629.04 | -$548.62 | 24.9% | 16,013 | 36.1% | 7,770 |
| AT0.0025_CB4 (worst) | +$381.18 | -$796.49 | 25.4% | 12,206 | 53.3% | 12,131 |

**Key findings:**
- ALL choppy configs produce lower PnL than baseline
- WR improves only +0.1-1.6pp while trade count drops significantly (0-53%)
- The choppy detection (ATR threshold) is binary — AT0.0010 barely triggers (0.3% choppy), higher thresholds trigger more
- Trade count reduction dominates marginal WR improvement

**Why it fails:** Choppy mode suppresses scalp trades during low-volatility periods, but WR only improves slightly (+0.1-1.6pp). The blocked trades represent scalp opportunities that are, on average, still profitable enough that removing them hurts more than it helps.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run72_1_results.json` — Grid search results
- `coinclaw/src/run72.rs` — Implementation