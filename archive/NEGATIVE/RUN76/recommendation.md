# RUN76 — Volatility-Adaptive Stop Loss: Recommendation

## Hypothesis
Named: `vol_adaptive_stop`

Mechanically: Scale SL inversely with ATR percentile rank: `SL = BASE / atr_pct_rank`, clamped [SL_MIN, SL_MAX].

## Results

### RUN76.1 — Volatility-Adaptive SL Grid Search (82 configs × 18 coins, 5-month 15m data)

**NEGATIVE — ALL 81 configs reduce PnL vs baseline.**

| Config | PnL | ΔPnL | WR% | Trades | PF |
|--------|------|------|-----|--------|-----|
| BASELINE (fixed 0.30%) | +$293.21 | — | 25.9% | 9,714 | 0.35 |
| B0.0020_W50_MIN0.0010_MAX0.0050 (best) | +$288.23 | -$4.98 | 28.9% | 9,303 | 0.41 |
| B0.0020_W50_MAX0.0080 | +$278.60 | -$14.61 | 31.4% | 8,921 | 0.46 |
| B0.0040_W50_MAX0.0080 (worst) | +$239.36 | -$53.85 | 37.3% | 8,253 | 0.59 |

**Key findings:**
- ALL 81 configs produce lower PnL than baseline
- WR improves substantially: +2.7 to +11.4pp across configs
- PF improves: 0.35 → up to 0.59
- Trade count drops: 4-15% reduction due to wider SLs catching fewer trades
- Trade count reduction + larger avg_loss per losing trade dominates marginal WR improvement

**Why it fails:** Volatility-adaptive SL widens stops in low-vol markets (where 0.30% is already too tight relative to the ~0.1% daily range). Wider stops allow trades to run longer but mean larger losses when they eventually stop out. The PF improvement (+0.24) doesn't compensate for the trade count loss (-4-15%). Fixed 0.30% SL remains optimal.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

## Files
- `run76_1_results.json` — Grid search results
- `coinclaw/src/run76.rs` — Implementation
