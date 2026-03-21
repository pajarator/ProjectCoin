# RUN110 — BB Width Compression Entry: Enter When Bollinger Band Width Has Been Compressed

## Hypothesis

**Named:** `bb_compression_entry`

**Mechanism:** COINCLAW's regime trades enter when z-score is extreme. But extreme z-score in a low-volatility (narrow BB) environment means the deviation happened with very little price movement — the potential energy stored in the compression will be released as the volatility expands. The BB Width Compression Entry adds a pre-entry filter: when BB width has been below `BB_COMPRESS_THRESHOLD * bb_width_avg` for `BB_COMPRESS_BARS` consecutive bars, the next regime LONG or SHORT entry has higher probability because volatility expansion is imminent.

**BB Width Compression Entry:**
- Track `bb_compression_count` per coin — consecutive bars with `bb_width < bb_width_avg * BB_COMPRESS_THRESHOLD`
- When `bb_compression_count >= BB_COMPRESS_BARS`:
  - Next regime entry (LONG or SHORT) gets a boost: entries are allowed with less extreme z-score
  - e.g., LONG normally requires z < -1.5, but after compression: z < -1.3
  - The compression itself acts as a "coil" — price will spring
- Require minimum hold after compression entry: BB_COMPRESS_MIN_HOLD bars

**Why this is not a duplicate:**
- RUN65 (BB squeeze duration) used BB squeeze as a regime filter for entries — this uses BB compression as an entry BOOTSTRAP, allowing less extreme entries after compression
- RUN72 (choppy mode) suppressed scalp during low ATR periods — this uses BB compression specifically, not ATR
- No prior RUN has used the COMPRESSION of BB width as a pre-condition that modifies entry thresholds

**Mechanistic rationale:** Bollinger Bands contract before they expand. A squeeze (RUN65) is an extreme case of compression. But even milder compression (BB width < 0.7× average for 4+ bars) indicates a coiled market. Entries fired after compression tend to have higher win rates because the subsequent volatility expansion moves price further in the mean-reversion direction. The compression is the preparation; the expansion is the payoff.

---

## Proposed Config Changes

```rust
// RUN110: BB Width Compression Entry
pub const BB_COMPRESS_ENABLE: bool = true;
pub const BB_COMPRESS_THRESHOLD: f64 = 0.70;  // bb_width < bb_width_avg * 0.70 = compressed
pub const BB_COMPRESS_BARS: u32 = 4;          // consecutive compressed bars required
pub const BB_COMPRESS_Z_RELAX: f64 = 0.20;   // relax z threshold by this much after compression (e.g., -1.5 → -1.3)
pub const BB_COMPRESS_MIN_HOLD: u32 = 3;       // minimum bars to hold after compression entry
```

**`state.rs` — CoinState additions:**
```rust
pub struct CoinState {
    // ... existing fields ...
    pub bb_compression_count: u32,   // consecutive bars of compressed BB width
}
```

**`strategies.rs` — modify long_entry and short_entry:**
```rust
/// Update BB compression counter each bar.
fn update_bb_compression(cs: &mut CoinState) {
    if let Some(ref ind) = cs.ind_15m {
        if !ind.bb_width_avg.is_nan() && ind.bb_width_avg > 0.0 {
            let ratio = ind.bb_width / ind.bb_width_avg;
            if ratio < config::BB_COMPRESS_THRESHOLD {
                cs.bb_compression_count += 1;
            } else {
                cs.bb_compression_count = 0;
            }
        }
    }
}

/// Check if BB compression conditions are met for relaxed entry.
fn bb_compression_active(cs: &CoinState) -> bool {
    if !config::BB_COMPRESS_ENABLE { return false; }
    cs.bb_compression_count >= config::BB_COMPRESS_BARS
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat, cs: &CoinState) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }

    // Base entry z threshold
    let base_thresh = match strat {
        LongStrat::VwapReversion => -1.5,
        LongStrat::BbBounce => -1.5,  // price near lower BB, not z-based
        LongStrat::AdrReversal => -1.5,
        LongStrat::DualRsi => -1.5,
        LongStrat::MeanReversion => -1.5,
        LongStrat::OuMeanRev => -1.5,
    };

    // Apply compression relaxation if active
    let effective_thresh = if bb_compression_active(cs) {
        base_thresh + config::BB_COMPRESS_Z_RELAX  // less extreme z required after compression
    } else {
        base_thresh
    };

    if ind.z >= effective_thresh { return false; }

    match strat {
        LongStrat::VwapReversion => {
            ind.z < -1.5 && ind.p < ind.vwap && ind.vol > ind.vol_ma * 1.2
        }
        LongStrat::BbBounce => {
            ind.p <= ind.bb_lo * 1.02 && ind.vol > ind.vol_ma * 1.3
        }
        LongStrat::AdrReversal => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_lo.is_nan() && range > 0.0
                && ind.p <= ind.adr_lo + range * 0.25
                && ind.vol > ind.vol_ma * 1.1
        }
        LongStrat::DualRsi => {
            ind.rsi < 40.0 && ind.rsi7 < 30.0 && ind.sma9 > ind.sma20
        }
        LongStrat::MeanReversion => ind.z < -1.5,
        LongStrat::OuMeanRev => {
            !ind.ou_halflife.is_nan() && !ind.ou_deviation.is_nan()
                && ind.std20 > 0.0
                && ind.ou_halflife >= config::OU_MIN_HALFLIFE
                && ind.ou_halflife <= config::OU_MAX_HALFLIFE
                && (ind.ou_deviation / ind.std20) < -config::OU_DEV_THRESHOLD
        }
    }
}
```

---

## Validation Method

### RUN110.1 — BB Compression Grid Search (Rust, parallel across 18 coins)

**Data:** 15m OHLCV for all 18 coins, 1-year dataset

**Baseline:** Current COINCLAW v16 — fixed z-thresholds, no compression filter

**Grid search:**

| Parameter | Values |
|-----------|--------|
| `BB_COMPRESS_THRESHOLD` | [0.60, 0.70, 0.80] |
| `BB_COMPRESS_BARS` | [3, 4, 5] |
| `BB_COMPRESS_Z_RELAX` | [0.10, 0.20, 0.30] |

**Per coin:** 3 × 3 × 3 = 27 configs × 18 coins = 486 backtests

**Key metrics:**
- `compression_entry_rate`: % of entries made after compression (relaxed threshold)
- `compression_win_rate`: win rate of compression entries vs non-compression entries
- `PF_delta`: profit factor change vs baseline
- `WR_delta`: win rate change vs baseline
- `avg_z_delta`: change in average |z| at entry (should decrease for compression entries)

### RUN110.2 — Walk-Forward Validation

**Method:** 3-window walk-forward (train 2mo, test 1mo)

For each window:
1. Train: find best COMPRESS_THRESHOLD × BARS × RELAX per coin
2. Test: evaluate on held-out month

**Pass criteria:**
- ≥ 10/18 coins show positive OOS P&L delta vs baseline
- Compression entries have higher win rate than non-compression entries

### RUN110.3 — Combined Comparison

Side-by-side:

| Metric | Baseline (v16, fixed thresholds) | BB Compression Entry | Delta |
|--------|-------------------------------|-------------------|-------|
| Total P&L | $X | $X | +$X |
| Win Rate | X% | X% | +Ypp |
| Profit Factor | X.XX | X.XX | +0.XX |
| Max Drawdown | X% | X% | -Ypp |
| Compression Entries | 0% | X% | — |
| Non-Compression Entries | X% | X% | -Y% |
| Compression Entry WR% | — | X% | — |
| Avg Z at Compression Entry | — | X | — |

---

## Why This Could Fail

1. **Compression doesn't guarantee expansion:** BB width can stay compressed for many bars before expanding. Entering after 4 compressed bars expecting an imminent expansion may not materialize.
2. **Relaxing z-threshold weakens entries:** Allowing less extreme z-scores (z = -1.3 instead of -1.5) after compression means weaker entry signals. The compression itself is not a signal — it just enables weaker entries.

---

## Why It Could Succeed

1. **BB expansion follows compression:** This is a well-established market phenomenon. Tight ranges precede big moves. Entering after compression catches the start of the move.
2. **Mean reversion works after compression:** After a compressed period, price is more likely to make a larger move in the mean-reversion direction. The relaxed z-threshold captures this.
3. **Adaptable to volatility regimes:** Compression is more meaningful in high-volatility regimes. The system naturally adapts — compression requires a baseline of normal volatility to be meaningful.

---

## Comparison to Baseline

| | Current COINCLAW v16 | RUN110 BB Compression Entry |
|--|--|--|
| Entry threshold | Fixed z = -1.5 | Fixed z = -1.5, relaxed to -1.3 after compression |
| BB width awareness | None | Compressed BB width relaxes threshold |
| Compression filter | None | 4 bars below 0.70× avg BB width |
| Entry quality | Always extreme | Mixed: extreme normally, less extreme after compression |
| Volatility adaptation | None | Natural (compression → expansion) |
