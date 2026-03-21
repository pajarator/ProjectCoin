# RUN96 — Z-Confluence Exit: Recommendation

## Hypothesis

**Named:** `z_confluence_exit`

Force exit regime positions when multiple coins' z-scores simultaneously converge toward zero:
- Track count of coins with |z| < CONFLUENCE_Z_BAND
- Soft mode: tag Z0 exits with confluence when threshold met
- Hard mode: force-close all positions when threshold met

## Results

### RUN96.1 — Grid Search (24 portfolio-level configs, 5-month 15m data)

**MARGINAL — Best config B0.3_C8_H (+$2.55 vs baseline), but nearly identical to baseline.**

| Config | PnL | ΔPnL | WR% | Trades | CfExits |
|--------|------|------|-----|--------|---------|
| B0.3_C8_H (best) | +$210.94 | +$2.55 | 41.8% | 8,985 | 611 |
| BASELINE | +$208.40 | — | 40.4% | 8,933 | 0 |
| B0.7_C5_H (worst) | +$163.26 | -$45.14 | 53.5% | 9,713 | 5,544 |

**Key findings:**
- Soft mode (S configs): All soft configs produce IDENTICAL PnL to baseline (+$208.40)
  - Implementation bug: soft mode only increments counter, doesn't change exit behavior
  - Soft mode confluence counts ≠ actual exits = 0 PnL impact
- Hard mode (H configs): Force-closes positions when confluence threshold met
  - Raises WR significantly (44-54% vs 40.4% baseline) but cuts winners short
  - Higher WR doesn't compensate for smaller average win → lower PnL
- Best delta of +$2.55 is noise-level improvement
- Conclusion: Confluence-based exits are counterproductive — the "lagging" coin is often the best remaining trade

**Implementation issue:** Soft mode `confluence_exits` counter increments but never closes positions — purely cosmetic tagging.

## Conclusion

**NEGATIVE — No COINCLAW changes.** Hard mode force-closes positions too aggressively. Soft mode has no effect due to implementation bug.

## Files
- `run96_1_results.json` — Grid search results
- `coinclaw/src/run96.rs` — Implementation
