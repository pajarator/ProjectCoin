# RUN83 — Cooldown by Market Mode: Recommendation

## Hypothesis
Named: `cooldown_by_mode`

Different cooldown periods based on market mode (LONG/ISO_SHORT/SHORT) and exit quality (good/bad).

## Results

### RUN83.1 — Cooldown by Mode Grid Search (244 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL 243 configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | PF |
|--------|------|------|-----|--------|-----|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.35 |
| LG1_LB4_IG6_IB20_E2 (best) | +$268.89 | -$23.96 | 26.8% | 8,668 | 0.37 |
| LG3_LB8_IG4_IB30_E5 (worst) | +$148.58 | -$144.27 | 27.2% | 5,497 | 0.37 |

**Key findings:**
- ALL 243 configs produce lower PnL than baseline
- Trade count drops 10-43% (baseline=9716, best non-baseline=8668)
- WR improves marginally (+0.5 to +1.3pp) but not enough to compensate for trade reduction
- Longer cooldowns (higher LB, IB, E values) produce worse results
- Escalation multiplier (E) amplifies losses when combined with long bad cooldowns

**Why it fails:** Longer cooldowns reduce trade count, but each trade's expected value (EV = WR × avg_win − (1-WR) × avg_loss) doesn't change. Since cooldowns don't change whether trades win or lose, they only reduce the number of trades. The marginal WR improvement (+0.5 to +1.3pp) is not enough to offset the 10-43% reduction in trade count.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run83_1_results.json` — Grid search results
- `coinclaw/src/run83.rs` — Implementation
