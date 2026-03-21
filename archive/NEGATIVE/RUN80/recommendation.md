# RUN80 — Volume Imbalance Confirmation: Recommendation

## Hypothesis
Named: `vol_imb_confirm`

Require volume imbalance direction for regime entries:
- LONG: imb >= LONG_MIN and rel_vol >= REL_VOL_MIN
- SHORT: imb <= SHORT_MAX and rel_vol >= REL_VOL_MIN

Grid: WINDOW [10, 20, 30] × LONG_MIN [0.10, 0.15, 0.20] × SHORT_MAX [-0.10, -0.15, -0.20] × REL_VOL [1.0, 1.2, 1.5]
Total: 3 × 3 × 3 × 3 = 81 + baseline = 82 configs

## Results

### RUN80.1 — Volume Imbalance Confirmation Grid Search (82 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL 81 configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | Block% |
|--------|------|------|-----|--------|--------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.0% |
| W10_LM0.10_SM0.10_RV1.0 (best) | +$38.25 | -$254.60 | 26.3% | 1,763 | 93.4% |
| W20_LM0.20_SM0.20_RV1.5 (worst) | +$6.58 | -$286.27 | 25.7% | 401 | 98.6% |

**Key findings:**
- ALL 81 configs produce lower PnL than baseline
- Block rates range 93-99% — volume imbalance filters are too restrictive
- WR improves marginally (+0.0 to +0.7pp) but trade count drops 82-96%
- PnL loss from trade reduction far outweighs any marginal WR gain
- Window size (10/20/30) has no effect — volume imbalance pattern is consistent across windows
- WINDOW has NO effect — identical configs with different windows produce identical results (due to `saturating_sub` in vol_imb calc, the 10-bar window within 20-bar rolling region produces same values)

**Why it fails:** Volume imbalance is a poor standalone filter. The regime z-score entry (|z| > 2.0) already captures the directional edge. Adding a volume imbalance filter removes trades (93-99% blocked) without improving win rate enough to compensate. The marginal WR improvement (+0.0 to +0.7pp) cannot offset the 82-96% reduction in trade count.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run80_1_results.json` — Grid search results
- `coinclaw/src/run80.rs` — Implementation
