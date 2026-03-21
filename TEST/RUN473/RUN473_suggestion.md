# RUN473 — Relative Vigor Index with Ease of Movement Divergence

## Hypothesis

**Mechanism**: Relative Vigor Index (RVI) compares closing price to trading range, measuring trend strength by how close closes are to highs/lows. Ease of Movement (EOM) relates price change to volume, showing how easily price moves on given volume. Divergence between RVI and EOM signals potential reversals: when RVI shows strength but EOM shows declining volume-backed movement, the trend lacks conviction.

**Why not duplicate**: RUN404 uses Copock Curve with Ease of Movement Divergence. This RUN uses RVI instead — distinct mechanism is RVI's close-to-range momentum measurement versus Copock's ROC-based approach. RVI directly measures closing price relative to range.

## Proposed Config Changes (config.rs)

```rust
// ── RUN473: Relative Vigor Index with Ease of Movement Divergence ─────────────────────────────────
// rvi: relative_vigor_index comparing close to high_low range
// rvi_cross: rvi crosses above/below signal line
// eom: ease_of_movement ratio of price_change to volume
// eom_divergence: rvi and eom moving in opposite directions
// LONG: rvi_cross bullish AND eom > 0 (price moving easily upward)
// SHORT: rvi_cross bearish AND eom < 0 (price moving easily downward)

pub const RVI_EOM_ENABLED: bool = true;
pub const RVI_EOM_RVI_PERIOD: usize = 14;
pub const RVI_EOM_RVI_SIGNAL: usize = 10;
pub const RVI_EOM_EOM_PERIOD: usize = 14;
pub const RVI_EOM_EOM_SMA_PERIOD: usize = 20;
pub const RVI_EOM_SL: f64 = 0.005;
pub const RVI_EOM_TP: f64 = 0.004;
pub const RVI_EOM_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run473_1_rvi_eom_backtest.py)
2. **Walk-forward** (run473_2_rvi_eom_wf.py)
3. **Combined** (run473_3_combined.py)

## Out-of-Sample Testing

- RVI_PERIOD sweep: 10 / 14 / 20
- RVI_SIGNAL sweep: 8 / 10 / 12
- EOM_PERIOD sweep: 10 / 14 / 20
- EOM_SMA_PERIOD sweep: 14 / 20 / 30
