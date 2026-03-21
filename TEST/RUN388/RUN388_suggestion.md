# RUN388 — Price Ladder with Momentum Rotation Filter

## Hypothesis

**Mechanism**: The Price Ladder concept tracks how price acceptance moves through discrete price levels — if price is repeatedly rejected at a level, that level becomes a resistance. Momentum Rotation measures which coins in the portfolio are leading vs lagging. When a coin's price is accepted at a new ladder level (breaks through a rejection zone) AND its momentum rotation rank is rising (it's becoming a leader), the signal has both price-structure and cross-coin momentum confirmation.

**Why not duplicate**: RUN310 uses Price Ladder Acceptance. RUN370 uses Price Momentum Rotation with Breadth. This RUN specifically combines Price Ladder structure (acceptance at discrete levels) with Momentum Rotation rank change — the distinct mechanism is using cross-coin momentum rank improvement to confirm price structure breakouts.

## Proposed Config Changes (config.rs)

```rust
// ── RUN388: Price Ladder with Momentum Rotation Filter ─────────────────────────
// price_ladder: track price acceptance at discrete levels (e.g., $100 increments)
// ladder_break: price closes above/below a level after N rejections
// momentum_rotation: rank of coin's ROC within the 18-coin universe
// rotation_rising: momentum rank improving (coin becoming a leader)
// LONG: ladder_break to upside AND momentum_rotation rising
// SHORT: ladder_break to downside AND momentum_rotation falling

pub const LADDER_MOM_ROT_ENABLED: bool = true;
pub const LADDER_MOM_ROT_PERIOD: usize = 20;      // ladder period (price increments)
pub const LADDER_MOM_ROT_REJECTIONS: u32 = 2;    // N rejections before break confirmed
pub const LADDER_MOM_ROT_MOM_PERIOD: usize = 14; // ROC period for momentum
pub const LADDER_MOM_ROT_SL: f64 = 0.005;
pub const LADDER_MOM_ROT_TP: f64 = 0.004;
pub const LADDER_MOM_ROT_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run388_1_ladder_mom_backtest.py)
2. **Walk-forward** (run388_2_ladder_mom_wf.py)
3. **Combined** (run388_3_combined.py)

## Out-of-Sample Testing

- LADDER_PERIOD sweep: 15 / 20 / 30
- REJECTIONS sweep: 2 / 3 / 4
- MOM_PERIOD sweep: 10 / 14 / 21
