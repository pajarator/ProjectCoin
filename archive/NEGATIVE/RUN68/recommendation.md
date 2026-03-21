# RUN68 — BTC Correlation Position Sizing: Recommendation

## Hypothesis
Named: `btc_correlation_sizing`

Scale position size by BTC correlation: higher correlation → larger position. When coin moves with BTC, size up; when idiosyncratic, size down.

## Results

### RUN68.1 — BTC Correlation Grid Search (18 configs × 18 coins, 5-month 15m data)

**POSITIVE in grid search, but walk-forward shows degradation.**

| Config | PnL | ΔPnL | WR% | Trades |
|--------|------|------|-----|--------|
| L10_M0.7_T0.2 (best) | +$267.79 | +$90.82 | 54.7% | 15,101 |
| L15_M0.7_T0.2 | +$267.47 | +$90.50 | 54.7% | 15,101 |
| L20_M0.7_T0.2 | +$266.63 | +$89.66 | 54.7% | 15,101 |
| ... all 18 configs beat baseline ... | | | | |
| BASELINE | +$176.97 | — | 54.7% | 15,101 |

**Key findings:**
- ALL 18 correlation configs beat baseline by significant margins (+$27 to +$91)
- Higher multiplier (0.7) consistently outperforms lower (0.3, 0.5) — correlation scaling amplifies returns
- Lookback of 10 bars slightly outperforms 15 and 20
- Lower threshold (0.2) slightly better than 0.4

### RUN68.2 — Walk-Forward Validation (3 windows, 2mo train / 1mo test)

**Walk-forward OOS deltas smaller than grid search, window 3 empty:**

| Window | Train Period | Test Period | Baseline OOS | Corr Sizing OOS | Δ |
|--------|------------|-------------|-------------|-----------------|---|
| 1 | bars 0-5760 | 5760-8640 | +$72.94 | +$111.15 | +$38.22 |
| 2 | 2880-8640 | 8640-11520 | +$34.72 | +$53.02 | +$18.30 |
| 3 | 5760-11520 | 11520-14400 | N/A | N/A | N/A |

- Universal config L10_M0.7_T0.2 won all 3 training windows
- OOS delta in windows 1-2: +$38 and +$18 (vs grid search delta of +$91)
- Window 3 test period is empty (last training window reaches end of dataset)
- Per-coin: 18/18 coins positive in both test windows (criterion met)
- Portfolio max DD: not computed in this simplified walk-forward

**Degradation concern:** Grid search delta (+$91) vs OOS delta (+$18-38) shows significant degradation. The correlation sizing effect is real but much smaller OOS.

## Conclusion

**NEGATIVE — No COINCLAW changes.**

While the grid search shows a strong positive effect (+$91 delta), the walk-forward shows significant degradation (+$18-38 OOS delta from +$91 IS delta). The walk-forward did not convincingly confirm 3/3 windows positive OOS (window 3 has no test data). Per-coin pass criterion was met but portfolio-level evidence is weak.

The correlation sizing mechanism is theoretically sound, but the practical implementation shows overfitting in grid search that doesn't fully hold OOS.

## Files
- `run68_1_results.json` — Grid search results
- `run68_2_results.json` — Walk-forward results
- `coinclaw/src/run68.rs` — Grid search implementation
- `coinclaw/src/run68_2.rs` — Walk-forward implementation