# RUN245 — ADX-Adaptive Trailing Stop: Volatility-Responsive Stop Management

## Hypothesis

**Mechanism**: The trailing stop should adapt to market volatility. When ADX > 30 (strong trend) → use a wider trailing stop (3× ATR) to avoid being stopped out by normal fluctuations. When ADX < 20 (weak/no trend) → use a tighter stop (1.5× ATR) to lock in profits. This makes the stop responsive to the market's current trend intensity.

**Why not duplicate**: No prior RUN adapts trailing stop based on ADX. All prior trailing stop RUNs use fixed ATR multipliers. ADX-adaptive stops are fundamentally different because they *respond to trend strength*, not just volatility magnitude.

## Proposed Config Changes (config.rs)

```rust
// ── RUN245: ADX-Adaptive Trailing Stop ──────────────────────────────────
// trailing_stop = entry_price - ATR × trail_mult × adx_factor
// adx_factor = if adx > 30: 1.5 (wide stop)
//              if adx < 20: 0.75 (tight stop)
//              else: 1.0 (normal)
// Activate trailing stop only after price is in profit by SL%

pub const ADX_TRAIL_ENABLED: bool = true;
pub const ADX_TRAIL_ADX_PERIOD: usize = 14;
pub const ADX_TRAIL_STRONG: f64 = 30.0;     // strong trend threshold
pub const ADX_TRAIL_WEAK: f64 = 20.0;       // weak trend threshold
pub const ADX_TRAIL_STRONG_MULT: f64 = 1.5; // ATR multiplier when ADX strong
pub const ADX_TRAIL_WEAK_MULT: f64 = 0.75;  // ATR multiplier when ADX weak
pub const ADX_TRAIL_NORMAL_MULT: f64 = 1.0;  // ATR multiplier when ADX normal
pub const ADX_TRAIL_PROFIT_LOCK: f64 = 0.003; // activate trailing after 0.3% profit
```

---

## Validation Method

1. **Historical backtest** (run245_1_adx_trail_backtest.py)
2. **Walk-forward** (run245_2_adx_trail_wf.py)
3. **Combined** (run245_3_combined.py)

## Out-of-Sample Testing

- STRONG_THRESH sweep: 25 / 30 / 35
- WEAK_THRESH sweep: 15 / 20 / 25
- STRONG_MULT sweep: 1.0 / 1.5 / 2.0
- WEAK_MULT sweep: 0.5 / 0.75 / 1.0
