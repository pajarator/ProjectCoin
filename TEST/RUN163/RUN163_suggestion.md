# RUN163 — Volume Profile POC Signal: High Volume Node as Dynamic Support/Resistance

## Hypothesis

**Mechanism**: In each 15m bar, the price level where the most volume traded (Point of Control / POC) acts as a magnetic price level. When price approaches the POC from below, it often bounces — POC acts as support. When price approaches from above, it often rejects — POC acts as resistance. COINCLAW has no volume profile data. Building a rolling 16-bar (4h) volume profile from Binance klines provides dynamic support/resistance levels unavailable from price-only indicators.

**Implementation**: Use Binance 1m klines to build a volume profile — group volume by price bucket (0.1% buckets), find the POC (highest volume bucket). Track POC across rolling 16-bar window.

**Why this is not a duplicate**: No prior RUN uses volume profile or POC data. All existing indicators are price-action derivatives. POC is a volume-weighted support level — mechanically distinct.

## Proposed Config Changes (config.rs)

```rust
// ── RUN163: Volume Profile POC Signal ──────────────────────────────────
// POC = price bucket with highest volume over rolling 16-bar window
// Price above POC + RSI extreme → SHORT (resistance rejection)
// Price below POC + RSI extreme → LONG (support bounce)
// Exit: price crosses POC OR MAX_HOLD bars

pub const POC_ENABLED: bool = true;
pub const POC_WINDOW_BARS: usize = 16;       // rolling window (4 hours at 15m)
pub const POC_BUCKET_SIZE: f64 = 0.001;    // 0.1% price buckets
pub const POC_RSI_THRESH: f64 = 30.0;     // for LONG bounce, RSI must be < 30
pub const POC_SL: f64 = 0.004;            // 0.4% stop
pub const POC_TP: f64 = 0.003;            // 0.3% take profit
pub const POC_MAX_HOLD: u32 = 12;          // ~3 hours
```

Add to `CoinState` in `state.rs`:

```rust
pub poc_price: f64,           // current POC price level
pub poc_price_prev: f64,      // previous bar's POC
pub volume_profile: HashMap<String, f64>,  // price_bucket -> volume
```

Add calculation in `indicators.rs`:

```rust
/// Build volume profile from candles and compute POC (Point of Control)
/// Groups volume into price buckets of POC_BUCKET_SIZE (0.1%)
pub fn compute_poc(candles_15m: &[Candle], bucket_size: f64) -> Option<f64> {
    if candles_15m.len() < 4 { return None; }
    let mut profile: HashMap<i64, f64> = HashMap::new();
    for c in candles_15m.iter().rev().take(16) {
        // Each candle contributes volume proportional to price range
        let high = c.h;
        let low = c.l;
        let vol = c.v;
        let steps = ((high - low) / (low * bucket_size)).ceil() as i64;
        if steps == 0 { continue; }
        let vol_per_step = vol / steps as f64;
        let mut price = low;
        while price < high {
            let bucket = (price / bucket_size).floor() as i64;
            *profile.entry(bucket).or_insert(0.0) += vol_per_step;
            price += low * bucket_size;
        }
    }
    profile.into_iter().max_by_key(|(_, v)| OrderedFloat(*v)).map(|(k, _)| k as f64 * bucket_size)
}
```

Add entry logic in `engine.rs`:

```rust
/// Fires when price is at POC and RSI confirms direction
fn check_poc_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::POC_ENABLED { return None; }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }

    let poc = cs.poc_price;
    if poc.is_nan() { return None; }

    let price = ind.p;
    let dist_to_poc = (price - poc).abs() / poc;

    // Price within 0.2% of POC
    if dist_to_poc > 0.002 { return None; }

    // LONG: price below POC + RSI oversold
    if price < poc && ind.rsi < config::POC_RSI_THRESH {
        return Some((Direction::Long, "poc_support"));
    }
    // SHORT: price above POC + RSI overbought
    if price > poc && ind.rsi > (100.0 - config::POC_RSI_THRESH) {
        return Some((Direction::Short, "poc_resistance"));
    }
    None
}
```

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data with 1m klines.
- **Baseline**: COINCLAW v16 (no POC signal)
- **Comparison**: POC trades tracked separately

**Metrics to measure**:
- POC bounce WR (hypothesis: >55%)
- PF on POC trades
- POC as support vs resistance accuracy

**Hypothesis**: Price bouncing off POC with RSI confirmation achieves WR >55%.

---

## Validation Method

1. **Historical backtest** (run163_1_poc_backtest.py)
2. **Walk-forward** (run163_2_poc_wf.py)
3. **Combined comparison** (run163_3_combined.py)

---

## Out-of-Sample Testing

- POC_WINDOW sweep: 8 / 16 / 24 bars
- POC_BUCKET sweep: 0.001 / 0.002 / 0.005
- RSI_THRESH sweep: 25 / 30 / 35
