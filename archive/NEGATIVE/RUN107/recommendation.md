# RUN107 — Percentile Rank Z-Filter: Recommendation

## Hypothesis

**Named:** `percentile_z_filter`

Require z-score to be in the most extreme X% of its own historical distribution before entering. This normalizes entry signals across volatility regimes.

## Results

### RUN107.1 — Grid Search (28 configs × 18 coins, 5-month 15m data)

**STRONGLY NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate |
|--------|------|------|-----|--------|-----------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% |
| ZW100_PL15_PS85 (best) | +$176.95 | -$183.60 | 37.9% | 7,198 | 66.7% |
| ZW200_PL15_PS90 | +$176.34 | -$184.21 | 37.9% | 7,207 | 66.7% |

**Key findings:**
- ALL percentile z-filter configs produce PnL 50% worse than baseline
- Filter rate is 66-75% — the percentile requirement is far too restrictive
- The percentile rank of z-score is NOT correlated with mean-reversion success
- Filtering 66% of entries doesn't improve quality enough to offset the lost opportunity
- WR actually drops slightly (39.0% → 37.9%) for filtered entries — the filter is anti-selective
- The z-score is already a normalized measure of deviation; adding percentile ranking on top is redundant

## Conclusion

**STRONGLY NEGATIVE.** The percentile z-filter fundamentally doesn't work. The z-score already captures deviation extremity; percentile ranking it against its own history adds no predictive value and costs too many opportunities.

## Files
- `run107_1_results.json` — Grid search results
- `coinclaw/src/run107.rs` — Implementation
