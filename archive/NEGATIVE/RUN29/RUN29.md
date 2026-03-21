# RUN29 — MAX_SCALP_OPENS_PER_CYCLE Optimization

## Goal
Determine the optimal `MAX_SCALP_OPENS_PER_CYCLE` cap for COINCLAW scalp layer.
Hypothesis: the current cap=3 (v13/v14) blocks profitable correlated-move entries that v9/v11 captured freely.

## Trigger
Live comparison showed v9/v11 significantly outperforming v13/v14 over a 30-minute window. Analysis identified that at 12:26, 8 coins simultaneously triggered scalp signals. v9/v11 (no cap) opened all 8; v13/v14 (cap=3) opened only 3. The 5 missed entries all hit TP +1–1.5% by 12:31.

## Method
- **Data**: 1-year 1m OHLCV for all 18 COINCLAW coins (526,636 bars/coin, ~365.7 days)
- **Signals**: stoch_cross + vol_spike_rev with F6 filter (exact COINCLAW v13 logic)
- **Simulation**: TP=0.80%, SL=0.10%, MaxHold=60 bars, scalp cooldown=300 bars after SL, fees=0.1%, slippage=0.05%
- **Grid**: cap ∈ {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, unlimited}
- **Tool**: Rust + Rayon, all 19 cap values in parallel

## Results

| cap | trades | WR% | total_pnl | captures |
|-----|--------|-----|-----------|----------|
| 1   | 17,670 | 11.1% | −29.86 | 96.5% |
| 2   | 18,237 | 10.9% | −31.04 | 99.6% |
| **3** | **18,301** | **10.9%** | **−31.16** | **99.9%** |
| 4   | 18,312 | 10.9% | −31.18 | 100.0% |
| 7+  | 18,318 | 10.9% | −31.20 | 100.0% |

Average simultaneous signals per signal bar: **1.097**

### Per-coin breakdown (cap=3)

| Coin | Trades | WR% | PnL |
|------|--------|-----|-----|
| BTC  | 1,682 | 12.3% | −2.87 |
| BNB  | 1,501 | 11.7% | −2.59 |
| LTC  | 1,211 | 11.6% | −1.99 |
| ETH  | 1,142 | 10.9% | −1.99 |
| XRP  | 1,147 | 11.9% | −1.95 |
| ATOM | 1,148 | 11.0% | −1.94 |
| SOL  | 1,043 | 10.5% | −1.81 |
| DOGE | 1,033 | 10.2% | −1.80 |
| XLM  |   928 |  9.4% | −1.66 |
| UNI  | 1,048 | 11.7% | −1.63 |
| DOT  |   963 | 11.2% | −1.62 |
| DASH |   984 | 11.4% | −1.61 |
| ADA  | 1,021 | 12.0% | −1.57 |
| ALGO |   907 | 11.2% | −1.47 |
| NEAR |   745 |  9.5% | −1.31 |
| AVAX |   666 |  7.8% | −1.25 |
| LINK |   611 |  8.0% | −1.15 |
| SHIB |   521 |  9.0% | −0.96 |

## Conclusions

**1. The cap is essentially irrelevant.**
Cap=3 already captures 99.9% of all scalp signals over the full year. The average number of simultaneous signals per bar is only 1.097 — meaning 2+ coins firing at once is rare, and 8+ coins firing at once (the v9/v11 live event) is a statistical outlier.

**2. The scalp strategy is unprofitable on 1m data.**
WR = 10.9% vs breakeven WR = 27.8% (at TP=0.8%, SL=0.1%, 0.15% round-trip cost). All 18 coins lose money over the full year. The R:R of 8:1 requires very high hit rate to be profitable — the signal conditions (stoch < 5.0 AND crossover, or RSI < 20 AND vol > 3.5×) are too rare and not directionally reliable enough.

**3. The v9/v11 live outperformance was a 30-minute statistical artifact.**
At 12:26 there was a correlated market pump. All 8 long scalps that v9/v11 captured happened to hit TP in the next 5 minutes. This is exactly the kind of favorable short window that looks great in live observation but doesn't survive full-year evaluation.

**4. No change to COINCLAW.**
Current cap=3 is fine — it captures essentially all signals. The scalp layer's long-term profitability is the actual open question (separate investigation if desired).

## Files
- `run29_1_scalp_cap_optimization.rs` — Rust simulation source
- `run29_1_results.json` — Full results per cap and per coin
