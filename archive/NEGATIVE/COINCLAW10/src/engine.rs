use crate::config::{self, COINS};
use crate::coordinator::{self, MarketMode, Regime};
use crate::state::{fmt_price, Position, SharedState, TradeRecord, TradeType};
use crate::strategies::{self, Direction};

/// Check exit conditions for coin at index `ci`. Returns true if position was closed.
pub fn check_exit(state: &mut SharedState, ci: usize) -> bool {
    let cs = &state.coins[ci];
    let ind = match &cs.ind_15m {
        Some(i) => i.clone(),
        None => return false,
    };
    let pos = match &cs.pos {
        Some(p) => p.clone(),
        None => return false,
    };

    let is_scalp = pos.trade_type == Some(TradeType::Scalp);
    let price = ind.p;

    if is_scalp {
        // Scalp exit: use latest 1m price if available, else 15m
        let check_price = state.coins[ci]
            .candles_1m
            .last()
            .map(|c| c.c)
            .unwrap_or(price);

        let pnl = if pos.dir == "long" {
            (check_price - pos.e) / pos.e
        } else {
            (pos.e - check_price) / pos.e
        };

        if pnl >= config::SCALP_TP {
            close_position(state, ci, check_price, "TP", TradeType::Scalp);
            return true;
        }
        if pnl <= -config::SCALP_SL {
            close_position(state, ci, check_price, "SL", TradeType::Scalp);
            return true;
        }
        return false;
    }

    // Regime exit
    let cs = &mut state.coins[ci];

    // Track high/low
    if pos.dir == "long" {
        if let Some(ref mut p) = cs.pos {
            if price > p.high { p.high = price; }
        }
    } else if let Some(ref mut p) = cs.pos {
        if price < p.low { p.low = price; }
    }

    // Count candles
    if let Some(ref mut p) = cs.pos {
        if Some(price) != p.last_price.map(|lp| lp) || p.last_price.is_none() {
            cs.candles_held += 1;
            p.last_price = Some(price);
        }
    }

    let pos = cs.pos.as_ref().unwrap();
    let held = cs.candles_held;

    if pos.dir == "long" {
        let pnl = (price - pos.e) / pos.e;
        if pnl <= -config::STOP_LOSS {
            close_position(state, ci, price, "SL", TradeType::Regime);
            return true;
        }
        if pnl > 0.0 && held >= config::MIN_HOLD_CANDLES {
            if price > ind.sma20 {
                close_position(state, ci, price, "SMA", TradeType::Regime);
                return true;
            }
            if ind.z > 0.5 {
                close_position(state, ci, price, "Z0", TradeType::Regime);
                return true;
            }
        }
    } else {
        let pnl = (pos.e - price) / pos.e;
        if pnl <= -config::STOP_LOSS {
            close_position(state, ci, price, "SL", TradeType::Regime);
            return true;
        }
        if pnl > 0.0 && held >= config::MIN_HOLD_CANDLES {
            if price < ind.sma20 {
                close_position(state, ci, price, "SMA", TradeType::Regime);
                return true;
            }
            if ind.z < -0.5 {
                close_position(state, ci, price, "Z0", TradeType::Regime);
                return true;
            }
        }
    }

    false
}

/// Check entry conditions for coin at index `ci`.
pub fn check_entry(
    state: &mut SharedState,
    ci: usize,
    mode: MarketMode,
    ctx: &crate::coordinator::MarketCtx,
) {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return; }
    if cs.cooldown > 0 {
        state.coins[ci].cooldown -= 1;
        return;
    }

    let ind = match &cs.ind_15m {
        Some(i) => i.clone(),
        None => return,
    };

    let cfg = &COINS[cs.config_idx];
    let regime = coordinator::detect_regime(ind.adx, ind.bb_width, ind.bb_width_avg);
    state.coins[ci].regime = regime;

    if regime == Regime::Squeeze {
        state.coins[ci].active_strat = None;
        return;
    }

    match mode {
        MarketMode::Long => {
            state.coins[ci].active_strat = Some(cfg.long_strat.to_string());
            if strategies::long_entry(&ind, cfg.long_strat) {
                open_position(state, ci, ind.p, &regime.to_string(),
                    &cfg.long_strat.to_string(), Direction::Long, TradeType::Regime);
            } else {
                // Try ISO short
                if strategies::iso_short_entry(&ind, cfg.iso_short_strat, ctx) {
                    state.coins[ci].active_strat = Some(cfg.iso_short_strat.to_string());
                    open_position(state, ci, ind.p, &regime.to_string(),
                        &cfg.iso_short_strat.to_string(), Direction::Short, TradeType::Regime);
                }
            }
        }
        MarketMode::IsoShort => {
            state.coins[ci].active_strat = Some(cfg.iso_short_strat.to_string());
            if strategies::iso_short_entry(&ind, cfg.iso_short_strat, ctx) {
                open_position(state, ci, ind.p, &regime.to_string(),
                    &cfg.iso_short_strat.to_string(), Direction::Short, TradeType::Regime);
            }
        }
        MarketMode::Short => {
            state.coins[ci].active_strat = Some(cfg.short_strat.to_string());
            if strategies::short_entry(&ind, cfg.short_strat) {
                open_position(state, ci, ind.p, &regime.to_string(),
                    &cfg.short_strat.to_string(), Direction::Short, TradeType::Regime);
            }
        }
    }
}

/// Check scalp entry for coin at index `ci` (uses 1m indicators).
/// Scalp direction must agree with market mode to avoid shorting into pumps / longing into dumps.
pub fn check_scalp_entry(state: &mut SharedState, ci: usize) {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return; }

    let ind_1m = match &cs.ind_1m {
        Some(i) => i.clone(),
        None => return,
    };

    let price = cs.candles_1m.last().map(|c| c.c).unwrap_or(0.0);
    if price == 0.0 { return; }

    if let Some((dir, strat_name)) = strategies::scalp_entry_with_price(&ind_1m, price) {
        // Enforce: scalp direction must match market mode
        let mode = state.market_mode;
        match (mode, dir) {
            (coordinator::MarketMode::Long, Direction::Short) => return,
            (coordinator::MarketMode::Short, Direction::Long) => return,
            _ => {} // IsoShort allows both; matching directions always allowed
        }
        let regime = cs.regime;
        open_position(state, ci, price, &regime.to_string(), strat_name, dir, TradeType::Scalp);
    }
}

fn open_position(
    state: &mut SharedState,
    ci: usize,
    price: f64,
    regime: &str,
    strat: &str,
    dir: Direction,
    trade_type: TradeType,
) {
    let cs = &mut state.coins[ci];
    if cs.pos.is_some() { return; }

    let risk = match trade_type {
        TradeType::Regime => config::RISK,
        TradeType::Scalp => config::SCALP_RISK,
    };
    let trade_amt = cs.bal * risk;
    let sz = (trade_amt * config::LEVERAGE) / price;
    let dir_str = dir.to_string();

    cs.pos = Some(Position {
        e: price,
        s: sz,
        high: price,
        low: price,
        margin: trade_amt,
        dir: dir_str.clone(),
        last_price: None,
        trade_type: Some(trade_type),
    });
    cs.candles_held = 0;
    cs.active_strat = Some(strat.to_string());

    let vol_r = cs.ind_15m.as_ref().map(|i| {
        if i.vol_ma > 0.0 { i.vol / i.vol_ma } else { 0.0 }
    }).unwrap_or(0.0);

    let ind_info = cs.ind_15m.as_ref().map(|i| {
        format!("RSI:{:.0} Z:{:+.2} ADX:{:.0} Vol:{:.1}x",
            i.rsi, i.z, i.adx, vol_r)
    }).unwrap_or_default();

    let action = if dir == Direction::Long { "BUY" } else { "SHORT" };
    let tt = match trade_type {
        TradeType::Scalp => " [SCALP]",
        TradeType::Regime => "",
    };
    let name = cs.name;
    let msg = format!(
        "{} {}{} [{}>{strat}] @ {} | {} | Cost:${:.2} Bal:${:.2}",
        action, name, tt, regime, fmt_price(price), ind_info, trade_amt, cs.bal
    );
    state.log(msg);
}

fn close_position(
    state: &mut SharedState,
    ci: usize,
    price: f64,
    reason: &str,
    trade_type: TradeType,
) {
    let cs = &mut state.coins[ci];
    let pos = match cs.pos.take() {
        Some(p) => p,
        None => return,
    };

    let cost = pos.s * pos.e;
    let margin = pos.margin;

    let pnl = if pos.dir == "long" {
        pos.s * price - cost
    } else {
        cost - pos.s * price
    };
    let pnl_pct = (pnl / margin) * 100.0;

    cs.bal += pnl;
    cs.trades.push(TradeRecord {
        pnl,
        reason: reason.to_string(),
        dir: pos.dir.clone(),
    });
    cs.cooldown = 2;
    cs.candles_held = 0;

    let action = if pos.dir == "long" { "SELL" } else { "COVER" };
    let tt = match trade_type {
        TradeType::Scalp => " [SCALP]",
        TradeType::Regime => "",
    };
    let name = cs.name;

    let ind_info = cs.ind_15m.as_ref().map(|i| {
        format!("RSI:{:.0} Z:{:+.2} MACD:{:+.4}", i.rsi, i.z, i.macd_hist)
    }).unwrap_or_default();

    let msg = format!(
        "{} {}{} ({}) @ {} | {} | PnL:${:.2}({:+.1}%) Bal:${:.2}",
        action, name, tt, reason, fmt_price(price), ind_info, pnl, pnl_pct, cs.bal
    );
    state.log(msg);
    state.save_state();
}
