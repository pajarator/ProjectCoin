# RUN12 — Scalp Market Mode Filter

## Goal
Fix scalps shorting into market-wide pumps (and longing into dumps) by enforcing that scalp trade direction must agree with the market mode.

## Problem
On 2026-03-15, COINCLAW v11 lost ~$5 across 18 coins in a 10-minute window (21:20–21:40) during a broad market pump. The scalp overlay (vol_spike_rev, stoch_cross) triggered SHORT entries on nearly every coin as RSI/stochastic went overbought on 1m, but the pump continued and all positions hit SL=0.1% within seconds. With no directional filter, the system re-entered SHORT immediately after each stop-out, creating a cascade of ~60 losing trades.

### Root Cause
`check_scalp_entry()` in `engine.rs` never checked `state.market_mode`. Scalps operated on 1m indicators only, completely independent of the regime system that governs 15m trades. So:

- Market mode = **LONG** (breadth ≤ 20%, almost no coins bearish — everything pumping)
- Regime trades correctly blocked shorts in LONG mode
- Scalps ignored this and shorted every coin because 1m RSI > 80 / stoch overbought
- SL=0.1% hit in seconds, re-enter, hit again — death spiral

### Evidence from trading_log.txt
```
[21:30:58] SHORT LINK [SCALP] [SQUEEZE>scalp_vol_spike_rev] @ $9.68  Z:+2.81
[21:30:58] SHORT ETH  [SCALP] [SQUEEZE>scalp_vol_spike_rev] @ $2225  Z:+2.66
[21:30:58] SHORT DOT  [SCALP] [WTREND>scalp_vol_spike_rev]  @ $1.47  Z:+2.17
[21:30:58] SHORT XRP  [SCALP] [SQUEEZE>scalp_vol_spike_rev] @ $1.46  Z:+1.52
[21:30:58] SHORT SOL  [SCALP] [SQUEEZE>scalp_vol_spike_rev] @ $92.89 Z:+0.44
... (all stopped out within 15-30 seconds, immediately re-entered)
```

90 round-trip trades in 3 hours. Win rate: 4.5% (4 TP wins out of ~88 trades).

## Fix
Single check added to `check_scalp_entry()` in `engine.rs`:

```rust
match (mode, dir) {
    (MarketMode::Long, Direction::Short) => return,   // no shorts during pump
    (MarketMode::Short, Direction::Long) => return,    // no longs during dump
    _ => {} // IsoShort allows both; matching directions always allowed
}
```

- **LONG mode**: scalps can only go long (buy oversold dips during a pump)
- **SHORT mode**: scalps can only go short (sell overbought bounces during a dump)
- **ISO_SHORT mode**: both directions allowed (individual coins can diverge from market)

## Impact
Every losing trade in the 21:20–21:40 cascade was a short scalp during LONG mode. This filter would have blocked all ~60 of those entries, preventing the entire loss.

## Conclusion
The regime system was already correctly identifying the market state — it just wasn't being consulted by the scalp subsystem. This was a supervision gap, not a strategy failure. The fix is minimal (4 lines) and requires no parameter tuning.
