# RUN84 — Session-Based Partial Exit Scaling: Recommendation

## Hypothesis
Named: `session_partial_scaling`

Apply session multipliers to partial exit tiers:
- ASIA (UTC 00:00-08:00): scale down tiers
- EUUS (UTC 08:00-16:00): keep or scale up
- US (UTC 16:00-24:00): scale down slightly

## Results

### RUN84.1 — Session Partial Scaling Grid Search (28 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL 27 session-scaled configs reduce PnL vs baseline.**

Note: Baseline here includes partial exits (TIER1=0.4%/20%, TIER2=0.8%/20%). Partial exits change baseline WR from 25.9% → 50.6% and PnL from +$292.85 → +$132.50.

| Config | PnL | ΔPnL | WR% | Trades | PF |
|--------|------|------|-----|--------|-----|
| BASELINE (partial exits) | +$132.50 | — | 50.6% | 13,025 | 1.02 |
| A0.80_U0.90_E1.10 (best) | +$126.27 | -$6.23 | 51.4% | 13,095 | 1.06 |
| A0.60_U0.80_E0.90 (worst) | +$112.91 | -$19.58 | 54.6% | 13,370 | 1.20 |

**Key findings:**
- ALL 27 session-scaled configs produce lower PnL than fixed-tier baseline
- PF improves (1.02 → 1.06-1.20) but PnL decreases — higher WR with lower avg win doesn't help
- Session scaling makes partial exits fire earlier (Asia) or later (EUUS at 1.1×), but net effect is negative
- The session hypothesis (Asia = tighter ranges, EUUS = wider moves) doesn't translate to profitable scaling

**Additional finding:** Partial exits on their own (without session scaling) reduce PnL from +$292.85 → +$132.50 (-55%) despite raising WR from 25.9% → 50.6%. The increased WR comes at the cost of taking profits early and missing larger moves.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run84_1_results.json` — Grid search results
- `coinclaw/src/run84.rs` — Implementation
