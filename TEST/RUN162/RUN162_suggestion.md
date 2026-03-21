# RUN162 — BTC Dominance Trend Reversal Filter: Dominance SMA Crossover as Market Regime Signal

## Hypothesis

**Mechanism**: BTC Dominance (BTCD) has a long-term trend. When BTCD crosses its 20-bar SMA (on the 4h timeframe), it signals a regime change in the broader crypto market. During BTCD bear trends (altseason), altcoins outperform — COINCLAW's regime LONG trades should be boosted. During BTCD bull trends, alts underperform — ISO shorts should be boosted. COINCLAW uses breadth (%) for regime but not BTCD. Adding BTCD trend tracking provides a higher-timeframe filter.

**API to use** (no external API needed — computed from existing BTC/USDT data):
- Already have BTC/USDT data in COINCLAW
- Compute BTCD from: BTC market cap / total crypto market cap
- Alternative: use Binance BTCDOM index: `GET https://api.binance.com/api/v3/ticker/price?symbol=BTCDOMUSDT` — BTCDOM is the Dominance index perpetual

**Why this is not a duplicate**: No prior RUN uses BTC Dominance as a filter. RUN43 uses breadth momentum. RUN20/21 use sentiment. BTCD is a market structure signal orthogonal to all tested signals.

## Proposed Config Changes (config.rs)

```rust
// ── RUN162: BTC Dominance Trend Reversal Filter ──────────────────────
// BTCD trend: 4h SMA(20) crossover of BTCD
// BTCD above SMA20 = bull trend → boost SHORT trades, reduce LONG trades
// BTCD below SMA20 = bear trend → boost LONG trades, reduce ISO_SHORT trades
// Applied as: multiply RISK by BTCD_TREND_MULT when entering in boosted direction

pub const BTCD_ENABLED: bool = true;
pub const BTCD_USE_INDEX: bool = false;      // true = use BTCDOM perpetual, false = compute from BTC data
pub const BTCD_SMA_PERIOD: usize = 20;       // 20-bar SMA on 4h = ~80h
pub const BTCD_BULL_MULT: f64 = 1.25;      // multiply RISK by 1.25x for SHORT in BTCD bull
pub const BTCD_BEAR_MULT: f64 = 1.25;      // multiply RISK by 1.25x for LONG in BTCD bear
```

Add to `SharedState` in `state.rs`:

```rust
pub btcd_price: f64,          // BTCDOM price or computed BTCD
pub btcd_sma20: f64,
pub btcd_history: Vec<f64>,
pub btcd_trend: BTCDTrend,    // Bull, Bear, or Neutral
```

```rust
enum BTCDTrend { Bull, Bear, Neutral }

fn compute_btcd_trend(state: &mut SharedState) {
    let btcd = state.btcd_price;
    state.btcd_history.push(btcd);
    if state.btcd_history.len() > config::BTCD_SMA_PERIOD {
        state.btcd_history.remove(0);
    }
    let sma = state.btcd_history.iter().sum::<f64>() / state.btcd_history.len() as f64;
    state.btcd_sma20 = sma;
    state.btcd_trend = if btcd > sma { BTCDTrend::Bull }
        else if btcd < sma { BTCDTrend::Bear }
        else { BTCDTrend::Neutral };
}
```

Modify position sizing in `open_position`:

```rust
fn effective_risk_for_btcd(trade_type: TradeType, dir: Direction, state: &SharedState) -> f64 {
    let base = match trade_type {
        TradeType::Regime | TradeType::Momentum => config::RISK,
        TradeType::Scalp => config::SCALP_RISK,
        _ => config::RISK,
    };
    if !config::BTCD_ENABLED { return base; }

    match (&state.btcd_trend, &dir) {
        (BTCDTrend::Bull, Direction::Short) => base * config::BTCD_BULL_MULT,
        (BTCDTrend::Bear, Direction::Long) => base * config::BTCD_BEAR_MULT,
        _ => base,
    }
}
```

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data with 4h BTCD data.
- **Baseline**: COINCLAW v16 (no BTCD filter)
- **Comparison**: BTCD-adjusted sizing vs fixed sizing

**Metrics to measure**:
- Portfolio Sharpe improvement
- % of trades boosted by BTCD filter
- BTCD trend accuracy as market regime predictor

**Hypothesis**: BTCD above its 20-bar SMA (bull regime) → SHORT entries boosted by 1.25x improve portfolio Sharpe >10%.

---

## Validation Method

1. **Historical backtest** (run162_1_btcd_backtest.py):
   - 18 coins, 1-year 15m data + 4h BTCD data
   - Simulate COINCLAW v16 with and without BTCD risk multiplier
   - Record: BTCD trend, direction, position size, P&L
   - Output: per-coin Sharpe, portfolio Sharpe comparison

2. **Walk-forward** (run162_2_btcd_wf.py):
   - 3-window walk-forward
   - Sweep BTCD_BULL_MULT: 1.0 / 1.25 / 1.5
   - Sweep BTCD_BEAR_MULT: 1.0 / 1.25 / 1.5

3. **Combined comparison** (run162_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + btcd_filter

---

## Out-of-Sample Testing

- MULT sweep: 1.0 / 1.25 / 1.5
- SMA_PERIOD sweep: 10 / 20 / 40
- OOS: final 4 months held out
