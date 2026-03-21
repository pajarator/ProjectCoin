# RUN293 — Opening Gap Fade: Overnight Disconnect Mean Reversion

## Hypothesis

**Mechanism**: Crypto has no true "open" but we can treat session boundaries. When price gaps more than 1% from the prior session's close → the gap is likely to fill. Long the gap if price gapped down. Short the gap if price gapped up. Gaps are filled by the end of the session.

**Why not duplicate**: No prior RUN uses opening gaps. All prior session RUNs use range or time-of-day. Opening gap fade is distinct because it specifically targets the *gap* from the prior session, not the session range.

## Proposed Config Changes (config.rs)

```rust
// ── RUN293: Opening Gap Fade ─────────────────────────────────────────────
// session_boundary = 00:00 UTC (use 4h bar close as proxy)
// gap_size = (open - prior_close) / prior_close
// LONG: gap_size < -0.01 (gapped down > 1%) → expect fill
// SHORT: gap_size > 0.01 (gapped up > 1%) → expect fill

pub const GAP_FADE_ENABLED: bool = true;
pub const GAP_FADE_THRESH: f64 = 0.01;       // 1% gap threshold
pub const GAP_FADE_SL: f64 = 0.005;
pub const GAP_FADE_TP: f64 = 0.004;
pub const GAP_FADE_MAX_HOLD: u32 = 32;
```

---

## Validation Method

1. **Historical backtest** (run293_1_gap_fade_backtest.py)
2. **Walk-forward** (run293_2_gap_fade_wf.py)
3. **Combined** (run293_3_combined.py)

## Out-of-Sample Testing

- THRESH sweep: 0.005 / 0.01 / 0.015
