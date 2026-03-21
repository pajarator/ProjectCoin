# RUN212 — Market Facilitation Index (MFI) 4-State Classification

## Hypothesis

**Mechanism**: MFI = (high - low) / volume × 100,000. Combined with volume trend creates 4 market states:
- **Green** (MFI up + volume up): strong trending market → ride the trend
- **Fade** (MFI down + volume down): trend exhausted → exit
- **Fake** (MFI up + volume down): unsustainable move → fade it
- **Squat** (MFI down + volume up): absorption → price about to explode → prepare for breakout

Trade with Green (momentum), fade Fake (mean-reversion), and wait through Squat.

**Why not duplicate**: No prior RUN uses the MFI 4-state classification. All prior volume+volatility RUNs treat these as separate signals. The MFI 4-state system is a holistic market classification that synthesizes both dimensions simultaneously.

## Proposed Config Changes (config.rs)

```rust
// ── RUN212: Market Facilitation Index (MFI) 4-State ─────────────────────
// mfi = (high - low) / volume × 1_000_000
// GREEN: mfi > mfi_ma AND volume > vol_ma → trend following
// FAKE: mfi > mfi_ma AND volume < vol_ma → mean-revert
// SQUAT: mfi < mfi_ma AND volume > vol_ma → breakout酝酿
// FADE: mfi < mfi_ma AND volume < vol_ma → no trade / exit

pub const MFI4_ENABLED: bool = true;
pub const MFI_PERIOD: usize = 14;           // MFI smoothing
pub const MFI_VOL_MA: usize = 20;           // volume MA period
pub const MFI_SL: f64 = 0.005;
pub const MFI_TP: f64 = 0.004;
pub const MFI_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run212_1_mfi4_backtest.py)
2. **Walk-forward** (run212_2_mfi4_wf.py)
3. **Combined** (run212_3_combined.py)

## Out-of-Sample Testing

- MFI_PERIOD sweep: 10 / 14 / 20
- MFI_VOL_MA sweep: 14 / 20 / 30
