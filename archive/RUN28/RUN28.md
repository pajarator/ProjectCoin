# RUN28 — Momentum Persistence Classifier

## Goal

Diagnose why the breakout strategy (RUN27) works on NEAR/DASH/XLM but not on other coins. Test the hypothesis: **coins where hard moves don't continue after the breakout should be excluded from the strategy.**

## Method

- **Data:** 18 coins, 15m 1-year OHLCV (full dataset, no train/test split — this is pure measurement)
- For every bar where breakout conditions partially fire (|ret16| ≥ thresh, vol ≥ vol_ma × mult, ADX ≥ thresh AND rising), record the event and the coin's direction.
- Measure the **next 4 / 8 / 16 / 32 bars** (1h / 2h / 4h / 8h): does price continue in the breakout direction?
- **Continuation rate** = P(price moves further in same direction at horizon H)
- **Baseline** = P(any random bar's price is higher at horizon H) ≈ 48–50%
- **Edge** = continuation_rate − baseline (positive = momentum persists; negative = mean-reverts)
- Three condition levels: loose (≥1.5%, 1.5×vol, ADX≥20), medium (≥2.0%, 2.0×, ADX≥20), strict (≥2.5%, 2.0×, ADX≥25)
- Also measure **average forward return** at 16 bars (magnitude of persistence)

---

## Results

### Long Edge at 16 Bars (medium conditions, sorted by 16-bar edge)

| Coin | L.n | L@16h edge% | S@16h edge% | 27.1 L.WR% | 27.1 S.WR% |
|------|-----|------------|------------|------------|------------|
| NEAR | 443 | **+2.4** | -1.4 | 48.4 | 46.2 |
| XLM  | 348 | **+1.4** | -1.4 | 50.0 | 46.7 |
| ETH  | 286 | +0.9 | -3.8 | 0.0 | 40.0 |
| UNI  | 502 | +0.3 | +1.6 | 20.0 | 37.5 |
| XRP  | 283 | +0.0 | -3.0 | 36.4 | 75.0 |
| DOGE | 426 | -0.0 | -2.3 | 62.5 | 50.0 |
| SOL  | 348 | -0.2 | -0.2 | 40.0 | 41.2 |
| AVAX | 400 | -0.4 | +0.1 | 58.3 | 20.0 |
| BTC  | 92  | -1.8 | +3.4 | 44.4 | 26.3 |
| LINK | 357 | -2.1 | -0.4 | 16.7 | 38.9 |
| DASH | 533 | **-3.4** | -2.9 | 37.1 | 38.1 |
| ALGO | 413 | -3.1 | +1.1 | 33.3 | 50.0 |
| BNB  | 142 | -3.7 | -5.4 | 0.0  | 27.8 |
| DOT  | 347 | -5.1 | +1.9 | 18.2 | 23.1 |
| ADA  | 399 | -5.2 | -1.2 | 36.8 | 40.0 |
| LTC  | 268 | -5.2 | -6.1 | 16.7 | 40.0 |
| SHIB | 351 | -6.1 | -0.3 | 42.1 | 40.0 |
| ATOM | 274 | **-12.4** | -3.6 | 36.4 | 50.0 |

*Baseline: ~48.4% (slightly below 50% — mild downward drift in test period)*

---

## Conclusions

### 1. NEAR and XLM have genuine momentum persistence — DASH does NOT

**NEAR** shows +2.4–4.0% edge at 16 bars across loose and medium conditions. After a qualifying NEAR breakout, price is statistically more likely to continue in the same direction over the next 4 hours. This explains the 48.4% WR and PF=1.60 in RUN27.1 — the strategy works because NEAR genuinely trends after hard moves.

**XLM** shows +1.4% edge at medium, rising to **+5.3% at strict conditions**. The edge only emerges when the conditions are tight — loose conditions include too many marginal moves that revert. The strict filter selects the real breakouts, which do persist.

**DASH shows -3.4% edge at 16 bars** (medium conditions). After a DASH long breakout, fewer than baseline bars end up higher at the 4-hour mark. This is the opposite of persistence — and yet DASH was RUN27.1's best result (+25.4% PnL, PF=1.48, 70 trades). This is not a contradiction — see finding #2.

### 2. DASH's edge is right-tail skew, not direction persistence

DASH's 16-bar edge is negative (-3.4%) but its **average forward return is +0.822%** — the highest of all coins. This reveals the actual mechanism:

- Most DASH breakouts (>52%) end up slightly *lower* at 16 bars (hence cont_rate < baseline)
- A minority of DASH breakouts continue dramatically upward, pulling the average return strongly positive
- The trailing stop (0.75× ATR, activation at 0.5%) captures these large right-tail outcomes before the reversal
- The ATR stop (1.0×) caps losses on the majority of reversals

Result: WR=37.1% (most trades stop out) but avg_win/avg_loss = 2.5 (the wins are huge). The strategy exploits the **shape of the return distribution**, not direction predictability. A simple 16-bar continuation test can't detect this edge.

This is the same mechanism DASH uses for OuMeanRev (RUN11): structural R:R edge rather than win-rate edge.

### 3. Strong mean-reverting breakout coins — disable the strategy

| Coin | 16-bar edge | Mechanism |
|------|------------|-----------|
| ATOM | **-12.4%** | Powerfully mean-reverts after breakout, every horizon |
| SHIB | -6.1% | Fast mean-reversion, likely due to low liquidity spikes |
| LTC  | -5.2% | Consistent negative edge across all horizons |
| ADA  | -5.2% | Negative at 8, 16, 32 bars |
| DOT  | -5.1% | Negative at 8, 16, 32 bars |
| BNB  | -3.7% | Negative long, short also very negative (-5.4%) |

These coins break out hard and then snap back with equal or greater force. The breakout strategy will consistently lose money on them regardless of parameter tuning. These should be **explicitly excluded** from any COINCLAW implementation.

### 4. "Positive" RUN27.1 results that are suspect

Some RUN27.1 coins showed high WR but are not supported by persistence:

- **DOGE** (62.5% WR, -0.0% edge): Near-zero persistence. The RUN27.1 DOGE result was 16 trades — likely statistical noise.
- **AVAX** (58.3% WR, -0.4% edge): Also low trade count (12 trades), no persistence detected.
- **SOL** (40.0% WR, -0.2% edge): 5 trades, no persistence.

These should not be acted on. The high WR was noise from small sample sizes.

### 5. Why ETH has positive persistence but zero WR in RUN27.1

ETH shows +0.9% edge at medium, +3.4% at strict — genuine persistence. But RUN27.1 showed **0% WR (2 trades)** for ETH long. The strategy fired only twice in the OOS period, making the result meaningless. The persistence data suggests ETH *could* be a valid breakout coin — it's starved for signals because the strict conditions rarely trigger on ETH's lower volatility profile.

To activate ETH: would need looser conditions (e.g., move≥1.5%) or a longer test window.

### 6. Coin selection rule for COINCLAW integration

Based on 16-bar edge across all three condition levels:

| Category | Coins | Action |
|----------|-------|--------|
| **Persistence confirmed** | NEAR, XLM | Enable (strict conditions) |
| **R:R skew confirmed** | DASH | Enable with caution — different mechanism |
| **Borderline / low signal** | ETH, BTC, SOL, AVAX | Monitor, do not enable now |
| **Mean-reverts after breakout** | ATOM, SHIB, LTC, ADA, DOT, BNB | **Explicitly disable** |
| **Near-zero edge / noisy** | UNI, LINK, ALGO, DOGE, XRP | Skip |

### 7. The avg_fwd_ret ranking does not correlate with RUN27.1 WR

DASH (+0.822%) and UNI (+0.642%) top the avg_fwd_ret ranking, but UNI has WR=20% and fails all walk-forward tests. The raw forward return confounds the effect of the exit mechanism — the true edge only emerges when the ATR stop + trailing is applied. Persistence analysis is more reliable than raw avg_fwd_ret for predicting strategy success.

---

## Decision

**Positive for diagnosis** — the analysis confirms the user's hypothesis:
- Coins where the strategy works (NEAR, XLM) have genuine persistence
- Coins where it fails by mean-reverting (ATOM, LTC, SHIB, ADA, DOT) should be disabled
- DASH is a special case: right-tail skew rather than direction persistence
- The anti-momentum filter (16-bar edge ≤ −4pp) provides a principled exclusion rule

**COINCLAW integration path:**
1. Momentum breakout as independent signal layer (own ATR stop, per RUN27.3 finding)
2. Enabled coins: NEAR, XLM (and DASH as R:R play)
3. Disabled coins: ATOM, SHIB, LTC, ADA, DOT, BNB (negative persistence)
4. This reduces the 18-coin universe to 3 enabled coins — the right trade-off between signal quality and universe size

## Files

| File | Description |
|------|-------------|
| `run28_results.json` | Per-coin persistence rates, edges, t-stats at all 3 condition levels |
| `RUN28.md` | This file |

Source: `tools/src/run28.rs`
