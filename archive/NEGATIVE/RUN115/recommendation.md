# RUN115 — Supertrend Confirmation: Recommendation

## Hypothesis

**Named:** `supertrend_confirm`

Use Supertrend (ATR-based adaptive bands) as entry direction filter and trailing stop for regime trades.

## Results

### RUN115.1 — Grid Search (12 configs × 18 coins, 5-month 15m data)

**NEGATIVE**

| Config | PnL | ΔPnL | WR% | Trades | FilterRate | STExits | STExitWR% |
|--------|------|------|-----|--------|--------|--------|--------|
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0% | 0 | — |
| M4.0_EF_NT (best) | +$325.40 | -$35.15 | 39.4% | 12,567 | 19.4% | 0 | — |
| M4.0_EF_TS | +$315.99 | -$44.56 | 37.8% | 12,724 | 19.2% | 706 | 0.1% |
| M2.0_NE_TS | +$346.77 | -$13.78 | 36.8% | 13,931 | 0% | 1,046 | 2.1% |
| M2.0_EF_TS | +$270.00 | -$90.56 | 35.5% | 11,126 | 38.7% | 1,025 | 1.7% |

**Key findings:**
- Supertrend trail stop fires 700-1000+ times per config with near-0% WR (0.1-2.1%)
- ST exits are catastrophic: the indicator catches falling knives — LONG exits when price drops below Supertrend_lower (meant to confirm downtrend) are almost always wrong
- Supertrend is a trend-following tool fundamentally misaligned with mean-reversion
- Entry filter (EF configs): blocks 19-39% of entries, reduces PnL by $36-91

## Conclusion

**STRONGLY NEGATIVE.** Supertrend is a trend-following indicator. Using it as an entry filter or trailing stop for mean-reversion trades is fundamentally contradictory. The ST trail stop fires with 0.1-2.1% WR — virtually every ST exit is a losing trade. The system exits when price briefly dips below the lower band (for longs), which is exactly the "oversold dip" that mean-reversion wants to buy.

## Files
- `run115_1_results.json` — Grid search results
- `coinclaw/src/run115.rs` — Implementation
