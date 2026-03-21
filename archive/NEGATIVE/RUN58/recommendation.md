# RUN58 — CME Gap Fill Strategy: Recommendation

## Hypothesis
Named: `cme_gap_fill`

Trade CME weekend gaps — short BTC/altcoins on Monday open when Friday close to Monday open gap exceeds threshold, expecting the gap to fill.

## Results

### RUN58.1 — CME Gap Fill Backtest

**INCONCLUSIVE — No CME gaps in Binance spot data.**

The Binance 15m data is continuous 24/7 trading. Investigation revealed:
- Data spans 00:00–23:45 UTC each day (96 × 15m bars)
- Friday 23:45 → Saturday 00:00 is a continuous 15-minute gap
- No overnight or weekend gap in Binance spot — crypto trades continuously
- CME Bitcoin futures have weekend gaps (Friday 5pm ET close → Monday 6pm ET open), but this dataset is Binance spot which is 24/7
- **0 Friday-Monday gap opportunities across all 18 coins over 5 months**

## Conclusion

**INCONCLUSIVE — No COINCLAW changes.**

The hypothesis cannot be tested with Binance spot data. CME gap fill would require CME futures data or a different gap definition (e.g., gap within the trading day). The strategy is not applicable to this dataset.

## Files
- `run58_1_results.json` — Grid search results (all zeros — no gaps detected)
- `coinclaw/src/run58.rs` — Implementation
