# RUN10 — Scalp Indicator Discovery, Filter Validation & Fee-Aware TP/SL Optimization

## Goal

More than half of scalp trades were losing. Discover which 1m indicators correlate with winning scalp trades, validate filters out-of-sample, and find TP/SL settings that remain profitable after trading fees.

## Method

### RUN10.1 — Indicator Discovery (Python)

Ran all 3 scalp strategies (vol_spike_rev, stoch_cross, bb_squeeze_break) across 18 coins on 5 months of 1m data. Collected 35,264 trades with 35+ indicator snapshots per trade.

**Analysis techniques:**
- Point-biserial correlation (indicator vs win/loss)
- Cohen's d effect size
- Quintile analysis (monotonic WR gradient across indicator ranges)
- Combined filter search (1, 2, and 3-indicator combos)

**Key findings:**
- No single indicator has strong correlation (all < 0.15)
- Best combined filter **F6**: `dir_roc_3 < -0.195 AND avg_body_3 > 0.072`
  - dir_roc_3: directional rate of change over 3 bars (counter-momentum)
  - avg_body_3: average candle body size over 3 bars (active candles, not dojis)
- F6 filters out ~42% of trades while preserving winning trades

### RUN10.2 — Walk-Forward Validation (Rust)

3-window walk-forward (2mo train, 1mo test) across 18 coins. Tested 7 filter candidates.

| Filter | OOS WR Delta | % Positive Windows | Verdict |
|--------|-------------|-------------------|---------|
| F7 (F1+F2+F3) | +2.9% | 83.7% | ACCEPT |
| F6 (F1+F2) | +2.5% | 82.0% | ACCEPT |
| F1 (dir_roc_3) | +2.3% | 79.6% | ACCEPT |
| F5 (atr_ratio) | +1.5% | 63.3% | MARGINAL |
| F2 (avg_body_3) | +1.2% | 61.2% | MARGINAL |
| F3 (spread_pct) | +0.8% | 55.1% | MARGINAL |
| F4b (SHORT z15m) | -3.6% | 28.6% | **REJECT** (overfit) |

**Decision:** Use F6 (simpler than F7, nearly equal performance, fewer parameters to overfit).

### RUN10.3 — Fee-Aware TP/SL Grid Search (Rust)

**Critical discovery:** RUN9 backtests had NO fee deduction. Current SL=0.10%/TP=0.20% scalps are unprofitable after any realistic fee.

Grid: 8 TP levels x 5 SL levels x 2 fee tiers x 6 strategy/filter combos = 480 evaluations.

**Results at taker fees (0.05%/side, round-trip 0.10%):**

Only 1 profitable combo:
| Config | WR | Net P&L | PF |
|--------|------|---------|-----|
| TP=0.80% SL=0.10% volspike_F6 | 25.9% | +6.9% | 1.22 |

**Results at maker fees (0.02%/side, round-trip 0.04%):**

114 profitable combos. Top 5:
| Config | WR | Net P&L | Monthly | PF |
|--------|------|---------|---------|-----|
| TP=0.80% SL=0.10% all_F6 | 24.4% | +379.5% | +75.9% | 1.76 |
| TP=0.80% SL=0.10% nobb_F6 | 24.4% | +379.5% | +75.9% | 1.76 |
| TP=0.60% SL=0.10% all_F6 | 27.3% | +242.2% | +48.4% | 1.50 |
| TP=0.80% SL=0.15% all_F6 | 28.4% | +279.9% | +56.0% | 1.59 |
| TP=0.50% SL=0.10% all_F6 | 29.8% | +181.9% | +36.4% | 1.39 |

**Results at zero fees (Bitfinex 0% taker):**

All gross P&L applies directly. TP=0.80% SL=0.10% with F6 is the clear winner.

**Additional findings:**
- `all_F6` and `nobb_F6` produce **identical** results across every combo — bb_squeeze_break contributes zero trades after F6 filtering
- vol_spike_rev has highest per-trade quality (PF=1.97) but half the volume of all strategies combined
- F6 filter adds +60-80% absolute net P&L across all TP/SL combos

## Conclusions

1. **Fees are critical** — at TP=0.20% (old config), round-trip fees consume 50-100% of gross win
2. **Wide TP is the fix** — TP=0.80% with SL=0.10% creates 8:1 reward/risk ratio; only needs ~24% WR to profit
3. **F6 filter validated OOS** — consistently improves results by removing flat/directionless entries
4. **bb_squeeze_break is dead** — zero contribution after F6 filter; removed
5. **vol_spike_rev rehabilitated** — was disabled in v9 due to poor WR at narrow TP; works well at wide TP + F6

## Changes Applied (COINCLAW v9 → v10)

| Parameter | v9 | v10 |
|-----------|-----|-----|
| SCALP_TP | 0.20% | **0.80%** |
| SCALP_SL | 0.10% | 0.10% (unchanged) |
| F6 filter | none | **dir_roc_3 < -0.195 AND avg_body_3 > 0.072** |
| vol_spike_rev | DISABLED | **ENABLED** (with F6 gate) |
| stoch_cross | active | active (with F6 gate) |
| bb_squeeze_break | active | **REMOVED** |

## Files

- `run10_1_scalp_indicator_discovery.py` — Python indicator discovery script
- `run10_1_trades.json` — Raw trade data with indicator snapshots (35,264 trades)
- `run10_1_results.json` — Correlation, Cohen's d, quintile, filter analysis
- `run10_2_wf_src/main.rs` — Rust walk-forward validation
- `run10_2_wf_Cargo.toml` — Cargo config for run10.2
- `run10_2_results.json` — Walk-forward results (7 filters, 3 windows, 18 coins)
- `run10_3_fees_src/main.rs` — Rust fee-aware grid search
- `run10_3_fees_Cargo.toml` — Cargo config for run10.3
- `run10_3_results.json` — Grid search results (480 combos)
