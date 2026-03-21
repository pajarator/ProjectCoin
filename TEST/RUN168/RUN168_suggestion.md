# RUN168 — Consecutive Candle Exhaustion Filter: N Same-Direction Bars as Reversal Signal

## Hypothesis

**Mechanism**: When N consecutive bars close in the same direction (e.g., 4 consecutive green candles), momentum is exhausted and mean-reversion probability rises. N consecutive bearish candles → price oversold → LONG. N consecutive bullish candles → price overbought → SHORT. COINCLAW has no consecutive-bar counter.

**Why not duplicate**: No prior RUN uses consecutive bar counting. RUN135 (Stress Accumulation Meter) tracks consecutive directional bars but was proposed (unexecuted). This RUN adds a threshold-based filter that blocks or triggers entries based on consecutive bar count.

## Proposed Config Changes (config.rs)

```rust
// ── RUN168: Consecutive Candle Exhaustion Filter ───────────────────────
// LONG block: 4+ consecutive green bars → block LONG (momentum exhausted)
// SHORT block: 4+ consecutive red bars → block SHORT (momentum exhausted)
// OR: Reverse trigger: 4+ consecutive green → SHORT (exhaustion reversal)

pub const CONSEC_ENABLED: bool = true;
pub const CONSEC_BARS: usize = 4;           // N consecutive bars to trigger
pub const CONSEC_MODE: &'static str = "block";  // "block" or "reverse"
```

Add to `CoinState` in `state.rs`:

```rust
pub consecutive_green_bars: usize,
pub consecutive_red_bars: usize,
```

Add logic in `engine.rs`:

```rust
fn update_consecutive_bars(cs: &mut CoinState, candle: &Candle) {
    if candle.c > candle.o {
        cs.consecutive_green_bars += 1;
        cs.consecutive_red_bars = 0;
    } else if candle.c < candle.o {
        cs.consecutive_red_bars += 1;
        cs.consecutive_green_bars = 0;
    } else {
        cs.consecutive_green_bars = 0;
        cs.consecutive_red_bars = 0;
    }
}

fn consecutive_filter(cs: &CoinState, proposed_dir: Direction) -> bool {
    if !config::CONSEC_ENABLED { return true; }
    match config::CONSEC_MODE {
        "block" => {
            if proposed_dir == Direction::Long && cs.consecutive_green_bars >= config::CONSEC_BARS {
                return false;  // block long after exhaustion
            }
            if proposed_dir == Direction::Short && cs.consecutive_red_bars >= config::CONSEC_BARS {
                return false;  // block short after exhaustion
            }
        }
        "reverse" => {
            if cs.consecutive_green_bars >= config::CONSEC_BARS {
                return false;  // reverse: reject long after exhaustion
            }
            if cs.consecutive_red_bars >= config::CONSEC_BARS {
                return false;  // reverse: reject short after exhaustion
            }
        }
        _ => {}
    }
    true
}
```

---

## Validation Method

1. **Historical backtest** (run168_1_consec_backtest.py): 18 coins, sweep CONSEC_BARS
2. **Walk-forward** (run168_2_consec_wf.py): 3-window walk-forward
3. **Combined** (run168_3_combined.py): vs baseline

## Out-of-Sample Testing

- BARS sweep: 3 / 4 / 5 / 6
- MODE: block vs reverse
