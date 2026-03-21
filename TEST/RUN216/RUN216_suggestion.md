# RUN216 — Turtle Trading System: Classic Donchian Channel Breakout

## Hypothesis

**Mechanism**: The Turtle System = buy when price breaks above the highest high of the last N periods, sell when it breaks below the lowest low of the last M periods. Entry: price crosses above 20-period Donchian upper band. Exit: price crosses below 10-period Donchian lower band. This is a pure momentum trend-following system — it rides trends until they reverse.

**Why not duplicate**: No prior RUN implements the Turtle System specifically. RUN189 (Donchian) and RUN189 (Price Channel) use similar concepts but don't implement the Turtle's specific N=20 entry / M=10 exit structure with ATR-based position sizing.

## Proposed Config Changes (config.rs)

```rust
// ── RUN216: Turtle Trading System ───────────────────────────────────────
// Entry: price crosses above highest high of last entry_period bars
// Exit: price crosses below lowest low of last exit_period bars
// entry_period = 20 (classic Turtle)
// exit_period = 10 (classic Turtle)
// ATR-based position sizing: risk = 2% of equity, stop = 2× ATR

pub const TURTLE_ENABLED: bool = true;
pub const TURTLE_ENTRY_PERIOD: usize = 20;   // Donchian entry lookback
pub const TURTLE_EXIT_PERIOD: usize = 10;     // Donchian exit lookback
pub const TURTLE_RISK_PCT: f64 = 0.02;        // risk per trade (2%)
pub const TURTLE_ATR_MULT: f64 = 2.0;         // stop = ATR × this
pub const TURTLE_MAX_HOLD: u32 = 96;         // ~24 hours at 15m
```

---

## Validation Method

1. **Historical backtest** (run216_1_turtle_backtest.py)
2. **Walk-forward** (run216_2_turtle_wf.py)
3. **Combined** (run216_3_combined.py)

## Out-of-Sample Testing

- ENTRY_PERIOD sweep: 15 / 20 / 30 / 40
- EXIT_PERIOD sweep: 5 / 10 / 15
- ATR_MULT sweep: 1.5 / 2.0 / 2.5
