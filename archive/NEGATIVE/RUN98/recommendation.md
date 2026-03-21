# RUN98 — Intraday Max Drawdown Clip: Recommendation

## Hypothesis

**Named:** `intraday_dd_clip`

Exit positions when intraday drawdown from daily high exceeds threshold:
- Track `daily_high` and `daily_low` per coin (reset at UTC midnight)
- When intraday DD >= `INTRADAY_DD_CAP`, exit immediately regardless of entry price
- Rationale: catches market-wide selloffs that haven't hit SL but are clearly broken

## Results

### RUN98.1 — Grid Search (5 configs × 18 coins, 5-month 15m data)

**NEGATIVE — All DD clip configs produce lower PnL than baseline.**

| Config | PnL | ΔPnL | WR% | Trades | DDClips |
|--------|------|------|-----|--------|---------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0 |
| DC0.0100 (best) | +$280.27 | -$12.58 | 34.1% | 11,024 | 2,690 |
| DC0.0150 | +$273.90 | -$18.95 | 29.9% | 10,399 | 1,252 |
| DC0.0075 | +$252.99 | -$39.85 | 36.7% | 11,626 | 4,125 |
| DC0.0050 (worst) | +$216.59 | -$76.26 | 40.1% | 12,493 | 6,330 |

**Key findings:**
- Tighter DD caps raise WR (up to 40.1% at 0.5%) but dramatically reduce PnL
- Clipping exits early cuts losses before recovery — at 25.9% WR, the avg_win is already small; clipping further shrinks it
- With asymmetric win ($0.30) vs loss ($1.50), cutting losses at 0.5% DD prevents the occasional big recovery win
- The SL at 0.3% already handles extreme intraday moves

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run98_1_results.json` — Grid search results
- `coinclaw/src/run98.rs` — Implementation
