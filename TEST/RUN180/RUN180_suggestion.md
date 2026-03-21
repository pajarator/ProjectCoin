# RUN180 — Exchange Inflow Anomaly: NVT Ratio Spike as Short-Term Top/Bottom Signal

## Hypothesis

**Mechanism**: NVT (Network Value to Transactions) ratio = market cap / daily transaction volume. When NVT spikes above its 30-day average by >50%, it signals the network is being overvalued relative to actual usage — a local top. When NVT drops below average, it's undervalued — local bottom. Applied to BTC via free API data (Blockchain.com or similar).

**Why not duplicate**: No prior RUN uses NVT ratio. All signals are price-action/derivatives.

## Proposed Config Changes (config.rs)

```rust
// ── RUN180: NVT Ratio Anomaly ──────────────────────────────────────────
// NVT = market_cap / daily_tx_volume
// NVT spike > 1.5x 30-day average → SHORT (overvalued)
// NVT drop < 0.7x 30-day average → LONG (undervalued)

pub const NVT_ENABLED: bool = true;
pub const NVT_SPIKE_MULT: f64 = 1.5;   // NVT must be 1.5x rolling avg
pub const NVT_DROP_MULT: f64 = 0.7;    // NVT must be 0.7x rolling avg
```

---

## Validation Method

1. **Historical backtest** (run180_1_nvt_backtest.py)
2. **Walk-forward** (run180_2_nvt_wf.py)
3. **Combined** (run180_3_combined.py)

## Out-of-Sample Testing

- SPIKE_MULT sweep: 1.25 / 1.5 / 2.0
- DROP_MULT sweep: 0.60 / 0.70 / 0.80
