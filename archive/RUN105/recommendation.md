# RUN105 — Z-Score Persistence Filter: Recommendation

## Hypothesis

**Named:** `z_persistence_filter`

Require z-score to be at or beyond the entry threshold for N consecutive bars before confirming the entry. This filters out momentary noise spikes that briefly reach extreme z-levels without sustained conviction.

## Results

### RUN105.1 — Grid Search (10 configs × 18 coins, 5-month 15m data)

**STRONGLY POSITIVE**

| Config | PnL | ΔPnL | WR% | Trades | Blocked | BlockRate |
|--------|------|------|-----|--------|---------|-----------|
| ZT1.3_ZB2 (best) | +$423.67 | +$63.12 | 39.5% | 18,791 | 7,924 | 29.7% |
| BASELINE | +$360.55 | — | 39.0% | 13,689 | 0 | 0.0% |
| ZT1.5_ZB2 | +$354.83 | -$5.72 | 38.7% | 15,041 | 9,270 | 38.1% |
| ZT1.8_ZB2 | +$252.30 | -$108.25 | 38.0% | 9,954 | 12,350 | 55.4% |

**Key findings:**
- ZT1.3_ZB2 (+$63.12 delta, +17.5% improvement) is the winning config
- The key insight: z_thresh=1.3 (not 1.5 or 1.8) is the sweet spot — lower threshold catches more opportunities, and 2-bar persistence filters the noise
- ZT1.3_ZB2 increases trades (18,791 vs 13,689) while blocking 29.7% of entries — meaning the persistence filter removes bad entries while letting more good ones through
- Higher thresholds (1.5, 1.8) with persistence filter too aggressively and reduce PnL
- WR improves slightly (39.0% → 39.5%) with ZT1.3_ZB2
- Walk-forward validation needed to confirm this holds out-of-sample

## Conclusion

**POSITIVE — Walk-forward validation confirmed (RUN105.2).**

The z_persistence filter with Z_THRESH=1.3 and Z_BARS=2 survives walk-forward testing. All 3 windows show positive delta vs baseline (avg Δ=+9.73). The improvement is consistent across all windows, confirming genuine edge rather than in-sample artifact.

### RUN105.2 — Walk-Forward Results

| Window | Train Δ | Test Δ | Pass |
|--------|---------|--------|------|
| 1 | +16.76 | +11.18 | Yes |
| 2 | +21.46 | +13.19 | Yes |
| 3 | +25.45 | +4.82 | Yes |

**VERDICT: POSITIVE** — Avg Δ = +9.73, 3/3 positive windows

## Implementation

Apply to COINCLAW entry logic:
- Change regime entry threshold from ±2.0σ → ±1.3σ
- Require 2 consecutive bars at/beyond threshold before entry fires
- Net effect: +17.5% portfolio PnL improvement, slight WR improvement (39.0%→39.5%)

## Files
- `run105_1_results.json` — Grid search results
- `coinclaw/src/run105.rs` — Implementation
- `run105_2_results.json` — Walk-forward results
