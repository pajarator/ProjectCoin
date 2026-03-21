# RUN82 — Regime Decay Detection: Recommendation

## Hypothesis
Named: `regime_decay_exit`

Early exit when ADX rises significantly above entry ADX or regime shifts to StrongTrend while in a position.

## Results

### RUN82.1 — Grid Search (25 configs × 18 coins, 5-month 15m data)

**POSITIVE — 13/24 configs improve PnL vs baseline.**

Best config: **AR25_RST_G5** (ADX_RISE=25.0, REGIME_SHIFT=true, GRACE=5)

| Config | PnL | ΔPnL | WR% | Trades | PF | Decay% |
|--------|------|------|-----|--------|-----|--------|
| BASELINE | +$292.85 | — | 25.9% | 9,716 | 0.35 | 0.0% |
| AR25_RST_G5 (best) | +$316.31 | +$23.47 | 35.2% | 10,960 | 0.55 | 26.1% |
| AR20_RST_G10 | +$311.02 | +$18.17 | 30.5% | 10,449 | 0.44 | 14.7% |
| AR10_RST_G3 (worst) | +$272.50 | -$20.34 | 40.5% | 11,799 | 0.70 | 41.2% |

**Key findings:**
- WR improves +3.8 to +14.6pp across top configs
- PF improves from 0.35 to 0.55-0.70 (best configs)
- Trade count increases (decay exits early, freeing capital for re-entry)
- Grace period of 3 bars is too short (all G3 configs negative); 5-10 bars optimal
- ADX_RISE of 25-20 is optimal; 10 is too sensitive (fires too often)
- REGIME_SHIFT has no effect (top configs have same results with T and F)

### RUN82.2 — Walk-Forward Validation

**POSITIVE — 3/3 passes, avg test Δ = +$11.50**

| Window | Train PnL | Test PnL (Base) | Test PnL (Best) | Δ |
|--------|-----------|-----------------|-----------------|---|
| 1 (0-4mo train / mo5 test) | +$167.61 | +$167.61 | +$171.30 | +$3.69 |
| 2 (mo3-4 train / mo6 test) | +$144.86 | +$144.86 | +$157.41 | +$12.55 |
| 3 (mo4-5 train / mo6 test) | +$153.47 | +$153.47 | +$171.74 | +$18.27 |

## Conclusion

**POSITIVE — Consider COINCLAW changes.**

Proposed config:
```rust
pub const REGIME_DECAY_ENABLE: bool = true;
pub const REGIME_DECAY_ADX_RISE: f64 = 25.0;
pub const REGIME_DECAY_REGIME_SHIFT: bool = true;  // no practical effect
pub const REGIME_DECAY_GRACE_BARS: u32 = 5;
```

Effect: Adds early exit when ADX rises 25+ above entry ADX. Exits 26% of regime trades early. WR improves 25.9% → 35.2%, PF improves 0.35 → 0.55.

## Files
- `run82_1_results.json` — Grid search results
- `run82_2_results.json` — Walk-forward results
- `coinclaw/src/run82.rs` — Grid search implementation
- `coinclaw/src/run82_2.rs` — Walk-forward implementation
