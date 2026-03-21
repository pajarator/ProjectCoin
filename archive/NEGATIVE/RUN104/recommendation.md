# RUN104 — Volume Dry-Up Exit: Recommendation

## Hypothesis

**Named:** `volume_dryup_exit`

Exit regime trades when volume collapses below a threshold (e.g., vol < vol_ma × 0.30) for N consecutive bars while the trade is profitable. The thesis: low volume means the price lacks conviction and reversal risk is elevated.

## Results

### RUN104.1 — Grid Search (28 configs × 18 coins, 5-month 15m data)

**MARGINAL POSITIVE**

| Config | PnL | ΔPnL | WR% | Trades | DryExits | DryupRate |
|--------|------|------|-----|--------|---------|-----------|
| VT0.30_VB3_MH12 (best) | +$361.23 | +$0.68 | 39.1% | 13,695 | 43 | 0.3% |
| VT0.30_VB2_MH12 | +$360.79 | +$0.24 | 39.1% | 13,697 | 105 | 0.8% |
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0 | 0.0% |

**Key findings:**
- Best delta is +$0.68 — effectively zero
- Configs with higher dryup rates (10-20%) produce negative PnL deltas
- The dryup exit only fires at very high thresholds (0.30) with many consecutive bars (3) and long min_hold (12) — meaning it barely fires
- Volume collapse events during profitable positions are rare and when they do fire, they're not adding alpha
- Higher volume thresholds (more sensitive) cut winners too aggressively

## Conclusion

**MARGINAL POSITIVE — Not worth the complexity.** The +$0.68 delta across 18 coins over 5 months is ~$0.04/coin/month — effectively zero.

## Files
- `run104_1_results.json` — Grid search results
- `coinclaw/src/run104.rs` — Implementation
