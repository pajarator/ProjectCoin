# RUN74 — Daily Equity Compounding with Per-Coin Reset: Recommendation

## Hypothesis
Named: `daily_compounding_reset`

Mechanically: Reset per-coin balance to $100 at UTC day/week/month boundaries. Optionally carry accumulated profits.

## Results

### RUN74.1 — Compounding Reset Grid Search (7 configs × portfolio, 5-month 15m data)

**NEGATIVE — All compounding configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| BASELINE (continuous) | +$292.85 | — | 25.9% | 9,716 |
| MTHLY_CARRY (best) | +$24.76 | -$268.09 | 26.0% | 9,751 |
| WKLY_CARRY | +$11.33 | -$281.52 | 26.4% | 9,828 |
| DLY_CARRY | +$0.96 | -$291.89 | 28.9% | 10,439 |

**Key findings:**
- Daily reset is catastrophic: PnL drops 99.7% (from +$293 to +$0.96)
- Monthly reset drops 91.5% — still devastating
- WR improves slightly (25.9% → 28.9% for daily) due to smaller position sizes after reset reducing loss magnitude
- Carry_profits has zero effect — the reset happens after the trade PnL is already credited to bal

**Why it fails:** Resetting balance to $100 forces position sizes to shrink (net = bal × 2% × 5× × exit_pct). After a reset, a $100 balance means $10/trade vs $120 balance = $12/trade before reset. The compounding engine (letting winners run) is destroyed by the constant capital base reset. While WR improves slightly, the PnL impact from reduced position sizing dominates.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run74_1_results.json` — Grid search results
- `coinclaw/src/run74.rs` — Implementation
