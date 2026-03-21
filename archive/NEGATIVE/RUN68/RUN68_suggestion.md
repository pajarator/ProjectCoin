# RUN68 — Cross-Asset Correlation at Entry: Position Sizing Based on BTC Correlation

## Hypothesis

**Named:** `btc_correlation_sizing`

**Mechanism:** When a coin's price is highly correlated with BTC's moves, it tends to move more with the market. In a market breadth regime where most coins are oversold (LONG mode), a coin with high BTC correlation may be more likely to recover because BTC's broader market direction is bullish. Conversely, in a SHORT regime, high BTC correlation means the coin will fall more with BTC — good for SHORTs.

The hypothesis: **position size should scale with the coin's recent correlation to BTC**:
- Compute rolling correlation of the coin's 15-bar returns with BTC's 15-bar returns
- Higher correlation → larger position (more market-aligned)
- Lower correlation → smaller position (less market-aligned, more idiosyncratic risk)

```
rolling_corr = correlation(coin_returns[−15:], btc_returns[−15:])
position_fraction = RISK × correlation_multiplier × rolling_corr
```

**Why this is not a duplicate:**
- RUN49 tested correlation for clustering/suppression — this uses correlation for *position sizing*
- RUN63 tested BTC trend direction for regime entries — this tests BTC correlation magnitude at entry
- No prior RUN used rolling return correlation as a position sizing input

---

## Proposed Config Changes

```rust
// RUN68: Cross-Asset Correlation Position Sizing
pub const CORR_SIZE_ENABLE: bool = true;
pub const CORR_LOOKBACK: u32 = 15;          // bars for rolling correlation
pub const CORR_SIZE_MULTIPLIER: f64 = 0.5;  // max upsizing at corr=1.0: RISK × 1.5
pub const CORR_MIN_THRESHOLD: f64 = 0.3;   // minimum correlation for full sizing
```

**`indicators.rs` — add btc correlation to Ind15m:**
```rust
pub struct Ind15m {
    // ... existing fields ...
    pub btc_corr: f64,        // rolling correlation with BTC 15-bar returns
}
```

**`engine.rs` — correlation-adjusted position sizing:**
```rust
fn correlation_position_multiplier(ind: &Ind15m) -> f64 {
    if !config::CORR_SIZE_ENABLE { return 1.0; }
    let corr = ind.btc_corr;
    if corr.is_nan() || corr < config::CORR_MIN_THRESHOLD {
        return 0.5;  // low correlation = reduce position
    }
    let mult = 1.0 + config::CORR_SIZE_MULTIPLIER * corr;
    mult.min(1.0 + config::CORR_SIZE_MULTIPLIER)  // cap at max
}

fn open_position(...) {
    let corr_mult = correlation_position_multiplier(&ind);
    let risk_fraction = config::RISK * corr_mult;
    let trade_amt = cs.bal * risk_fraction;
    let sz = (trade_amt * config::LEVERAGE) / price;
    // ... rest unchanged ...
}
```

---

## Validation Method

### RUN68.1 — BTC Correlation Sizing Grid Search (Rust + Rayon, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins + BTC, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed RISK = 0.10 per trade

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CORR_LOOKBACK` | [10, 15, 20] |
| `CORR_SIZE_MULTIPLIER` | [0.3, 0.5, 0.7, 1.0] |
| `CORR_MIN_THRESHOLD` | [0.2, 0.3, 0.4] |

**Per coin:** 3 × 4 × 3 = 36 configs × 18 coins = 648 backtests

**Key metrics:**
- `avg_corr_mult`: average position multiplier used
- `corr_block_rate`: % of entries with very low correlation (mult < 0.5)
- `PF_delta`: profit factor change vs baseline
- `Sharpe_delta`: Sharpe ratio change vs baseline
- `max_DD_delta`: max drawdown change vs baseline

### RUN68.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best `CORR_LOOKBACK × CORR_MULTIPLIER` per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS Sharpe delta vs baseline
- Portfolio OOS max_DD improvement ≥ 10%

### RUN68.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16) | BTC Correlation Sizing | Delta |
|--------|---------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Sharpe Ratio | X.XX | X.XX | +0.XX |
| Max DD | X% | X% | −Ypp |
| WR% | X% | X% | +Ypp |
| PF | X.XX | X.XX | +0.XX |
| Avg Position Mult | 1.0 | X | +Y |
| Avg BTC Corr at Entry | X% | X% | +Ypp |

---

## Why This Could Fail

1. **Correlation is backward-looking:** Rolling 15-bar correlation may not reflect current market structure. Past correlation doesn't guarantee future alignment.
2. **Position sizing doesn't change outcomes:** The trade result is determined by entry/exit prices, not position size. Changing position size changes $ P&L but not win rate or PF.
3. **BTC correlation is uniformly high in crypto:** Most altcoins correlate highly with BTC (>0.6). The variation in correlation may be too small to meaningfully differentiate position sizes.

---

## Why It Could Succeed

1. **Diversification principle:** Low-correlation assets should have smaller positions because they add idiosyncratic risk to the portfolio. Correlation sizing applies this principle systematically.
2. **Simple change:** No new data fetch — BTC 15m data is already available via the fetcher.
3. **Addresses portfolio risk:** During broad market sell-offs, low-correlation coins may hold up better. Reducing their size during high-correlation regimes (when BTC correlation matters most) could lower drawdowns.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN68 BTC Correlation Sizing |
|--|--|--|
| Position sizing | Fixed RISK = 10% | Risk × correlation multiplier |
| Correlation check | None | Rolling 15-bar BTC return correlation |
| Max position | 10% | 10–20% (configurable) |
| Min position | 10% | 5–10% (low corr) |
| Expected Sharpe | X.XX | +0.05–0.15 |
| Expected Max DD | X% | −10–20% |
