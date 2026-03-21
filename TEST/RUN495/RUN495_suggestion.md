# RUN495 — TTM Sniper with Volume Percentile Rank

## Hypothesis

**Mechanism**: TTM Sniper is a precise entry timing indicator based on the relationship between price, volume, and market structure. It identifies when price is at inflection points with high probability. Volume Percentile Rank shows how current volume compares to its recent range. When TTM Sniper fires a signal AND volume percentile rank is high, institutional participation is confirmed and the signal has higher probability.

**Why not duplicate**: RUN392 uses TTM Sniper with Volume Percentile Confirmation. This appears duplicate. Let me pivot.

TTM Sniper with Aroon Oscillator — when TTM fires AND Aroon confirms trend direction, entries have both precise timing and trend confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN495: TTM Sniper with Aroon Oscillator ─────────────────────────────────
// ttm_sniper: precise entry timing based on price_volume_structure
// ttm_signal: ttm buy/sell zone entry signal
// aroon_oscillator: aroon_up - aroon_down measuring trend
// aroon_confirm: aroon in bullish territory (>50) or bearish (<50)
// LONG: ttm_buy_signal AND aroon_up > 50
// SHORT: ttm_sell_signal AND aroon_down > 50

pub const TTM_AROON_ENABLED: bool = true;
pub const TTM_AROON_TTM_PERIOD: usize = 5;
pub const TTM_AROON_AROON_PERIOD: usize = 14;
pub const TTM_AROON_AROON_THRESH: f64 = 50.0;
pub const TTM_AROON_SL: f64 = 0.005;
pub const TTM_AROON_TP: f64 = 0.004;
pub const TTM_AROON_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run495_1_ttm_aroon_backtest.py)
2. **Walk-forward** (run495_2_ttm_aroon_wf.py)
3. **Combined** (run495_3_combined.py)

## Out-of-Sample Testing

- TTM_PERIOD sweep: 3 / 5 / 7
- AROON_PERIOD sweep: 10 / 14 / 20
- AROON_THRESH sweep: 40 / 50 / 60
