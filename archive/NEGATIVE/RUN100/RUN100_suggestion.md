# RUN100 — Portfolio Correlation Risk Limit: Reduce Deployed Capital When Cross-Coin Correlation Spikes

## Hypothesis

**Named:** `corr_risk_limit`

**Mechanism:** COINCLAW currently deploys capital to each coin independently. But when cross-coin return correlation spikes (all coins move together), the portfolio is effectively a single concentrated bet on market direction. In high-correlation regimes, a drawdown on one coin means a drawdown on all coins simultaneously — the diversification benefit disappears and portfolio risk is underestimated.

**Portfolio Correlation Risk Limit:**
- Compute rolling average cross-coin return correlation each bar (mean pairwise correlation of all coin pairs)
- When `avg_correlation >= CORR_RISK_THRESHOLD` (e.g., 0.60):
  - Reduce total deployed capital by `CORR_DEPLOY_MULT` (e.g., 0.70×)
  - Achieved by reducing RISK fraction proportionally across all coins
  - When `avg_correlation >= CORR_CRITICAL` (e.g., 0.75): further reduce to 0.50×
- Correlation is computed from 20-bar rolling returns; smoothed with EMA to avoid noise
- Scalp and momentum trades exempt (they operate on shorter timeframes)

**Why this is not a duplicate:**
- RUN86 (correlation cluster suppression) suppressed entries within correlated clusters — this reduces overall portfolio capital deployment when correlation is elevated
- RUN64 (position density filter) used position count — this uses return correlation as the risk metric
- No prior RUN has scaled total portfolio capital based on market-wide correlation levels

**Mechanistic rationale:** Diversification works when coins move independently. When correlation spikes (e.g., BTC crashes drag all alts down), the portfolio's effective diversification collapses and risk is concentrated. Reducing deployed capital during high-correlation periods preserves capital for when diversification returns. This is the portfolio-level analogue of position-level risk management.

---

## Proposed Config Changes

```rust
// RUN100: Portfolio Correlation Risk Limit
pub const CORR_RISK_LIMIT_ENABLE: bool = true;
pub const CORR_LOOKBACK: usize = 20;          // bars for rolling correlation
pub const CORR_RISK_THRESHOLD: f64 = 0.60;   // activate risk limit when avg corr >= 0.60
pub const CORR_CRITICAL: f64 = 0.75;         // critical threshold for maximum reduction
pub const CORR_DEPLOY_MULT: f64 = 0.70;      // multiply deployed capital by 0.70 at threshold
pub const CORR_CRITICAL_MULT: f64 = 0.50;    // multiply deployed capital by 0.50 at critical
pub const CORR_SMOOTHING: f64 = 0.90;        // EMA smoothing factor for correlation
```

**`state.rs` — SharedState additions:**
```rust
pub struct SharedState {
    // ... existing fields ...
    pub smoothed_avg_corr: f64,      // EMA-smoothed average pairwise correlation
}
```

**`engine.rs` — compute_portfolio_correlation and apply_risk_limit:**
```rust
/// Compute average pairwise return correlation across all coins.
fn compute_avg_correlation(state: &SharedState, lookback: usize) -> f64 {
    let n = state.coins.len();
    if n < 2 { return 0.0; }

    // Collect returns for each coin
    let mut returns: Vec<Vec<f64>> = Vec::new();
    for cs in &state.coins {
        if cs.candles_15m.len() < lookback + 1 {
            returns.push(vec![]);
            continue;
        }
        let bars = &cs.candles_15m[cs.candles_15m.len() - lookback - 1..];
        let mut rets = Vec::with_capacity(bars.len() - 1);
        for w in bars.windows(2) {
            rets.push((w[1].c - w[0].c) / w[0].c);
        }
        returns.push(rets);
    }

    // Compute sum of all pairwise correlations
    let mut total_corr = 0.0;
    let mut pair_count = 0;
    for i in 0..n {
        for j in (i+1)..n {
            if returns[i].len() >= 5 && returns[j].len() >= 5 {
                total_corr += pearson_corr(&returns[i], &returns[j]);
                pair_count += 1;
            }
        }
    }

    if pair_count == 0 { return 0.0; }
    total_corr / pair_count as f64
}

fn pearson_corr(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.len() < 5 { return 0.0; }
    let n = a.len() as f64;
    let (sum_a, sum_b, sum_ab, sum_a2, sum_b2) = a.iter().zip(b.iter())
        .fold((0.0, 0.0, 0.0, 0.0, 0.0), |(sa,sb,sab,sa2,sb2), (x,y)| {
            (sa+x, sb+y, sab+x*y, sa2+x*x, sb2+y*y)
        });
    let num = n*sum_ab - sum_a*sum_b;
    let den = ((n*sum_a2-sum_a*sum_a)*(n*sum_b2-sum_b*sum_b)).sqrt();
    if den == 0.0 { 0.0 } else { num/den }
}

/// Get the correlation-adjusted deployment multiplier.
fn corr_deployment_mult(state: &SharedState) -> f64 {
    if !config::CORR_RISK_LIMIT_ENABLE { return 1.0; }

    let avg_corr = state.smoothed_avg_corr;
    if avg_corr >= config::CORR_CRITICAL {
        config::CORR_CRITICAL_MULT
    } else if avg_corr >= config::CORR_RISK_THRESHOLD {
        config::CORR_DEPLOY_MULT
    } else {
        1.0
    }
}

// In the coordinator tick — update smoothed correlation:
pub fn update_corr_risk(state: &mut SharedState) {
    let raw_corr = compute_avg_correlation(state, config::CORR_LOOKBACK);
    state.smoothed_avg_corr = state.smoothed_avg_corr * config::CORR_SMOOTHING
        + raw_corr * (1.0 - config::CORR_SMOOTHING);
}

// In open_position — apply correlation multiplier to regime trades:
let corr_mult = corr_deployment_mult(state);
let risk = if trade_type == TradeType::Regime {
    config::RISK * corr_mult  // reduce risk when correlation is elevated
} else {
    config::RISK
};
let trade_amt = cs.bal * risk;
```

---

## Validation Method

### RUN100.1 — Correlation Risk Limit Grid Search (Rust, 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed capital deployment, no correlation adjustment

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `CORR_RISK_THRESHOLD` | [0.50, 0.60, 0.70] |
| `CORR_DEPLOY_MULT` | [0.60, 0.70, 0.80] |
| `CORR_CRITICAL` | [0.70, 0.75, 0.80] |
| `CORR_CRITICAL_MULT` | [0.40, 0.50, 0.60] |

**Note:** Portfolio-level optimization. Total configs: 3 × 3 × 3 × 3 = 81.

**Key metrics:**
- `avg_deployed_mult`: average capital deployment multiplier
- `corr_activation_rate`: % of bars where correlation risk limit is active
- `PF_delta`: profit factor change vs baseline
- `max_DD_delta`: max drawdown change vs baseline
- `total_PnL_delta`: P&L change vs baseline

### RUN100.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best threshold/mult combinations per window
2. Test: evaluate on held-out month

**Pass criteria:**
- Portfolio P&L delta ≥ 0 vs baseline
- Max drawdown reduced vs baseline
- Correlation risk limit activates at least 10% of bars

### RUN100.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed deploy) | Correlation Risk Limit | Delta |
|--------|---------------------------|----------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Avg Deployed Capital | 100% | X% | -Y% |
| Corr Activation Rate | 0% | X% | — |
| High-Corr Period Return | $X | $X | +$X |

---

## Why This Could Fail

1. **Correlation is backward-looking:** By the time correlation spikes, the drawdown may already be happening. The deployment reduction arrives late.
2. **Reduces gains during rallies:** When all coins are moving up together (high correlation), we're reducing exposure just when the portfolio is doing well. This caps upside.
3. **Hard to calibrate:** The threshold (0.60) and multiplier (0.70) are guesses. Optimal values may vary across market regimes.

---

## Why It Could Succeed

1. **High correlation = concentrated risk:** When all coins move together, the portfolio isn't diversified. Reducing exposure is the correct risk management response.
2. **Preserves capital during stress:** The periods of highest correlation are often the periods of market stress. Reducing deployment during these periods prevents the largest drawdowns.
3. **Institutional practice:** Risk parity and correlation-adjusted portfolio limits are standard in institutional portfolio management.
4. **Simple and portfolio-level:** No per-coin logic — just one multiplier applied to all regime risk simultaneously.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN100 Correlation Risk Limit |
|--|--|--|
| Capital deployment | Fixed 100% | Scales 50-100% based on correlation |
| Correlation awareness | None | Average pairwise correlation |
| High-corr deployment | 100% | 50-70% |
| Low-corr deployment | 100% | 100% |
| Drawdown protection | Per-coin SL only | Portfolio-level capital reduction |
| Diversification assumption | Always valid | Only when correlation is low |
