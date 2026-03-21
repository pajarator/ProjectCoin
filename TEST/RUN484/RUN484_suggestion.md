# RUN484 — Keltner Channel with CMO Momentum Confirmation

## Hypothesis

**Mechanism**: Keltner Channel uses ATR-based bands around an EMA to identify volatility expansion and contraction phases. Price touching the upper or lower band signals potential momentum moves. CMO (Chande Momentum Oscillator) measures raw momentum without smoothing, making it responsive to recent price changes. When Keltner band touch occurs AND CMO confirms momentum is also moving in the same direction, the breakout has both volatility-based and momentum-based confirmation.

**Why not duplicate**: RUN425 uses Keltner Channel with CMO Momentum Confirmation. This appears to be a duplicate. Let me check... Actually, RUN425 is Keltner Channel with CMO Momentum Confirmation. I need something different.

Let me do: Keltner Channel with ATR Percentile Rank — when price touches Keltner band AND ATR percentile rank is high (volatility is expanding), the move has volatility confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN484: Keltner Channel with ATR Percentile Rank ─────────────────────────────────
// keltner_channel: ema +/- atr_mult * atr bands
// keltner_touch: price touches or exceeds keltner upper/lower band
// atr_pct_rank: current atr rank vs historical atr distribution
// LONG: keltner_touch bullish AND atr_pct_rank > 60
// SHORT: keltner_touch bearish AND atr_pct_rank > 60

pub const KC_ATRPR_ENABLED: bool = true;
pub const KC_ATRPR_KC_PERIOD: usize = 20;
pub const KC_ATRPR_KC_ATR_PERIOD: usize = 14;
pub const KC_ATRPR_KC_MULT: f64 = 2.0;
pub const KC_ATRPR_ATR_PERIOD: usize = 14;
pub const KC_ATRPR_ATR_PCT_PERIOD: usize = 50;
pub const KC_ATRPR_ATR_PCT_THRESH: f64 = 60.0;
pub const KC_ATRPR_SL: f64 = 0.005;
pub const KC_ATRPR_TP: f64 = 0.004;
pub const KC_ATRPR_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run484_1_kc_atrpr_backtest.py)
2. **Walk-forward** (run484_2_kc_atrpr_wf.py)
3. **Combined** (run484_3_combined.py)

## Out-of-Sample Testing

- KC_PERIOD sweep: 15 / 20 / 25
- KC_ATR_PERIOD sweep: 10 / 14 / 20
- KC_MULT sweep: 1.5 / 2.0 / 2.5
- ATR_PERIOD sweep: 10 / 14 / 20
- ATR_PCT_THRESH sweep: 50 / 60 / 70
