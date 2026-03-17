use crate::config::{self, COINS};
use crate::coordinator::{self, MarketMode, Regime};
use crate::state::{fmt_price, Position, SharedState, TradeRecord, TradeType};
use crate::strategies::{self, Direction};

// ── Momentum breakout helpers (RUN27/28) ──────────────────────────────────────

/// Returns true if the 15m indicators satisfy LONG breakout entry conditions.
fn momentum_long_signal(ind: &crate::indicators::Ind15m, mb: &config::MomentumBreakout) -> bool {
    if ind.atr14_price <= 0.0 || ind.sma50 <= 0.0 { return false; }
    ind.ret16 >= mb.move_thresh
        && ind.vol_ma > 0.0 && ind.vol >= ind.vol_ma * mb.vol_mult
        && ind.adx >= mb.adx_thresh && ind.adx > ind.adx_prev3
        && ind.p > ind.sma50
        && ind.rsi >= 50.0 && ind.rsi <= 75.0
}

pub fn check_momentum_entry(state: &mut SharedState, ci: usize) {
    if state.coins[ci].pos.is_some() { return; }
    if state.coins[ci].cooldown > 0 { return; }

    let cfg = &COINS[state.coins[ci].config_idx];
    let mb = match &cfg.momentum {
        Some(m) => m.clone(),
        None => return,
    };

    let ind = match &state.coins[ci].ind_15m {
        Some(i) => i.clone(),
        None => return,
    };
    if !ind.valid { return; }

    if momentum_long_signal(&ind, &mb) {
        let atr_stop      = ind.p - ind.atr14_price * mb.atr_mult;
        let trail_distance = ind.atr14_price * mb.trail_atr;
        let trail_act_price = ind.p * (1.0 + mb.trail_act);
        open_momentum_position(state, ci, ind.p, atr_stop, trail_distance, trail_act_price);
    }
}

fn open_momentum_position(
    state: &mut SharedState,
    ci: usize,
    price: f64,
    atr_stop: f64,
    trail_distance: f64,
    trail_act_price: f64,
) {
    let cs = &mut state.coins[ci];
    if cs.pos.is_some() { return; }

    let trade_amt = cs.bal * config::RISK;
    let sz = (trade_amt * config::LEVERAGE) / price;

    cs.pos = Some(Position {
        e: price,
        s: sz,
        high: price,
        low: price,
        margin: trade_amt,
        dir: "long".to_string(),
        last_price: None,
        trade_type: Some(TradeType::Momentum),
        atr_stop: Some(atr_stop),
        trail_distance: Some(trail_distance),
        trail_act_price: Some(trail_act_price),
    });
    cs.candles_held = 0;
    cs.active_strat = Some("momentum".to_string());

    let vol_r = if cs.ind_15m.as_ref().map(|i| i.vol_ma).unwrap_or(0.0) > 0.0 {
        cs.ind_15m.as_ref().map(|i| i.vol / i.vol_ma).unwrap_or(0.0)
    } else { 0.0 };

    let ind_info = cs.ind_15m.as_ref().map(|i| {
        format!("Ret16:{:+.1}% RSI:{:.0} ADX:{:.0} Vol:{:.1}x ATR_SL:{:.4}",
            i.ret16 * 100.0, i.rsi, i.adx, vol_r, atr_stop)
    }).unwrap_or_default();

    let name = cs.name;
    let msg = format!(
        "BUY {} [MOMENTUM] @ {} | {} | Cost:${:.2} Bal:${:.2}",
        name, fmt_price(price), ind_info, trade_amt, cs.bal
    );
    state.log(msg);
}

/// Check exit for an open momentum position. Returns true if closed.
fn check_momentum_exit(state: &mut SharedState, ci: usize) -> bool {
    let ind = match &state.coins[ci].ind_15m {
        Some(i) => i.clone(),
        None => return false,
    };
    let pos = match &state.coins[ci].pos {
        Some(p) if p.trade_type == Some(TradeType::Momentum) => p.clone(),
        _ => return false,
    };

    let price = ind.p;
    let atr_stop     = pos.atr_stop.unwrap_or(pos.e * (1.0 - 0.003));
    let trail_dist   = pos.trail_distance.unwrap_or(0.0);
    let trail_act    = pos.trail_act_price.unwrap_or(f64::MAX);
    let peak         = pos.high;

    // Update peak
    if price > peak {
        if let Some(ref mut p) = state.coins[ci].pos { p.high = price; }
    }
    let peak = state.coins[ci].pos.as_ref().map(|p| p.high).unwrap_or(price);

    // Count candles held
    if let Some(ref mut p) = state.coins[ci].pos {
        if p.last_price != Some(price) {
            state.coins[ci].candles_held += 1;
            if let Some(ref mut p2) = state.coins[ci].pos { p2.last_price = Some(price); }
        }
    }
    let held = state.coins[ci].candles_held;

    let cfg = &COINS[state.coins[ci].config_idx];
    let rsi_exit = cfg.momentum.as_ref().map(|m| m.rsi_exit).unwrap_or(78.0);

    // 1. ATR hard stop
    if price <= atr_stop {
        close_position(state, ci, price, "ATR_SL", TradeType::Momentum);
        return true;
    }

    // 2. Trailing stop (once peak ≥ trail_act_price)
    if trail_dist > 0.0 && peak >= trail_act {
        let trail_stop = peak - trail_dist;
        if price <= trail_stop {
            close_position(state, ci, price, "TRAIL", TradeType::Momentum);
            return true;
        }
    }

    // 3. RSI overbought exhaustion
    if ind.rsi > rsi_exit {
        close_position(state, ci, price, "RSI_OB", TradeType::Momentum);
        return true;
    }

    // 4. Signal exit: price falls back below SMA20 while profitable (after warmup)
    if held >= config::MIN_HOLD_CANDLES {
        let pnl = (price - pos.e) / pos.e;
        if pnl > 0.0 && price < ind.sma20 {
            close_position(state, ci, price, "SMA20", TradeType::Momentum);
            return true;
        }
    }

    false
}

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

    let trade_type = pos.trade_type;
    let price = ind.p;

    // Momentum exit runs independently with its own ATR stop + trail logic
    if trade_type == Some(TradeType::Momentum) {
        return check_momentum_exit(state, ci);
    }

    let is_scalp = trade_type == Some(TradeType::Scalp);

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
            } else if strategies::complement_entry(&ind, cfg.complement_strat, cfg.complement_z_filter) {
                // RUN13: complementary long entry — fires at different times than primary
                state.coins[ci].active_strat = Some(cfg.complement_strat.to_string());
                open_position(state, ci, ind.p, &regime.to_string(),
                    &cfg.complement_strat.to_string(), Direction::Long, TradeType::Regime);
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

    // RUN27/28: Momentum breakout layer — fires independently of regime mode.
    // Only activates if the coin has a MomentumBreakout config and no position was
    // just opened by the regime logic above.
    if state.coins[ci].pos.is_none() {
        check_momentum_entry(state, ci);
    }
}

/// Check scalp entry for coin at index `ci` (uses 1m indicators).
/// Scalp direction must agree with market mode to avoid shorting into pumps / longing into dumps.
pub fn check_scalp_entry(state: &mut SharedState, ci: usize) {
    // Fix #2: block scalps in Squeeze regime
    if state.coins[ci].regime == Regime::Squeeze { return; }
    if state.coins[ci].pos.is_some() { return; }

    // Fix #1: respect time-based scalp cooldown
    if let Some(until) = state.coins[ci].scalp_cooldown_until {
        if std::time::Instant::now() < until { return; }
        state.coins[ci].scalp_cooldown_until = None;
    }

    let ind_1m = match &state.coins[ci].ind_1m {
        Some(i) => i.clone(),
        None => return,
    };

    let price = state.coins[ci].candles_1m.last().map(|c| c.c).unwrap_or(0.0);
    if price == 0.0 { return; }

    // Fix #3: only use scalp_stoch_cross (vol_spike_rev removed — 5.9% WR is noise)
    if let Some((dir, strat_name)) = strategies::scalp_entry_stoch_only(&ind_1m) {
        // Enforce: scalp direction must match market mode
        let mode = state.market_mode;
        match (mode, dir) {
            (coordinator::MarketMode::Long, Direction::Short) => return,
            (coordinator::MarketMode::Short, Direction::Long) => return,
            _ => {} // IsoShort allows both; matching directions always allowed
        }
        let regime = state.coins[ci].regime;
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
        TradeType::Regime | TradeType::Momentum => config::RISK,
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
        atr_stop: None,
        trail_distance: None,
        trail_act_price: None,
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
        TradeType::Momentum => " [MOMENTUM]",
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
    cs.candles_held = 0;

    // Fix #1: time-based scalp cooldown after SL
    if trade_type == TradeType::Scalp && reason == "SL" {
        cs.scalp_cooldown_until = Some(
            std::time::Instant::now() + std::time::Duration::from_secs(config::SCALP_COOLDOWN_SECS)
        );
    }

    // Fix #4: escalating cooldown for consecutive SL on regime trades
    if reason == "SL" {
        cs.consecutive_sl += 1;
        if trade_type == TradeType::Regime && cs.consecutive_sl >= 2 {
            cs.cooldown = config::ISO_SL_ESCALATE_COOLDOWN;
        } else {
            cs.cooldown = 2;
        }
    } else {
        cs.consecutive_sl = 0;
        cs.cooldown = 2;
    }

    let action = if pos.dir == "long" { "SELL" } else { "COVER" };
    let tt = match trade_type {
        TradeType::Scalp => " [SCALP]",
        TradeType::Momentum => " [MOMENTUM]",
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
