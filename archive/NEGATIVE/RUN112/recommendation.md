# RUN112 — Money Flow Index Confirmation: Recommendation

## Hypothesis

**Named:** `mfi_confirmation`

Require MFI extreme at regime entry: MFI < 30 for LONG (selling pressure), MFI > 70 for SHORT (buying pressure).

## Results

### RUN112.1 — Grid Search (9 configs × 18 coins, 5-month 15m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% |
| LM40_SM60 (best) | +$185.38 | -$175.18 | 39.5% | 7,082 | 67.4% |
| LM30_SM60 | +$184.46 | -$176.09 | 39.6% | 7,064 | 67.6% |

**Key findings:**
- ALL MFI configs block 67-68% of entries — far too restrictive
- WR improves slightly (+0.5pp) but PnL drops ~50% due to massive trade reduction
- MFI threshold combinations produce nearly identical results (filter rate 67-68% always)
- Even the loosest thresholds (LM40, SM60) block two-thirds of entries

## Conclusion

**NEGATIVE.** MFI confirmation is too restrictive to be useful. While WR ticks up marginally, the 67% entry reduction more than doubles the PnL loss. MFI is essentially redundant with z-score as a regime filter — the z-score already captures the "extreme" that MFI also tries to capture, so requiring both doubles down on filtering.

## Files
- `run112_1_results.json` — Grid search results
- `coinclaw/src/run112.rs` — Implementation
