# RUN483 — Price Ladder RSI Pullback with Volume Surge

## Hypothesis

**Mechanism**: Price Ladder visualizes price action across multiple time frames or price levels, identifying where buying/selling pressure concentrates. RSI Pullback identifies when RSI pulls back from overbought/oversold to neutral, creating better entry prices. Volume Surge confirms institutional involvement at those pullback levels. When RSI pulls back from extreme AND price is at a ladder support/resistance level AND volume surges, the entry has price structure, oscillator timing, and institutional conviction.

**Why not duplicate**: RUN419 uses Price Ladder RSI Pullback with Volume Surge Confirmation. This appears duplicate. Let me pivot.

Price Ladder with Money Flow Index — when price is at ladder level AND MFI confirms buying/selling pressure, entries have both price structure and money flow conviction.

## Proposed Config Changes (config.rs)

```rust
// ── RUN483: Price Ladder with Money Flow Index ─────────────────────────────────
// price_ladder: key price levels where multiple period highs/lows cluster
// ladder_touch: price touches or approaches ladder level
// mfi: money_flow_index combining price and volume for money flow
// mfi_cross: mfi crosses above/below 50 (bullish/bearish threshold)
// LONG: price at ladder_support AND mfi > 50 AND mfi_rising
// SHORT: price at ladder_resistance AND mfi < 50 AND mfi_falling

pub const LADDER_MFI_ENABLED: bool = true;
pub const LADDER_MFI_LADDER_PERIOD: usize = 20;
pub const LADDER_MFI_LADDER_TOLERANCE: f64 = 0.001;
pub const LADDER_MFI_MFI_PERIOD: usize = 14;
pub const LADDER_MFI_MFI_THRESH: f64 = 50.0;
pub const LADDER_MFI_SL: f64 = 0.005;
pub const LADDER_MFI_TP: f64 = 0.004;
pub const LADDER_MFI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run483_1_ladder_mfi_backtest.py)
2. **Walk-forward** (run483_2_ladder_mfi_wf.py)
3. **Combined** (run483_3_combined.py)

## Out-of-Sample Testing

- LADDER_PERIOD sweep: 14 / 20 / 30
- LADDER_TOLERANCE sweep: 0.0005 / 0.001 / 0.002
- MFI_PERIOD sweep: 10 / 14 / 20
- MFI_THRESH sweep: 45 / 50 / 55
