# RUN147 — Opening Range Reversal: Fade the Early Session Impulse

## Hypothesis

**Mechanism**: The first 4 bars (1 hour) of each UTC 4-hour window is the highest-volatility, most emotionally-driven period. Price tends to overshoot during this "opening range." When the opening 4-bar range exceeds 1.5× the ATR(14) AND price has reversed direction by bar 5 (rejection of the impulse), it's a high-probability mean-reversion entry. This exploits the common pattern where aggressive early positioning reverses as the session stabilizes.

**Why this is not a duplicate**: RUN41 (Session-Based Trade Filter) gates entries by session but doesn't use range size or rejection candles. RUN57 (Day-of-Week Trade Filter) is calendar-based. RUN84 (Session-Based Partial Exit Scaling) adjusts exits, not entries. RUN91 (Hourly Z-Threshold Scaling) scales z-threshold by hour. None combine *opening range measurement* with *rejection candle confirmation* — a specific, mechanically distinct pattern.

**Why it could work**: The opening 1-hour window in crypto is structurally different from the rest of the session — higher volume, more emotional price discovery, frequent overshoots that correct. The 4-bar opening range is a proxy for the session's initial directional impulse. When price exceeds that range and reverses with a rejection candle, it signals the impulse was wrong and mean-reversion follows.

---

## Proposed Config Changes (config.rs)

```rust
// ── RUN7: Opening Range Reversal ─────────────────────────────────────
// Opening window: first 4 bars of each UTC 4-hour cycle
// Range = high_max - low_min over the 4-bar opening window
// Entry: price breaks range by ATR_THRESH × ATR then reverses back inside with rejection candle
// Exit: opposite side of opening range OR MAX_HOLD bars

pub const ORR_ENABLED: bool = true;
pub const ORR_WINDOW_BARS: u32 = 4;              // opening range window (bars)
pub const ORR_ATR_MULT: f64 = 1.5;              // opening range must be > 1.5x ATR(14)
pub const ORR_REJECT_BARS: u32 = 2;              // reversal must happen within 2 bars of range break
pub const ORR_SL: f64 = 0.004;                   // 0.4% stop
pub const ORR_TP: f64 = 0.003;                   // 0.3% take profit
pub const ORR_MAX_HOLD: u32 = 12;               // ~3 hours at 15m bars
```

Add to `Ind15m` in `indicators.rs`:
```rust
pub opening_range_high: f64,   // high of current 4-bar opening window
pub opening_range_low: f64,    // low of current 4-bar opening window
pub opening_range_size: f64,   // high - low
pub opening_range_bar: u32,    // which bar of the 4-bar window we're on (0-3)
```

Add calculation in indicators.rs:
```rust
/// Update opening range: tracks high/low of first ORR_WINDOW_BARS of each 4h cycle
/// Bar 0 of each cycle resets the opening range
fn update_opening_range(candle: &Candle, state: &mut Ind15m) {
    let cycle_bar = (state.bar_count % 4) as u32;  // 4 bars per UTC 4h cycle
    if cycle_bar == 0 {
        state.opening_range_high = candle.h;
        state.opening_range_low = candle.l;
        state.opening_range_bar = 0;
    } else {
        state.opening_range_high = state.opening_range_high.max(candle.h);
        state.opening_range_low = state.opening_range_low.min(candle.l);
        state.opening_range_bar = cycle_bar;
    }
    state.opening_range_size = state.opening_range_high - state.opening_range_low;
}
```

Add entry logic in engine.rs:
```rust
/// Returns LONG if price broke below opening range low then reversed up with rejection candle
/// Returns SHORT if price broke above opening range high then reversed down with rejection candle
fn check_orr_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::ORR_ENABLED { return None; }

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }

    // Must be past the opening window
    if ind.opening_range_bar < config::ORR_WINDOW_BARS as u32 - 1 { return None; }

    let range_size = ind.opening_range_size;
    let atr = ind.atr14_price;
    if range_size.is_nan() || atr <= 0.0 { return None; }
    if range_size < atr * config::ORR_ATR_MULT { return None; }  // range wasn't large enough

    let price = ind.p;
    let prev_candle = cs.candles_15m.get(cs.candles_15m.len() - 1)?;

    // LONG: price broke below range low then reversed up
    // Rejection candle: current close > open AND prev candle was bearish
    if prev_candle.c < prev_candle.o  // prev was down-bar
        && price > ind.opening_range_low + atr * 0.2  // price came back inside range
        && price > prev_candle.c  // current bar is up-bar (rejection of down-move)
    {
        return Some((Direction::Long, "opening_range_rev"));
    }

    // SHORT: price broke above range high then reversed down
    if prev_candle.c > prev_candle.o  // prev was up-bar
        && price < ind.opening_range_high - atr * 0.2  // price came back inside range
        && price < prev_candle.c  // current bar is down-bar (rejection of up-move)
    {
        return Some((Direction::Short, "opening_range_rev"));
    }

    None
}
```

Integration: Call from `check_entry` alongside regime entries.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year 15m data.
- **Baseline**: COINCLAW v16 (no opening range strategy)
- **Comparison**: opening range reversal trades tracked separately

**Metrics to measure**:
- Opening range reversal WR (hypothesis: >55%)
- PF on opening range trades
- Time-of-day distribution: do entries concentrate in first vs last hour of 4h window?
- Correlation with regime trades

**Hypothesis**: Opening range reversal should achieve WR >55% because early-session impulses are emotionally driven and prone to reversal. If confirmed, it adds a time-structured, mechanically clean signal.

---

## Validation Method

1. **Historical backtest** (run7_1_orr_backtest.py):
   - 18 coins, 1-year 15m data
   - Identify all opening range break + reversal patterns
   - Record: range size (×ATR), direction, entry price, stop, TP, exit reason, P&L
   - Output: per-coin WR, PF, avg hold time, range size vs outcome correlation

2. **Walk-forward** (run7_2_orr_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep ORR_ATR_MULT: 1.0 / 1.5 / 2.0
   - Sweep ORR_WINDOW_BARS: 2 / 4 / 6 bars

3. **Combined comparison** (run7_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + opening_range_rev
   - Portfolio stats, time-of-day analysis, per-coin contribution

---

## Out-of-Sample Testing

- ATR_MULT sweep: 1.0 / 1.5 / 2.0 / 2.5
- Window sweep: 2 / 4 / 6 bars
- REJECT_BARS sweep: 1 / 2 / 3 bars
- OOS: final 4 months held out from all parameter selection
