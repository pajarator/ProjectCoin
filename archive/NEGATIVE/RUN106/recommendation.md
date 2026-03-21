# RUN106 — Hourly Scalp Cooldown: Recommendation

## Hypothesis

**Named:** `hourly_scalp_cooldown`

Scale scalp cooldown by UTC session: longer during quiet Asia session (UTC 00-08), shorter during volatile US session (UTC 13-20).

## Results

### RUN106.1 — Grid Search (13 configs × 18 coins, 5-month 1m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| BASELINE | +$295.20 | — | 20.6% | 12,753 |
| AS2_US1_LT2 | +$295.20 | +$0.00 | 20.6% | 12,753 |
| AS3_US1_LT2 | +$285.17 | -$10.03 | 20.6% | 12,399 |
| AS4_US1_LT2 | +$278.17 | -$17.03 | 20.5% | 12,218 |

**Key findings:**
- Session-aware cooldown provides ZERO improvement over fixed 2-bar baseline
- US=1 vs US=2 makes zero difference (opportunity frequency unchanged)
- ASIA=3 or 4 cooldowns reduce trades and hurt PnL (over-filtering)
- Crypto is 24/7 — no true "session" effect like equities have
- The scalp entry signal frequency (F6 filter) is the real limiting factor, not cooldown

## Conclusion

**NEGATIVE — No session effect in crypto markets.** The 24/7 nature of crypto eliminates the session-based opportunity differences that work in equities. The fixed 2-bar cooldown is already optimal.

## Files
- `run106_1_results.json` — Grid search results
- `coinclaw/src/run106.rs` — Implementation
