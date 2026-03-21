# RUN284 — Parabolic SAR with ADX Trend Confirmation

## Hypothesis

**Mechanism**: Parabolic SAR alone can whipsaw in choppy markets. Add ADX filter: only take LONG when SAR flips bullish AND ADX > 25 (strong trend). Only take SHORT when SAR flips bearish AND ADX > 25. When ADX < 20 → no trend → ignore SAR signals.

**Why not duplicate**: RUN192 uses Parabolic SAR alone. RUN284 adds ADX confirmation — a distinct filter that reduces whipsaws by requiring trend strength.

## Proposed Config Changes (config.rs)

```rust
// ── RUN284: Parabolic SAR with ADX Filter ────────────────────────────────
// sar = parabolic stop and reverse
// LONG: sar flips bullish AND ADX > 25
// SHORT: sar flips bearish AND ADX > 25
// NO TRADE: ADX < 20

pub const PSAR_ADX_ENABLED: bool = true;
pub const PSAR_ADX_AF_START: f64 = 0.02;
pub const PSAR_ADX_AF_MAX: f64 = 0.20;
pub const PSAR_ADX_ADX_PERIOD: usize = 14;
pub const PSAR_ADX_ADX_STRONG: f64 = 25.0;
pub const PSAR_ADX_ADX_WEAK: f64 = 20.0;
pub const PSAR_ADX_SL: f64 = 0.005;
pub const PSAR_ADX_TP: f64 = 0.004;
pub const PSAR_ADX_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run284_1_psar_adx_backtest.py)
2. **Walk-forward** (run284_2_psar_adx_wf.py)
3. **Combined** (run284_3_combined.py)

## Out-of-Sample Testing

- AF_START sweep: 0.01 / 0.02 / 0.03
- AF_MAX sweep: 0.15 / 0.20 / 0.25
- ADX_STRONG sweep: 20 / 25 / 30
