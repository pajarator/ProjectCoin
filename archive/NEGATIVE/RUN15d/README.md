# RUN15d: Pre-Signal Filter Comparison for COINCLAW Scalp Signals

**Date:** 2026-03-16
**Objective:** Test whether pre-signal filters (Time-of-Day, Liquidity) improve actual scalp trade quality, with and without an LR gate.

---

## Experiment Design

### Filters Tested
| Filter | Description |
|--------|-------------|
| **Baseline** | All vol_spike_rev + stoch_cross signals |
| **Time-of-Day** | Only during UTC hours 14–21 (NYSE session: 9:30 AM – 4 PM ET) |
| **Liquidity** | Only when vol_ratio ≥ 2.0 (filters low-volume stoch_cross signals) |

Each filter tested **without** and **with** LR(9 features) gate at threshold 0.55 — 6 total configurations.

### Trade Parameters
- TP=0.80%, SL=0.10%, fee=0.10%/side, slip=0.05%/side, max hold 60 bars
- Breakeven WR required: **≈44%**

### Methodology
- Full 1-year 1m data (~430k–525k bars per coin)
- 50/50 chronological split (train first half, test second half)
- LR trained on filtered signals from train half only
- TP/SL simulated on OHLCV bars (pessimistic: SL first if both hit same bar)

---

## Corrected Results

### Per-Coin Summary (binary WR by filter)

| Coin | Baseline WR | Baseline N | ToD WR | ToD N | Liquidity WR | Liquidity N |
|------|------------|------------|--------|-------|-------------|-------------|
| BTC  | 6.4% | 25,576 | 6.7% | 8,408 | 6.5% | 5,192 |
| ETH  | 7.5% | 21,803 | 7.5% | 7,070 | 7.4% | 4,882 |
| BNB  | 7.7% | 24,498 | 7.8% | 7,922 | 7.8% | 4,921 |
| SOL  | 7.9% | 22,382 | 7.9% | 7,363 | 7.7% | 4,517 |
| ADA  | 8.0% | 22,614 | 7.6% | 7,418 | 8.1% | 5,135 |
| XRP  | 7.7% | 22,986 | 7.7% | 7,572 | 8.2% | 4,478 |
| DOGE | 7.3% | 22,597 | 7.2% | 7,401 | 7.7% | 5,312 |
| LTC  | 7.4% | 23,130 | 7.6% | 7,586 | 7.6% | 4,755 |
| LINK | 5.9% | 17,761 | 5.7% | 6,211 | 5.6% | 4,107 |
| DOT  | 7.7% | 22,233 | 8.0% | 7,375 | 8.0% | 5,243 |

### Summary (OOS test half)

| Configuration | Avg WR | Avg PF | Total Trades | LR Trades | LR%ofBin |
|---------------|--------|--------|-------------|-----------|----------|
| **Baseline binary** | 7.3% | 0.092 | 225,580 | — | — |
| **Baseline + LR**   | 0.0% | 0.000 | — | **0** | 0.0% |
| **ToD binary**      | 7.4% | 0.098 | 74,326  | — | — |
| **ToD + LR**        | 0.0% | 0.000 | — | **0** | 0.0% |
| **Liquidity binary**| 7.5% | 0.094 | 48,542  | — | — |
| **Liquidity + LR**  | 0.0% | 0.000 | — | **0** | 0.0% |

---

## Conclusions

### Strong negative — pre-signal filters do not help

**Binary WR improvement from filters: negligible.** Time-of-Day moves avg WR from 7.3% to 7.4%. Liquidity moves it to 7.5%. Both are within noise — still 37 percentage points below the 44% breakeven. No filter fixes the fundamental problem.

**LR gate: 0 trades on all 6 configurations.** Identical to baseline run15b/c: the LR model learns the dominant 93%-loss class and assigns P(win) ≈ 7% to every signal. No signal passes the 0.55 threshold regardless of which pre-filter was applied.

**The liquidity filter removes 79% of signals** (225k → 48k) while barely changing WR (7.3% → 7.5%). The filtered signals are not materially better — the entire signal space (whether high-volume or not, whether NYSE hours or not) has the same fundamental problem: reaching TP=0.80% before hitting SL=0.10% on 1m bars.

**Why the filters can't help:** The 8:1 R:R ratio (TP=0.80%/SL=0.10%) combined with 1m bars means the price must travel 8× as far in the winning direction as in the losing direction before the trade resolves. In highly volatile 1m data, this essentially never happens regardless of market session or volume context.

### Bugs in original implementation

1. **3-bar lookahead target** — measured 3-minute price direction (≈coin-flip ~49.5%), not TP/SL outcome
2. **Artificial R:R** — modeled win=+3% (1%×3× leverage), loss=−1%. COINCLAW uses 8:1 R:R, not 3:1. All PnL figures fictional
3. **3-feature NN** — `run15d_final.py` trains on only 4 features (rsi, vol_ratio, stoch_k, hour) vs run15b's 9
4. **No fees or slippage** — critical at 1m with TP=0.80%/SL=0.10%
5. **Mixed model training** — `y_long = return > 0` for ALL signals (not just long signals)
6. **"Liquidity filter" mislabeled** — vol_ratio≥2.0 mainly filters stoch_cross; vol_spike already requires >3.5. Not a general liquidity filter

### Original claimed result vs corrected

| Approach | Original WR (3-bar lookahead, no fees) | Corrected WR (TP/SL, fees) |
|----------|---------------------------------------|---------------------------|
| Baseline | 49.5% | 7.3% |
| ToD | 49.9% | 7.4% |
| Liquidity | 51.4% | 7.5% |
| Baseline+NN | 55.3% | 0 trades |
| ToD+NN | 54.7% | 0 trades |
| Liquidity+NN | **57.1%** | 0 trades |

The ordering (Liquidity+NN > ToD+NN > Baseline+NN) was real in the coin-flip measurement frame. In the actual trade frame, all NN variants get 0 trades and the binary WRs are uniformly disastrous.

### Recommendation

**No COINCLAW changes.** All 6 configurations confirm the same conclusion: vol_spike_rev + stoch_cross signals have ~7.3% actual WR with COINCLAW scalp parameters. No pre-signal filter or NN gate can make these signals viable.

---

## Files

| File | Description |
|------|-------------|
| `run15d_test_filters.py` | v1 original (21 days) |
| `run15d_full_year.py` | v2 original (full year, buggy PnL model) |
| `run15d_final.py` | v4 original (corrected PnL model, still 3-bar lookahead) |
| `run15d_corrected.py` | v3 partial correction |
| `ideas.md` | Original brainstorm of 10 filter ideas |
| `run15d_results.json` | Results from original buggy run |
| `run15d_corrected_results.json` | Results from corrected Rust implementation |
| `README.md` | This file (updated with corrected conclusions) |

Corrected implementation source: `tools/src/run15d.rs`
