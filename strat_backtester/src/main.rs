//! High-Performance Multi-Strategy Backtester
//! Uses Rayon for parallel processing across multiple CPUs

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

// =======================
// Data Structures
// =======================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub strategy: String,
    pub coin: String,
    pub initial_capital: f64,
    pub final_capital: f64,
    pub total_trades: i32,
    pub winning_trades: i32,
    pub losing_trades: i32,
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    pub max_drawdown: f64,
    pub pnl_pct: f64,
}

#[derive(Debug, Clone)]
pub struct StrategyParams {
    pub stop_loss: f64,
    pub min_hold_candles: i32,
    pub risk: f64,
    pub leverage: f64,
    pub rsi_period: i32,
    pub rsi_overbought: f64,
    pub rsi_oversold: f64,
    pub bb_period: i32,
    pub bb_std: f64,
    pub ma_period: i32,
    pub z_score_threshold: f64,
    pub volume_multiplier: f64,
}

// =======================
// Indicators
// =======================

fn calculate_sma(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    prices
        .windows(period)
        .map(|w| Some(w.iter().sum::<f64>() / period as f64))
        .chain(std::iter::repeat(None))
        .collect()
}

fn calculate_ema(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut ema = Vec::with_capacity(prices.len());
    let multiplier = 2.0 / (period as f64 + 1.0);
    
    for (i, &price) in prices.iter().enumerate() {
        if i < period - 1 {
            ema.push(None);
        } else if i == period - 1 {
            let sma = prices[i + 1 - period..=i].iter().sum::<f64>() / period as f64;
            ema.push(Some(sma));
        } else {
            if let Some(prev) = ema[i - 1] {
                ema.push(Some((price - prev) * multiplier + prev));
            } else {
                ema.push(None);
            }
        }
    }
    ema
}

fn calculate_stddev(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    prices
        .windows(period)
        .map(|w| {
            let mean = w.iter().sum::<f64>() / period as f64;
            let variance = w.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / period as f64;
            Some(variance.sqrt())
        })
        .chain(std::iter::repeat(None))
        .collect()
}

fn calculate_rsi(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut rsi = Vec::with_capacity(prices.len());
    let mut gains = Vec::with_capacity(prices.len());
    let mut losses = Vec::with_capacity(prices.len());
    
    for i in 0..prices.len() {
        if i == 0 {
            gains.push(0.0);
            losses.push(0.0);
            rsi.push(None);
        } else {
            let change = prices[i] - prices[i - 1];
            let gain = if change > 0.0 { change } else { 0.0 };
            let loss = if change < 0.0 { -change } else { 0.0 };
            
            let avg_gain = if i < period {
                gains.iter().sum::<f64>() / i as f64
            } else {
                (gains[i - period..].iter().sum::<f64>()) / period as f64
            };
            
            let avg_loss = if i < period {
                losses.iter().sum::<f64>() / i as f64
            } else {
                (losses[i - period..].iter().sum::<f64>()) / period as f64
            };
            
            gains.push(gain);
            losses.push(loss);
            
            if avg_loss == 0.0 {
                rsi.push(Some(100.0));
            } else {
                let rs = avg_gain / avg_loss;
                rsi.push(Some(100.0 - (100.0 / (1.0 + rs))));
            }
        }
    }
    rsi
}

fn calculate_macd(prices: &[f64]) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let ema12 = calculate_ema(prices, 12);
    let ema26 = calculate_ema(prices, 26);
    let macd: Vec<Option<f64>> = ema12.iter()
        .zip(ema26.iter())
        .map(|(e12, e26)| {
            match (e12, e26) {
                (Some(v12), Some(v26)) => Some(v12 - v26),
                _ => None
            }
        })
        .collect();
    let valid_macd: Vec<f64> = macd.iter().flatten().cloned().collect();
    let signal = calculate_ema(&valid_macd, 9);
    let mut signal_full = vec![None; prices.len()];
    let mut sig_idx = 0;
    for (i, m) in macd.iter().enumerate() {
        if m.is_some() {
            if sig_idx < signal.len() {
                signal_full[i] = signal[sig_idx];
                sig_idx += 1;
            }
        }
    }
    let histogram: Vec<Option<f64>> = macd.iter()
        .zip(signal_full.iter())
        .map(|(m, s)| match (m, s) {
            (Some(mv), Some(sv)) => Some(mv - sv),
            _ => None
        })
        .collect();
    (macd, signal_full, histogram)
}

fn calculate_atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut atr = Vec::with_capacity(close.len());
    
    for i in 0..close.len() {
        if i == 0 {
            atr.push(None);
            continue;
        }
        
        let tr = high[i] - low[i];
        let tr1 = (high[i] - close[i - 1]).abs();
        let tr2 = (low[i] - close[i - 1]).abs();
        let true_range = tr.max(tr1).max(tr2);
        
        if i < period {
            atr.push(None);
        } else {
            let prev_atr = atr[i - 1].unwrap_or(true_range);
            Some((prev_atr * (period - 1) as f64 + true_range) / period as f64)
        }
    }
    atr
}

fn calculate_adx(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<Option<f64>> {
    let atr = calculate_atr(high, low, close, period);
    let mut adx = Vec::with_capacity(close.len());
    
    for i in 0..close.len() {
        if i < period * 2 {
            adx.push(None);
        } else {
            if let Some(a) = atr[i] {
                let avg_price = close[i-period..i].iter().sum::<f64>() / period as f64;
                adx.push(Some((a / avg_price) * 100.0));
            } else {
                adx.push(None);
            }
        }
    }
    adx
}

// =======================
// Strategy Signals
// =======================

fn mean_reversion_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 20 { return false; }
    
    if let Some(z) = indicators.get("z_score").and_then(|v| v.get(idx)).flatten() {
        let vol_mult = params.volume_multiplier;
        if let Some(vol_ratio) = indicators.get("volume_ratio").and_then(|v| v.get(idx)).flatten() {
            return *z < -params.z_score_threshold && *vol_ratio > vol_mult;
        }
    }
    false
}

fn rsi_reversal_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < params.rsi_period as usize { return false; }
    
    if let Some(rsi) = indicators.get("rsi").and_then(|v| v.get(idx)).flatten() {
        let vol_mult = params.volume_multiplier;
        if let Some(vol_ratio) = indicators.get("volume_ratio").and_then(|v| v.get(idx)).flatten() {
            return *rsi < params.rsi_oversold && *vol_ratio > vol_mult;
        }
    }
    false
}

fn macd_cross_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 30 { return false; }
    
    if let (Some(macd), Some(signal)) = (
        indicators.get("macd").and_then(|v| v.get(idx)).flatten(),
        indicators.get("macd_signal").and_then(|v| v.get(idx)).flatten()
    ) {
        if let (Some(macd_prev), Some(signal_prev)) = (
            indicators.get("macd").and_then(|v| v.get(idx - 1)).flatten(),
            indicators.get("macd_signal").and_then(|v| v.get(idx - 1)).flatten()
        ) {
            return *macd > *signal && *macd_prev <= *signal_prev;
        }
    }
    false
}

fn bb_bounce_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < params.bb_period as usize { return false; }
    
    let price = candles[idx].close;
    
    if let (Some(bb_lower), Some(vol_ratio)) = (
        indicators.get("bb_lower").and_then(|v| v.get(idx)).flatten(),
        indicators.get("volume_ratio").and_then(|v| v.get(idx)).flatten()
    ) {
        return price <= *bb_lower * 1.02 && *vol_ratio > params.volume_multiplier;
    }
    false
}

fn trend_follow_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < params.ma_period as usize + 5 { return false; }
    
    let price = candles[idx].close;
    
    if let (Some(ma), Some(adx)) = (
        indicators.get("sma").and_then(|v| v.get(idx)).flatten(),
        indicators.get("adx").and_then(|v| v.get(idx)).flatten()
    ) {
        let vol_mult = params.volume_multiplier;
        if let Some(vol_ratio) = indicators.get("volume_ratio").and_then(|v| v.get(idx)).flatten() {
            return price > *ma && *adx > 25.0 && *vol_ratio > vol_mult;
        }
    }
    false
}

fn adr_reversal_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 24 { return false; }
    
    let price = candles[idx].close;
    let low_24 = candles[idx.saturating_sub(24)..=idx].iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    let high_24 = candles[idx.saturating_sub(24)..=idx].iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
    let range = high_24 - low_24;
    
    return price <= low_24 + range * 0.25;
}

fn stochastic_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 14 { return false; }
    
    if let Some(stoch) = indicators.get("stochastic").and_then(|v| v.get(idx)).flatten() {
        return *stoch < 20.0;
    }
    false
}

fn volume_spike_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 20 { return false; }
    
    if let Some(vol_ratio) = indicators.get("volume_ratio").and_then(|v| v.get(idx)).flatten() {
        return *vol_ratio > 2.0;
    }
    false
}

fn adx_breakout_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 30 { return false; }
    
    if let (Some(adx), Some(vol_ratio)) = (
        indicators.get("adx").and_then(|v| v.get(idx)).flatten(),
        indicators.get("volume_ratio").and_then(|v| v.get(idx)).flatten()
    ) {
        if let Some(adx_prev) = indicators.get("adx").and_then(|v| v.get(idx - 1)).flatten() {
            return *adx > 30.0 && *adx > *adx_prev && *vol_ratio > params.volume_multiplier;
        }
    }
    false
}

fn gap_fill_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 1 { return false; }
    
    let prev_close = candles[idx - 1].close;
    let open = candles[idx].open;
    let gap = (open - prev_close) / prev_close;
    
    return gap < -0.01;
}

fn pivot_reversal_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 5 { return false; }
    
    let lows: Vec<f64> = candles[idx.saturating_sub(5)..=idx].iter().map(|c| c.low).collect();
    let pivot_low = lows.iter().cloned().fold(f64::INFINITY, f64::min);
    let price = candles[idx].close;
    
    return price < pivot_low * 1.01;
}

fn engulfs_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 2 { return false; }
    
    let curr = &candles[idx];
    let prev = &candles[idx - 1];
    
    let prevBearish = prev.close < prev.open;
    let currBullish = curr.close > curr.open;
    let bodyEngulfs = curr.close > prev.open && curr.open < prev.close;
    
    return prevBearish && currBullish && bodyEngulfs;
}

fn pin_bar_entry(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>) -> bool {
    if idx < 2 { return false; }
    
    let candle = &candles[idx];
    let body = (candle.close - candle.open).abs();
    let upper_wick = candle.high - candle.close.max(candle.open);
    let lower_wick = candle.open.min(candle.close) - candle.low;
    
    return lower_wick > body * 2.0 && lower_wick > (candle.high - candle.low) * 0.5;
}

// =======================
// Exit Signals
// =

fn exit_signal(candles: &[Candle], params: &StrategyParams, idx: usize, indicators: &HashMap<String, Vec<Option<f64>>>, 
               position: &mut bool, entry_price: f64, candles_held: &mut i32, high_price: f64) -> (bool, String) {
    if !*position || idx < 1 { return (false, String::new()); }
    
    *candles_held += 1;
    let current_price = candles[idx].close;
    let pnl_pct = (current_price - entry_price) / entry_price;
    let leveraged_pnl = pnl_pct * params.leverage;
    
    if leveraged_pnl <= -params.stop_loss {
        *position = false;
        return (true, "SL".to_string());
    }
    
    if leveraged_pnl >= params.stop_loss * 2.0 {
        *position = false;
        return (true, "TP".to_string());
    }
    
    if leveraged_pnl >= params.stop_loss {
        let trail_price = high_price * (1.0 - params.stop_loss * 0.5);
        if current_price < trail_price {
            *position = false;
            return (true, "TS".to_string());
        }
    }
    
    if pnl_pct > 0.0 && *candles_held >= params.min_hold_candles {
        if let Some(ma) = indicators.get("sma").and_then(|v| v.get(idx)).flatten() {
            if current_price > *ma {
                *position = false;
                return (true, "SMA".to_string());
            }
        }
        
        if let Some(z) = indicators.get("z_score").and_then(|v| v.get(idx)).flatten() {
            if *z > 0.5 {
                *position = false;
                return (true, "Z0".to_string());
            }
        }
        
        if let Some(rsi) = indicators.get("rsi").and_then(|v| v.get(idx)).flatten() {
            if *rsi > 70.0 {
                *position = false;
                return (true, "RSI".to_string());
            }
        }
    }
    
    (false, String::new())
}

// =======================
// Indicators Calculation
// =

fn calculate_indicators(candles: &[Candle], params: &StrategyParams) -> HashMap<String, Vec<Option<f64>>> {
    let mut indicators = HashMap::new();
    
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let highs: Vec<f64> = candles.iter().map(|c| c.high).collect();
    let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();
    let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();
    
    indicators.insert("sma".to_string(), calculate_sma(&closes, params.ma_period as usize));
    
    let sma = &indicators["sma"];
    let stddev = calculate_stddev(&closes, 20);
    
    let z_score: Vec<Option<f64>> = closes.iter()
        .enumerate()
        .map(|(i, &price)| {
            if let (Some(m), Some(s)) = (sma.get(i).flatten(), stddev.get(i).flatten()) {
                if *s > 0.0 { Some((price - m) / s) } else { None }
            } else { None }
        })
        .collect();
    indicators.insert("z_score".to_string(), z_score);
    
    let sma_vec: Vec<f64> = sma.iter().flatten().cloned().collect();
    let std_vec: Vec<f64> = stddev.iter().flatten().cloned().collect();
    
    let bb_upper: Vec<Option<f64>> = closes.iter()
        .enumerate()
        .map(|(i, &price)| {
            if let (Some(m), Some(s)) = (sma_vec.get(i).cloned(), std_vec.get(i).cloned()) {
                Some(m + params.bb_std * s)
            } else { None }
        })
        .collect();
    let bb_lower: Vec<Option<f64>> = closes.iter()
        .enumerate()
        .map(|(i, &price)| {
            if let (Some(m), Some(s)) = (sma_vec.get(i).cloned(), std_vec.get(i).cloned()) {
                Some(m - params.bb_std * s)
            } else { None }
        })
        .collect();
    indicators.insert("bb_upper".to_string(), bb_upper);
    indicators.insert("bb_lower".to_string(), bb_lower);
    
    indicators.insert("rsi".to_string(), calculate_rsi(&closes, params.rsi_period as usize));
    
    let (macd, signal, hist) = calculate_macd(&closes);
    indicators.insert("macd".to_string(), macd);
    indicators.insert("macd_signal".to_string(), signal);
    indicators.insert("macd_hist".to_string(), hist);
    
    indicators.insert("atr".to_string(), calculate_atr(&highs, &lows, &closes, 14));
    indicators.insert("adx".to_string(), calculate_adx(&highs, &lows, &closes, 14));
    
    let stoch: Vec<Option<f64>> = closes.iter()
        .enumerate()
        .map(|(i, &price)| {
            if i < 14 { return None; }
            let low_14 = lows[i.saturating_sub(14)..=i].iter().cloned().fold(f64::INFINITY, f64::min);
            let high_14 = highs[i.saturating_sub(14)..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let range = high_14 - low_14;
            if range > 0.0 { Some((price - low_14) / range * 100.0) } else { None }
        })
        .collect();
    indicators.insert("stochastic".to_string(), stoch);
    
    let vol_ma = calculate_sma(&volumes, 20);
    let vol_ratio: Vec<Option<f64>> = volumes.iter()
        .enumerate()
        .map(|(i, &vol)| {
            if let Some(ma) = vol_ma.get(i).flatten() {
                if *ma > 0.0 { Some(vol / ma) } else { None }
            } else { None }
        })
        .collect();
    indicators.insert("volume_ratio".to_string(), vol_ratio);
    
    indicators
}

// =======================
// Backtest Engine
// =

fn run_backtest(candles: &[Candle], strategy: &str, params: &StrategyParams) -> BacktestResult {
    let indicators = calculate_indicators(candles, params);
    let initial_capital = 100.0;
    let mut balance = initial_capital;
    let mut position = false;
    let mut entry_price = 0.0;
    let mut high_price = 0.0;
    let mut candles_held = 0;
    let mut trades = Vec::new();
    
    for idx in 1..candles.len() {
        if !position {
            let entry = match strategy {
                "mean_reversion" => mean_reversion_entry(candles, params, idx, &indicators),
                "rsi_reversal" => rsi_reversal_entry(candles, params, idx, &indicators),
                "macd_cross" => macd_cross_entry(candles, params, idx, &indicators),
                "bb_bounce" => bb_bounce_entry(candles, params, idx, &indicators),
                "trend_follow" => trend_follow_entry(candles, params, idx, &indicators),
                "adr_reversal" => adr_reversal_entry(candles, params, idx, &indicators),
                "stochastic" => stochastic_entry(candles, params, idx, &indicators),
                "volume_spike" => volume_spike_entry(candles, params, idx, &indicators),
                "adx_breakout" => adx_breakout_entry(candles, params, idx, &indicators),
                "gap_fill" => gap_fill_entry(candles, params, idx, &indicators),
                "pivot_reversal" => pivot_reversal_entry(candles, params, idx, &indicators),
                "engulfing" => engulfs_entry(candles, params, idx, &indicators),
                "pin_bar" => pin_bar_entry(candles, params, idx, &indicators),
                _ => false,
            };
            
            if entry {
                position = true;
                entry_price = candles[idx].close;
                high_price = entry_price;
                candles_held = 0;
                balance -= balance * params.risk;
            }
        } else {
            if candles[idx].close > high_price {
                high_price = candles[idx].close;
            }
            
            let (exited, _reason) = exit_signal(candles, params, idx, &indicators, 
                                                &mut position, entry_price, &mut candles_held, high_price);
            
            if exited {
                let current_price = candles[idx].close;
                let pnl_pct = (current_price - entry_price) / entry_price;
                let leveraged_pnl = pnl_pct * params.leverage;
                let pnl = balance * params.risk * leveraged_pnl;
                balance += balance * params.risk + pnl;
                trades.push(pnl);
            }
        }
    }
    
    if position {
        let current_price = candles.last().unwrap().close;
        let pnl_pct = (current_price - entry_price) / entry_price;
        let leveraged_pnl = pnl_pct * params.leverage;
        let pnl = balance * params.risk * leveraged_pnl;
        balance += balance * params.risk + pnl;
        trades.push(pnl);
    }
    
    let total_trades = trades.len() as i32;
    let winning_trades = trades.iter().filter(|&&t| t > 0.0).count() as i32;
    let losing_trades = trades.iter().filter(|&&t| t < 0.0).count() as i32;
    
    let wins: Vec<f64> = trades.iter().filter(|&&t| t > 0.0).cloned().collect();
    let losses: Vec<f64> = trades.iter().filter(|&&t| t < 0.0).cloned().collect();
    
    let win_rate = if total_trades > 0 { winning_trades as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let avg_win = if !wins.is_empty() { wins.iter().sum::<f64>() / wins.len() as f64 } else { 0.0 };
    let avg_loss = if !losses.is_empty() { losses.iter().sum::<f64>() / losses.len() as f64 } else { 0.0 };
    
    let total_wins = wins.iter().sum::<f64>();
    let total_losses = losses.iter().sum::<f64>().abs();
    let profit_factor = if total_losses > 0.0 { total_wins / total_losses } else { 0.0 };
    
    let pnl_pct = (balance - initial_capital) / initial_capital * 100.0;
    
    BacktestResult {
        strategy: strategy.to_string(),
        coin: "multi".to_string(),
        initial_capital,
        final_capital: balance,
        total_trades,
        winning_trades,
        losing_trades,
        win_rate,
        avg_win,
        avg_loss,
        profit_factor,
        max_drawdown: 0.0,
        pnl_pct,
    }
}

// =======================
// Data Loading
// =

fn load_csv_data(path: &str) -> Vec<Candle> {
    let mut candles = Vec::new();
    
    if let Ok(mut rdr) = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path) 
    {
        for result in rdr.records() {
            if let Ok(record) = result {
                if let (Ok(ts), Ok(o), Ok(h), Ok(l), Ok(c), Ok(v)) = (
                    record.get(0).unwrap().parse::<i64>(),
                    record.get(1).unwrap().parse::<f64>(),
                    record.get(2).unwrap().parse::<f64>(),
                    record.get(3).unwrap().parse::<f64>(),
                    record.get(4).unwrap().parse::<f64>(),
                    record.get(5).unwrap().parse::<f64>(),
                ) {
                    candles.push(Candle { timestamp: ts, open: o, high: h, low: l, close: c, volume: v });
                }
            }
        }
    }
    
    candles
}

// =======================
// Main
// =

fn main() {
    println!("=== STRAT1000 Parallel Backtester (Rust + Rayon) ===");
    println!("Using multi-CPU parallel processing\n");
    
    let strategies = vec![
        "mean_reversion", "rsi_reversal", "macd_cross", "bb_bounce",
        "trend_follow", "adr_reversal", "stochastic", "volume_spike",
        "adx_breakout", "gap_fill", "pivot_reversal", "engulfing", "pin_bar",
    ];
    
    let stop_losses = vec![0.005, 0.01, 0.015, 0.02];
    let min_holds = vec![2, 4, 6, 8];
    let risks = vec![0.05, 0.10, 0.15];
    
    let mut param_sets = Vec::new();
    for sl in &stop_losses {
        for mh in &min_holds {
            for r in &risks {
                param_sets.push(StrategyParams {
                    stop_loss: *sl,
                    min_hold_candles: *mh,
                    risk: *r,
                    leverage: 5.0,
                    rsi_period: 14,
                    rsi_overbought: 70.0,
                    rsi_oversold: 30.0,
                    bb_period: 20,
                    bb_std: 2.0,
                    ma_period: 20,
                    z_score_threshold: 1.5,
                    volume_multiplier: 1.2,
                });
            }
        }
    }
    
    let mut test_configs = Vec::new();
    for strat in &strategies {
        for params in &param_sets {
            test_configs.push((strat.clone(), params.clone()));
        }
    }
    
    println!("Total configurations: {}", test_configs.len());
    
    let data_dir = "/home/scamarena/ProjectCoin/data_cache";
    let coin_files = vec![
        "ETH_USDT_15m_5months.csv",
        "BTC_USDT_15m_5months.csv",
        "SOL_USDT_15m_5months.csv",
        "DASH_USDT_15m_5months.csv",
    ];
    
    println!("\nRunning parallel backtests...\n");
    
    let results: Vec<BacktestResult> = test_configs
        .par_iter()
        .map(|(strategy, params)| {
            let mut strat_results = Vec::new();
            
            for coin_file in &coin_files {
                let path = format!("{}/{}", data_dir, coin_file);
                let candles = load_csv_data(&path);
                
                if candles.len() > 100 {
                    let mut result = run_backtest(&candles, strategy, params);
                    result.coin = coin_file.replace("_15m_5months.csv", "").replace("_", "/");
                    strat_results.push(result);
                }
            }
            
            if !strat_results.is_empty() {
                let total_trades: i32 = strat_results.iter().map(|r| r.total_trades).sum();
                let wins: f64 = strat_results.iter().map(|r| r.winning_trades as f64 * r.avg_win).sum();
                let losses: f64 = strat_results.iter().map(|r| r.losing_trades as f64 * r.avg_loss).sum();
                
                let combined_pnl: f64 = strat_results.iter().map(|r| r.pnl_pct).sum::<f64>() / strat_results.len() as f64;
                let avg_wins = wins / strat_results.len() as f64;
                let avg_losses = losses / strat_results.len() as f64;
                
                BacktestResult {
                    strategy: strategy.clone(),
                    coin: "COMBINED".to_string(),
                    initial_capital: 100.0,
                    final_capital: 100.0 * (1.0 + combined_pnl / 100.0),
                    total_trades,
                    winning_trades: strat_results.iter().map(|r| r.winning_trades).sum(),
                    losing_trades: strat_results.iter().map(|r| r.losing_trades).sum(),
                    win_rate: if total_trades > 0 { (strat_results.iter().map(|r| r.winning_trades).sum::<i32>()) as f64 / total_trades as f64 * 100.0 } else { 0.0 },
                    avg_win: avg_wins,
                    avg_loss: avg_losses,
                    profit_factor: if avg_losses.abs() > 0.0 { avg_wins / avg_losses.abs() } else { 0.0 },
                    max_drawdown: 0.0,
                    pnl_pct: combined_pnl,
                }
            } else {
                BacktestResult {
                    strategy: strategy.clone(),
                    coin: "COMBINED".to_string(),
                    initial_capital: 100.0,
                    final_capital: 100.0,
                    total_trades: 0,
                    winning_trades: 0,
                    losing_trades: 0,
                    win_rate: 0.0,
                    avg_win: 0.0,
                    avg_loss: 0.0,
                    profit_factor: 0.0,
                    max_drawdown: 0.0,
                    pnl_pct: 0.0,
                }
            }
        })
        .collect();
    
    let mut sorted_results = results.clone();
    sorted_results.sort_by(|a, b| b.profit_factor.partial_cmp(&a.profit_factor).unwrap());
    
    println!("\n=== TOP 20 STRATEGIES ===\n");
    println!("{:<20} {:>10} {:>10} {:>10} {:>10} {:>10}", 
             "Strategy", "P&L%", "WinRate%", "AvgWin", "AvgLoss", "PF");
    println!("{}", "-".repeat(75));
    
    for (i, result) in sorted_results.iter().take(20).enumerate() {
        if result.total_trades > 0 {
            println!("{:<20} {:>+10.1} {:>10.1} {:>+10.2} {:>+10.2} {:>10.2}", 
                     result.strategy, result.pnl_pct, result.win_rate, 
                     result.avg_win, result.avg_loss, result.profit_factor);
        }
    }
    
    println!("\n=== DONE ===");
}
