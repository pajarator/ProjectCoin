# RUN310 — Price Ladder Acceptance: Sequential Level Strength

## Hypothesis

**Mechanism**: Define a ladder of price levels at fixed percentage intervals (e.g., every 0.5%). Price "accepts" a level when it closes above/below it on N consecutive bars. "Rejects" a level when it touches it but fails to close through. Acceptance of a level above price = bullish pressure (breakout continuation). Rejection from a level below price = resistance exhaustion = SHORT. The key is sequential acceptance: if price accepted level 1 then 2 then 3 → strong upward momentum.

**Why not duplicate**: RUN257 uses session H-L rejection (touching high/low). RUN264 uses POC rejection. RUN256 uses supply/demand zones. No RUN uses sequential price ladder logic — the concept of consecutive closes above/below fixed percentage levels as a momentum signal is unique.

## Proposed Config Changes (config.rs)

```rust
// ── RUN310: Price Ladder Acceptance ──────────────────────────────────────────
// ladder_step = 0.005 (0.5% between rungs)
// acceptance = N consecutive closes above/below a level
// LONG: price accepts level_above AND prior level also accepted
// SHORT: price rejects from level_below AND prior level also rejected
// rejection = price touches level but fails to close through

pub const LADDER_ENABLED: bool = true;
pub const LADDER_STEP: f64 = 0.005;          // 0.5% between levels
pub const LADDER_N_LEVELS: usize = 5;       // number of levels to track
pub const LADDER_ACCEPTANCE: u32 = 2;       // consecutive closes to accept
pub const LADDER_SL: f64 = 0.005;
pub const LADDER_TP: f64 = 0.004;
pub const LADDER_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run310_1_ladder_backtest.py)
2. **Walk-forward** (run310_2_ladder_wf.py)
3. **Combined** (run310_3_combined.py)

## Out-of-Sample Testing

- STEP sweep: 0.003 / 0.005 / 0.008
- ACCEPTANCE sweep: 1 / 2 / 3
- N_LEVELS sweep: 3 / 5 / 8
