# RUN395 — Volume Profile POC with VWAP Trend Alignment

## Hypothesis

**Mechanism**: Volume Profile shows the "fair price" areas based on where the most volume was traded — the Point of Control (POC) is the price level with the highest traded volume. VWAP is the volume-weighted average price for the session. When price is above both the POC AND VWAP, it indicates strong institutional bullish consensus. When both align as support, you have a high-conviction long entry. The same for the bearish case: price below POC and VWAP indicates institutional distribution.

**Why not duplicate**: RUN366 uses Random Walk Index with Trend Mode. RUN343 uses VWAP Deviation Percentile. This RUN specifically uses Volume Profile POC (a distinct price-volume relationship measure) combined with VWAP as a directional alignment tool — the distinct mechanism is requiring price to be above/below BOTH the volume-profile POC AND VWAP simultaneously for confirmation.

## Proposed Config Changes (config.rs)

```rust
// ── RUN395: Volume Profile POC with VWAP Trend Alignment ─────────────────────────────
// volume_profile: track volume traded at each price level over lookback
// poc = price level with highest volume traded (point of control)
// vwap = cumulative(price * volume) / cumulative(volume)
// bullish_alignment: close > poc AND close > vwap
// bearish_alignment: close < poc AND close < vwap
// entry on pullback to POC/VWAP cluster after initial alignment

pub const VPOC_VWAP_ENABLED: bool = true;
pub const VPOC_VWAP_VPOC_PERIOD: usize = 20;   // volume profile lookback
pub const VPOC_VWAP_VWAP_PERIOD: usize = 14;   // VWAP session period
pub const VPOC_VWAP_PULLBACK: f64 = 0.005;     // max pullback from POC to enter
pub const VPOC_VWAP_SL: f64 = 0.005;
pub const VPOC_VWAP_TP: f64 = 0.004;
pub const VPOC_VWAP_MAX_HOLD: u32 = 48;
```

---

## Validation Method

1. **Historical backtest** (run395_1_vpoc_vwap_backtest.py)
2. **Walk-forward** (run395_2_vpoc_vwap_wf.py)
3. **Combined** (run395_3_combined.py)

## Out-of-Sample Testing

- VPOC_PERIOD sweep: 14 / 20 / 30
- VWAP_PERIOD sweep: 10 / 14 / 21
- PULLBACK sweep: 0.003 / 0.005 / 0.007
