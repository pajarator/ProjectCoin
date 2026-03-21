# RUN187 — Money Flow Index (MFI) Mean Reversion: Volume-Weighted Overbought/Oversold

## Hypothesis

**Mechanism**: RSI only uses price. MFI uses both price AND volume — money flow tracks the magnitude of price moves weighted by volume. High MFI with price rise = institutional accumulation (smart money pushing price up). Low MFI with price fall = distribution. MFI extremes (90+/10-) are stronger signals than RSI extremes because volume confirms the move is real, not just price noise.

**Why not duplicate**: No prior RUN uses MFI. All prior overbought/oversold RUNs use RSI (RUN169, RUN169_2). MFI is a fundamentally different signal because it incorporates volume_confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN187: Money Flow Index Mean Reversion ─────────────────────────────
// MFI = 100 - (100 / (1 + money_ratio))
// money_ratio = positive_money_flow / negative_money_flow
// Typical money flow: sum of (typical_price × volume) where typical_price rises
// MFI > 80 = overbought (distribution), MFI < 20 = oversold (accumulation)
// MFI mean-reversion: when MFI crosses 50 from below → bullish, from above → bearish

pub const MFI_ENABLED: bool = true;
pub const MFI_PERIOD: usize = 14;        // MFI lookback period
pub const MFI_OVERSOLD: f64 = 20.0;      // oversold threshold (accumulation)
pub const MFI_OVERBOUGHT: f64 = 80.0;     // overbought threshold (distribution)
pub const MFI_NEUTRAL_LOW: f64 = 40.0;   // below 40 = bearish territory
pub const MFI_NEUTRAL_HIGH: f64 = 60.0;  // above 60 = bullish territory
pub const MFI_SL: f64 = 0.005;
pub const MFI_TP: f64 = 0.004;
pub const MFI_MAX_HOLD: u32 = 36;
```

Add in `indicators.rs`:

```rust
pub fn mfi(highs: &[f64], lows: &[f64], closes: &[f64], volumes: &[f64], period: usize) -> f64 {
    if closes.len() < period + 1 || volumes.is_empty() {
        return 50.0; // neutral
    }

    let mut positive_flow = 0.0;
    let mut negative_flow = 0.0;

    for i in 1..closes.len() {
        let typical_curr = (highs[i] + lows[i] + closes[i]) / 3.0;
        let typical_prev = (highs[i-1] + lows[i-1] + closes[i-1]) / 3.0;
        let money_flow = typical_curr * volumes[i];

        if typical_curr > typical_prev {
            positive_flow += money_flow;
        } else if typical_curr < typical_prev {
            negative_flow += money_flow;
        }
    }

    if negative_flow == 0.0 {
        return 100.0;
    }

    let money_ratio = positive_flow / negative_flow;
    let mfi = 100.0 - (100.0 / (1.0 + money_ratio));
    mfi
}
```

---

## Validation Method

1. **Historical backtest** (run187_1_mfi_backtest.py)
2. **Walk-forward** (run187_2_mfi_wf.py)
3. **Combined** (run187_3_combined.py)

## Out-of-Sample Testing

- PERIOD sweep: 10 / 14 / 20
- OVERSOLD sweep: 15 / 20 / 25
- OVERBOUGHT sweep: 75 / 80 / 85
- NEUTRAL_LOW sweep: 35 / 40 / 45
- NEUTRAL_HIGH sweep: 55 / 60 / 65
