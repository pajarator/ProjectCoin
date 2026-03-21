# RUN333 — Connors RSI with Streak Count: Short-Term RSI Pattern Recognition

## Hypothesis

**Mechanism**: Connors RSI = average of 3 components: (1) standard RSI, (2) streak up/down count (consecutive up/down bars), and (3) percentile rank of the current RSI. The streak component is key — 3 consecutive down closes = RSI oversold even if raw RSI isn't at extreme. When Connors RSI crosses above 30 from below AND the streak count is at least 3 → strong mean-reversion signal.

**Why not duplicate**: RUN134 mentions Connors RSI but it's listed as unexecuted. No prior RUN actually implements Connors RSI. Standard RSI RUNs use plain RSI. The streak component is what makes Connors distinct — it captures the psychological momentum of consecutive directional closes.

## Proposed Config Changes (config.rs)

```rust
// ── RUN333: Connors RSI with Streak Count ──────────────────────────────────────
// connors_rsi = (RSI(close, period) + streak_component + rsi_percentile) / 3
// streak_component = streak_count → mapped to 0-100 based on streak length
// LONG: connors_rsi crosses above RSI_OVERSOLD (typically 30)
// SHORT: connors_rsi crosses below RSI_OVERBOUGHT (typically 70)
// Streak gate: require streak_count >= STREAK_MIN before signal fires

pub const CONNORS_RSI_ENABLED: bool = true;
pub const CONNORS_RSI_PERIOD: usize = 14;
pub const CONNORS_RSI_OVERSOLD: f64 = 30.0;
pub const CONNORS_RSI_OVERBOUGHT: f64 = 70.0;
pub const CONNORS_RSI_STREAK_MIN: u32 = 3;
pub const CONNORS_RSI_PCT_RANK_PERIOD: usize = 100;
pub const CONNORS_RSI_SL: f64 = 0.005;
pub const CONNORS_RSI_TP: f64 = 0.004;
pub const CONNORS_RSI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run333_1_connors_rsi_backtest.py)
2. **Walk-forward** (run333_2_connors_rsi_wf.py)
3. **Combined** (run333_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 21
- OVERSOLD sweep: 20 / 30 / 40
- OVERBOUGHT sweep: 60 / 70 / 80
- STREAK_MIN sweep: 2 / 3 / 5
