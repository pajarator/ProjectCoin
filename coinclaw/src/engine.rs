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

        // RUN35: stoch_50 exit — close while profitable when stochastic exits overbought/oversold zone
        // (Stoch K > 50 for longs, < 50 for shorts, both while pnl > 0)
        if pnl > 0.0 {
            if let Some(ref ind_1m) = state.coins[ci].ind_1m {
                if !ind_1m.stoch_k.is_nan() {
                    if pos.dir == "long" && ind_1m.stoch_k > 50.0 {
                        close_position(state, ci, check_price, "STOCH", TradeType::Scalp);
                        return true;
                    }
                    if pos.dir == "short" && ind_1m.stoch_k < 50.0 {
                        close_position(state, ci, check_price, "STOCH", TradeType::Scalp);
                        return true;
                    }
                }
            }
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
            // RUN82: regime decay exit — ADX has risen significantly above entry
            if config::REGIME_DECAY_ENABLE && held >= config::REGIME_DECAY_GRACE_BARS {
                if let Some(entry_adx) = pos.entry_adx {
                    if ind.adx > entry_adx + config::REGIME_DECAY_ADX_RISE {
                        close_position(state, ci, price, "DECAY", TradeType::Regime);
                        return true;
                    }
                }
            }
            // RUN88: trailing z-score exit — z has recovered Z_RECOVERY_PCT toward 0
            if config::TRAILING_Z_EXIT_ENABLE && held >= config::Z_RECOVERY_MIN_HOLD {
                if let Some(entry_z) = pos.entry_z {
                    if entry_z < 0.0 {
                        let threshold = entry_z + entry_z.abs() * config::Z_RECOVERY_PCT;
                        if ind.z >= threshold {
                            close_position(state, ci, price, "ZREC", TradeType::Regime);
                            return true;
                        }
                    }
                }
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
            // RUN82: regime decay exit — ADX has risen significantly above entry
            if config::REGIME_DECAY_ENABLE && held >= config::REGIME_DECAY_GRACE_BARS {
                if let Some(entry_adx) = pos.entry_adx {
                    if ind.adx > entry_adx + config::REGIME_DECAY_ADX_RISE {
                        close_position(state, ci, price, "DECAY", TradeType::Regime);
                        return true;
                    }
                }
            }
            // RUN88: trailing z-score exit — z has recovered Z_RECOVERY_PCT toward 0
            if config::TRAILING_Z_EXIT_ENABLE && held >= config::Z_RECOVERY_MIN_HOLD {
                if let Some(entry_z) = pos.entry_z {
                    if entry_z > 0.0 {
                        let threshold = entry_z - entry_z.abs() * config::Z_RECOVERY_PCT;
                        if ind.z <= threshold {
                            close_position(state, ci, price, "ZREC", TradeType::Regime);
                            return true;
                        }
                    }
                }
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
    // Extract values under immutable borrow, then release it before mutable ops
    let (has_pos, cooldown, last_z, reentry_count, ind_15m, regime, active_strat) = {
        let cs = &state.coins[ci];
        let ind = cs.ind_15m.clone();
        let regime = cs.regime;
        let active_strat = cs.active_strat.clone();
        (cs.pos.is_some(), cs.cooldown, cs.last_entry_z, cs.reentry_count, ind, regime, active_strat)
    };

    if has_pos { return; }

    // RUN94: cooldown partial reentry
    if cooldown > 0 {
        // Decrement cooldown
        let new_cooldown = cooldown - 1;
        state.coins[ci].cooldown = new_cooldown;

        if new_cooldown == 0 {
            // Cooldown expired without reentry; reset reentry state
            state.coins[ci].reentry_count = 0;
            state.coins[ci].last_entry_z = None;
            return;
        }

        // Still in cooldown window — check partial reentry
        if let Some(entry_z) = last_z {
            if reentry_count < config::MAX_REENTRY_COUNT {
                let ind = match &state.coins[ci].ind_15m {
                    Some(i) => i.clone(),
                    None => return,
                };
                let z_extreme_enough = match mode {
                    MarketMode::Long => {
                        entry_z < 0.0 && ind.z <= entry_z * config::REENTRY_Z_MULT
                    }
                    MarketMode::Short => {
                        entry_z > 0.0 && ind.z >= entry_z * config::REENTRY_Z_MULT
                    }
                    MarketMode::IsoShort => false,
                };
                if z_extreme_enough {
                    let strat = format!("{}[REENTRY]", active_strat.as_deref().unwrap_or("Unknown"));
                    open_position(state, ci, ind.p, &regime.to_string(),
                        &strat, Direction::Long, TradeType::Regime,
                        Some(ind.z), Some(ind.adx), Some(config::REENTRY_SIZE_PCT));
                    state.coins[ci].reentry_count += 1;
                    state.coins[ci].last_entry_z = Some(ind.z);
                }
            }
        }
        return;
    }

    let ind = match ind_15m {
        Some(i) => i,
        None => return,
    };

    let cfg = &COINS[state.coins[ci].config_idx];
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
                    &cfg.long_strat.to_string(), Direction::Long, TradeType::Regime,
                    Some(ind.z), Some(ind.adx), None);
            } else {
                // Try ISO short
                if strategies::iso_short_entry(&ind, cfg.iso_short_strat, ctx) {
                    state.coins[ci].active_strat = Some(cfg.iso_short_strat.to_string());
                    open_position(state, ci, ind.p, &regime.to_string(),
                        &cfg.iso_short_strat.to_string(), Direction::Short, TradeType::Regime,
                        Some(ind.z), Some(ind.adx), None);
                }
            }
        }
        MarketMode::IsoShort => {
            state.coins[ci].active_strat = Some(cfg.iso_short_strat.to_string());
            if strategies::iso_short_entry(&ind, cfg.iso_short_strat, ctx) {
                open_position(state, ci, ind.p, &regime.to_string(),
                    &cfg.iso_short_strat.to_string(), Direction::Short, TradeType::Regime,
                    Some(ind.z), Some(ind.adx), None);
            }
        }
        MarketMode::Short => {
            state.coins[ci].active_strat = Some(cfg.short_strat.to_string());
            if strategies::short_entry(&ind, cfg.short_strat) {
                open_position(state, ci, ind.p, &regime.to_string(),
                    &cfg.short_strat.to_string(), Direction::Short, TradeType::Regime,
                    Some(ind.z), Some(ind.adx), None);
            }
        }
    }
}

/// Check scalp entry for coin at index `ci` (uses 1m indicators).
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
        let regime = cs.regime;
        open_position(state, ci, price, &regime.to_string(), strat_name, dir, TradeType::Scalp, None, None, None);
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
    entry_z: Option<f64>,
    entry_adx: Option<f64>,
    size_mult: Option<f64>,
) {
    let cs = &mut state.coins[ci];
    if cs.pos.is_some() { return; }

    let risk = match trade_type {
        TradeType::Regime => config::RISK,
        TradeType::Scalp => config::SCALP_RISK,
    };
    let size_multiplier = size_mult.unwrap_or(1.0);
    let trade_amt = cs.bal * risk * size_multiplier;
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
        entry_z,
        entry_adx,
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
        trade_type: Some(trade_type),
    });
    cs.cooldown = 2;
    cs.candles_held = 0;
    // RUN75: record PnL percentage for regime trades for Sharpe computation
    if trade_type == TradeType::Regime {
        cs.trade_pnls_pct.push_back(pnl_pct);
        if cs.trade_pnls_pct.len() > config::SHARPE_WINDOW * 2 {
            cs.trade_pnls_pct.pop_front();
        }
    }
    // RUN94: save entry_z for partial reentry threshold, reset reentry count
    if trade_type == TradeType::Regime {
        cs.last_entry_z = pos.entry_z;
        cs.reentry_count = 0;
    }

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
