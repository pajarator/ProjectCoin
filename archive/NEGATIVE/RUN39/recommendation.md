# RUN39 — Asymmetric Win/Loss Cooldown: Recommendation

## Hypothesis
Named: `win_loss_cooldown_asymmetry`

Wins and losses should have different cooldown periods:
- After a **WIN** (non-SL exit): shorter cooldown (1 bar) — market may continue
- After a **LOSS** (SL exit): longer cooldown (3 bars) — regime may have shifted
- Consecutive SLs: faster escalation (4-12 bars) vs ISO-only 60-bar rule

## Results

### RUN39.1 — Grid Search (193 configs × 18 coins, 5-month 15m data)

**POSITIVE on in-sample data.**

| Config | PnL | ΔPnL | WR% | PF | Cascades |
|--------|------|------|-----|----|---------|
| Baseline (W=2,L=2,C2=60,C3=60) | +$5.5 | — | 48.6% | 1.048 | 0 |
| **W2_L5_C2_12_C3_20** (best) | **+$8.5** | **+$2.9** | 48.6% | 1.059 | 0 |
| W1_L5_C2_12_C3_20 | +$8.2 | +$2.7 | 48.5% | 1.063 | 0 |
| W2_L3_C2_12_C3_20 | +$7.8 | +$2.3 | 48.5% | 1.059 | 0 |

**Key finding:** The improvement comes from faster consecutive-loss escalation (C2=12 vs 60), not from asymmetric win/loss cooldowns. WIN_COOLDOWN=1 and WIN_COOLDOWN=2 produce nearly identical results.

### RUN39.2 — Walk-Forward Validation (3 windows, train=2mo, test=1mo)

**NEGATIVE on out-of-sample data.**

| Window | Train Range | Test Range | ΔPnL | Positive Coins |
|--------|-------------|------------|------|----------------|
| 0 | [empty] | [0-2880] | +$0.0 | 0/18 (0%) |
| 1 | [0-2880] | [2880-5760] | +$0.9 | 5/18 (28%) |
| 2 | [0-5760] | [5760-8640] | -$0.3 | 4/18 (22%) |

- **Avg ΔPnL:** +$0.2 (marginal, not significant)
- **Avg positive coins:** 17% (far below 50% threshold)
- **VERDICT:** NEGATIVE — OOS results do not confirm grid search

**Note:** Window 0 had a training bug (empty train set) which distorted the first result.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

The in-sample improvement (+$2.9 P&L delta from grid search) does not survive out-of-sample walk-forward validation. The cooldown parameter changes are overfitting to the training period. Faster consecutive-loss escalation (C2=12 vs 60) appears attractive in-sample but provides no reliable OOS improvement.

The asymmetry hypothesis (win vs loss cooldowns) is not supported — WIN_COOLDOWN has negligible effect on outcomes. The 60-bar ISO escalation rule is not meaningfully improved by reducing it to 12 bars in practice.

## Files
- `run39_1_results.json` — Grid search results (193 configs)
- `run39_2_results.json` — Walk-forward results
- `run39.rs` — Grid search implementation
- `run39_2.rs` — Walk-forward implementation
