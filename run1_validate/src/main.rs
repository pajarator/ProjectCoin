//! RUN2 Validation: Test RUN2's top strategies on 5-month data
//!
//! RUN2 strategies (from ~7 days of data):
//!   1. VWAP Reversion  — vwap z < -1.5, price < SMA20, recovering
//!   2. Dual RSI         — RSI(7) < 30 AND RSI(14) < 35
//!   3. RSI Divergence   — price lower low + RSI higher low + RSI < 40
//!   Plus RUN1 holdovers: Mean Reversion, BB Bounce, Williams %R, ADR Reversal
//!
//! Three modes: RAW (RUN2 params: SL=2%, TP=1.5%), V13 (SL=0.3%, signal exit), REGIME

use rayon::prelude::*;
use std::fmt;

const DATA_DIR: &str = "/home/scamarena/ProjectCoin/data_cache";
const FEE: f64 = 0.001;
const SLIPPAGE: f64 = 0.0005;

// RUN2 original params
const RUN2_SL: f64 = 0.02;   // 2% stop loss
const RUN2_TP: f64 = 0.015;  // 1.5% take profit

// COINCLAW v13 params
const V13_SL: f64 = 0.003;
const V13_MIN_HOLD: usize = 2;
const V13_COOLDOWN: usize = 2;
const BREADTH_MAX: f64 = 0.20;

const COINS: &[&str] = &[
    "ADA", "ALGO", "ATOM", "AVAX", "BNB", "BTC", "DASH", "DOGE",
    "DOT", "ETH", "LINK", "LTC", "NEAR", "SHIB", "SOL", "TRX",
    "UNI", "XLM", "XRP",
];

const STRATEGIES: &[&str] = &[
    "vwap_reversion", "dual_rsi", "rsi_divergence",
    "mean_reversion", "bb_bounce", "williams_r", "adr_reversal",
];

// ═══════════════════════════════════════════
// Data
// ═══════════════════════════════════════════

#[derive(Clone)]
struct Candle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

fn load_csv(coin: &str) -> Vec<Candle> {
    let path = format!("{}/{}_USDT_15m_5months.csv", DATA_DIR, coin);
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)
        .unwrap_or_else(|e| panic!("Cannot open {}: {}", path, e));
    let mut candles = Vec::with_capacity(15000);
    for result in rdr.records() {
        let rec = match result { Ok(r) => r, Err(_) => continue };
        let o: f64 = rec[1].parse().unwrap_or(0.0);
        let h: f64 = rec[2].parse().unwrap_or(0.0);
        let l: f64 = rec[3].parse().unwrap_or(0.0);
        let c: f64 = rec[4].parse().unwrap_or(0.0);
        let v: f64 = rec[5].parse().unwrap_or(0.0);
        if c > 0.0 { candles.push(Candle { open: o, high: h, low: l, close: c, volume: v }); }
    }
    candles
}

// ═══════════════════════════════════════════
// Indicators
// ═══════════════════════════════════════════

fn sma(prices: &[f64], period: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; prices.len()];
    if prices.len() < period { return out; }
    let mut sum: f64 = prices[..period].iter().sum();
    out[period - 1] = sum / period as f64;
    for i in period..prices.len() {
        sum += prices[i] - prices[i - period];
        out[i] = sum / period as f64;
    }
    out
}

fn rolling_std(prices: &[f64], period: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; prices.len()];
    if prices.len() < period { return out; }
    for i in (period - 1)..prices.len() {
        let slice = &prices[i + 1 - period..=i];
        let mean = slice.iter().sum::<f64>() / period as f64;
        let var = slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (period - 1) as f64;
        out[i] = var.sqrt();
    }
    out
}

fn rsi_calc(prices: &[f64], period: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; prices.len()];
    if prices.len() < period + 1 { return out; }
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=period {
        let ch = prices[i] - prices[i - 1];
        if ch > 0.0 { avg_gain += ch; } else { avg_loss += -ch; }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;
    out[period] = if avg_loss == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + avg_gain / avg_loss) };
    for i in (period + 1)..prices.len() {
        let ch = prices[i] - prices[i - 1];
        let g = if ch > 0.0 { ch } else { 0.0 };
        let l = if ch < 0.0 { -ch } else { 0.0 };
        avg_gain = (avg_gain * (period - 1) as f64 + g) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + l) / period as f64;
        out[i] = if avg_loss == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + avg_gain / avg_loss) };
    }
    out
}

fn williams_r_ind(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Vec<f64> {
    let n = closes.len();
    let mut out = vec![f64::NAN; n];
    if n < period { return out; }
    for i in (period - 1)..n {
        let hh = highs[i + 1 - period..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let ll = lows[i + 1 - period..=i].iter().cloned().fold(f64::INFINITY, f64::min);
        let range = hh - ll;
        if range > 0.0 { out[i] = -100.0 * (hh - closes[i]) / range; }
    }
    out
}

fn bollinger_bands(prices: &[f64], period: usize, num_std: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mid = sma(prices, period);
    let sd = rolling_std(prices, period);
    let n = prices.len();
    let mut upper = vec![f64::NAN; n];
    let mut lower = vec![f64::NAN; n];
    for i in 0..n {
        if !mid[i].is_nan() && !sd[i].is_nan() {
            upper[i] = mid[i] + num_std * sd[i];
            lower[i] = mid[i] - num_std * sd[i];
        }
    }
    (upper, mid, lower)
}

/// Cumulative VWAP
fn vwap(candles: &[Candle]) -> Vec<f64> {
    let n = candles.len();
    let mut out = vec![f64::NAN; n];
    let mut cum_tpv = 0.0;
    let mut cum_vol = 0.0;
    for i in 0..n {
        let tp = (candles[i].high + candles[i].low + candles[i].close) / 3.0;
        cum_tpv += tp * candles[i].volume;
        cum_vol += candles[i].volume;
        if cum_vol > 0.0 { out[i] = cum_tpv / cum_vol; }
    }
    out
}

fn compute_z_scores(candles: &[Candle]) -> Vec<f64> {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let sma20 = sma(&closes, 20);
    let std20 = rolling_std(&closes, 20);
    let mut z = vec![f64::NAN; closes.len()];
    for i in 0..closes.len() {
        if !sma20[i].is_nan() && !std20[i].is_nan() && std20[i] > 0.0 {
            z[i] = (closes[i] - sma20[i]) / std20[i];
        }
    }
    z
}

/// Rolling min over `period` bars
fn rolling_min(data: &[f64], period: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (period - 1)..n {
        let min = data[i + 1 - period..=i].iter().cloned().fold(f64::INFINITY, f64::min);
        out[i] = min;
    }
    out
}

// ═══════════════════════════════════════════
// Strategy signals
// ═══════════════════════════════════════════

fn signals_vwap_reversion(candles: &[Candle]) -> (Vec<bool>, Vec<bool>) {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let vw = vwap(candles);
    let sma20 = sma(&closes, 20);
    let std20 = rolling_std(&closes, 20);
    let n = closes.len();

    let mut vwap_dist = vec![f64::NAN; n];
    for i in 0..n {
        if !vw[i].is_nan() && !std20[i].is_nan() && std20[i] > 0.0 {
            vwap_dist[i] = (closes[i] - vw[i]) / std20[i];
        }
    }

    let mut entry = vec![false; n];
    let mut exit = vec![false; n];
    for i in 2..n {
        if !vwap_dist[i].is_nan() && !vwap_dist[i-1].is_nan() && !vwap_dist[i-2].is_nan()
            && !sma20[i].is_nan()
        {
            let oversold = vwap_dist[i] < -1.5 && closes[i] < sma20[i];
            let recovering = vwap_dist[i] > vwap_dist[i-1] && vwap_dist[i-1] < vwap_dist[i-2];
            entry[i] = oversold && recovering;
            exit[i] = vwap_dist[i] > 0.0 || closes[i] > sma20[i];
        }
    }
    (entry, exit)
}

fn signals_dual_rsi(candles: &[Candle]) -> (Vec<bool>, Vec<bool>) {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let rsi7 = rsi_calc(&closes, 7);
    let rsi14 = rsi_calc(&closes, 14);
    let n = closes.len();
    let mut entry = vec![false; n];
    let mut exit = vec![false; n];
    for i in 0..n {
        if !rsi7[i].is_nan() && !rsi14[i].is_nan() {
            entry[i] = rsi7[i] < 30.0 && rsi14[i] < 35.0;
            exit[i] = rsi7[i] > 70.0 || rsi14[i] > 65.0;
        }
    }
    (entry, exit)
}

fn signals_rsi_divergence(candles: &[Candle]) -> (Vec<bool>, Vec<bool>) {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();
    let rsi14 = rsi_calc(&closes, 14);
    let sma20 = sma(&closes, 20);
    let n = closes.len();

    let mut entry = vec![false; n];
    let mut exit = vec![false; n];
    for i in 5..n {
        if !rsi14[i].is_nan() && !rsi14[i-5].is_nan() {
            let price_ll = lows[i] < lows[i - 5]; // price lower low
            let rsi_hl = rsi14[i] > rsi14[i - 5];  // RSI higher low
            entry[i] = price_ll && rsi_hl && rsi14[i] < 40.0;
        }
        if !rsi14[i].is_nan() && !sma20[i].is_nan() {
            exit[i] = rsi14[i] > 60.0 || closes[i] > sma20[i];
        }
    }
    (entry, exit)
}

fn signals_mean_reversion(candles: &[Candle]) -> (Vec<bool>, Vec<bool>) {
    let z = compute_z_scores(candles);
    let entry: Vec<bool> = z.iter().map(|&v| !v.is_nan() && v < -1.5).collect();
    let exit: Vec<bool> = z.iter().map(|&v| !v.is_nan() && v > 0.0).collect();
    (entry, exit)
}

fn signals_bb_bounce(candles: &[Candle]) -> (Vec<bool>, Vec<bool>) {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let (_upper, middle, lower) = bollinger_bands(&closes, 20, 2.0);
    let n = closes.len();
    let mut entry = vec![false; n];
    let mut exit = vec![false; n];
    for i in 1..n {
        if !lower[i].is_nan() && !lower[i-1].is_nan() {
            entry[i] = closes[i] <= lower[i] && closes[i-1] > lower[i-1];
        }
        if !middle[i].is_nan() { exit[i] = closes[i] >= middle[i]; }
    }
    (entry, exit)
}

fn signals_williams_r(candles: &[Candle]) -> (Vec<bool>, Vec<bool>) {
    let highs: Vec<f64> = candles.iter().map(|c| c.high).collect();
    let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let wr = williams_r_ind(&highs, &lows, &closes, 14);
    let n = wr.len();
    let mut entry = vec![false; n];
    let mut exit = vec![false; n];
    for i in 1..n {
        if !wr[i].is_nan() && !wr[i-1].is_nan() {
            entry[i] = wr[i] < -80.0 && wr[i-1] >= -80.0;
            exit[i] = wr[i] > -20.0 && wr[i-1] <= -20.0;
        }
    }
    (entry, exit)
}

fn signals_adr_reversal(candles: &[Candle]) -> (Vec<bool>, Vec<bool>) {
    let n = candles.len();
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let rsi_vals = rsi_calc(&closes, 14);
    let mut entry = vec![false; n];
    let mut exit = vec![false; n];
    for i in 24..n {
        let mut h24 = f64::NEG_INFINITY;
        let mut l24 = f64::INFINITY;
        for j in (i-23)..=i { if candles[j].high > h24 { h24 = candles[j].high; } if candles[j].low < l24 { l24 = candles[j].low; } }
        let range = h24 - l24;
        let at_ext = candles[i].close <= l24 + range * 0.25;
        let body = (candles[i].close - candles[i].open).abs();
        let lsh = candles[i].open.min(candles[i].close) - candles[i].low;
        let ush = candles[i].high - candles[i].open.max(candles[i].close);
        let hammer = lsh > 2.0 * body && body < ush;
        let rsi_os = !rsi_vals[i].is_nan() && rsi_vals[i] < 40.0;
        entry[i] = at_ext && (hammer || rsi_os);
        exit[i] = candles[i].close >= (h24 + l24) / 2.0;
    }
    (entry, exit)
}

fn get_signals(strategy: &str, candles: &[Candle]) -> (Vec<bool>, Vec<bool>) {
    match strategy {
        "vwap_reversion"  => signals_vwap_reversion(candles),
        "dual_rsi"        => signals_dual_rsi(candles),
        "rsi_divergence"  => signals_rsi_divergence(candles),
        "mean_reversion"  => signals_mean_reversion(candles),
        "bb_bounce"       => signals_bb_bounce(candles),
        "williams_r"      => signals_williams_r(candles),
        "adr_reversal"    => signals_adr_reversal(candles),
        _ => panic!("Unknown strategy: {}", strategy),
    }
}

// ═══════════════════════════════════════════
// Backtest result + stats
// ═══════════════════════════════════════════

#[derive(Clone)]
struct BacktestResult {
    strategy: String,
    coin: String,
    total_trades: usize,
    winning: usize,
    win_rate: f64,
    avg_win_pct: f64,
    avg_loss_pct: f64,
    profit_factor: f64,
    total_pnl_pct: f64,
    max_drawdown: f64,
}

impl fmt::Display for BacktestResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:>5} {:>16}  {:>4} trd  WR {:>5.1}%  PF {:>5.2}  P&L {:>+7.2}%  AvgW {:>+5.2}%  AvgL {:>+5.2}%  DD {:>5.2}%",
            self.coin, self.strategy, self.total_trades,
            self.win_rate, self.profit_factor, self.total_pnl_pct,
            self.avg_win_pct, self.avg_loss_pct, self.max_drawdown)
    }
}

fn stats_from_pnls(pnls: &[f64], strategy: &str, coin: &str) -> BacktestResult {
    let total = pnls.len();
    let wins: Vec<f64> = pnls.iter().filter(|&&p| p > 0.0).cloned().collect();
    let losses: Vec<f64> = pnls.iter().filter(|&&p| p <= 0.0).cloned().collect();
    let wr = if total > 0 { wins.len() as f64 / total as f64 * 100.0 } else { 0.0 };
    let aw = if !wins.is_empty() { wins.iter().sum::<f64>() / wins.len() as f64 } else { 0.0 };
    let al = if !losses.is_empty() { losses.iter().sum::<f64>() / losses.len() as f64 } else { 0.0 };
    let tw = wins.iter().sum::<f64>();
    let tl = losses.iter().sum::<f64>().abs();
    let pf = if tl > 0.0 { tw / tl } else if tw > 0.0 { 99.0 } else { 0.0 };
    let tp = pnls.iter().sum::<f64>();
    let mut eq = 100.0; let mut pk = eq; let mut mdd = 0.0_f64;
    for &p in pnls { eq *= 1.0 + p / 100.0; if eq > pk { pk = eq; } let dd = (pk - eq) / pk * 100.0; if dd > mdd { mdd = dd; } }
    BacktestResult { strategy: strategy.into(), coin: coin.into(), total_trades: total,
        winning: wins.len(), win_rate: wr, avg_win_pct: aw, avg_loss_pct: al,
        profit_factor: pf, total_pnl_pct: tp, max_drawdown: mdd }
}

// ═══════════════════════════════════════════
// Backtester: RUN2 style (SL=2%, TP=1.5%)
// ═══════════════════════════════════════════

fn backtest_run2(candles: &[Candle], entry: &[bool], exit: &[bool], strat: &str, coin: &str) -> BacktestResult {
    let n = candles.len();
    let mut pnls: Vec<f64> = Vec::new();
    let mut in_pos = false;
    let mut ep = 0.0;
    for i in 0..n {
        if in_pos {
            let p = candles[i].close;
            let pnl_raw = (p - ep) / ep;
            if pnl_raw <= -RUN2_SL {
                pnls.push((-RUN2_SL - 2.0 * FEE) * 100.0);
                in_pos = false; continue;
            }
            if pnl_raw >= RUN2_TP {
                pnls.push((RUN2_TP - 2.0 * FEE) * 100.0);
                in_pos = false; continue;
            }
            if exit[i] {
                let exit_p = p * (1.0 - SLIPPAGE);
                pnls.push(((exit_p / ep - 1.0) - 2.0 * FEE) * 100.0);
                in_pos = false; continue;
            }
        } else if entry[i] {
            ep = candles[i].close * (1.0 + SLIPPAGE);
            in_pos = true;
        }
    }
    if in_pos { pnls.push(((candles[n-1].close / ep - 1.0) - 2.0 * FEE) * 100.0); }
    stats_from_pnls(&pnls, strat, coin)
}

// ═══════════════════════════════════════════
// Backtester: V13 style (SL=0.3%, SMA20/Z exit)
// ═══════════════════════════════════════════

fn backtest_v13_inner(candles: &[Candle], entry_sig: &[bool], sma20: &[f64], z: &[f64]) -> Vec<f64> {
    let n = candles.len();
    let mut pnls: Vec<f64> = Vec::new();
    let mut in_pos = false;
    let mut ep = 0.0;
    let mut held: usize = 0;
    let mut cd: usize = 0;
    for i in 0..n {
        if in_pos {
            let p = candles[i].close;
            let pnl_raw = (p - ep) / ep;
            held += 1;
            if pnl_raw <= -V13_SL {
                pnls.push(((p * (1.0 - SLIPPAGE) / ep - 1.0) - 2.0 * FEE) * 100.0);
                in_pos = false; cd = V13_COOLDOWN; continue;
            }
            if pnl_raw > 0.0 && held >= V13_MIN_HOLD {
                if !sma20[i].is_nan() && p > sma20[i] {
                    pnls.push(((p * (1.0 - SLIPPAGE) / ep - 1.0) - 2.0 * FEE) * 100.0);
                    in_pos = false; cd = V13_COOLDOWN; continue;
                }
                if !z[i].is_nan() && z[i] > 0.5 {
                    pnls.push(((p * (1.0 - SLIPPAGE) / ep - 1.0) - 2.0 * FEE) * 100.0);
                    in_pos = false; cd = V13_COOLDOWN; continue;
                }
            }
        } else {
            if cd > 0 { cd -= 1; }
            else if entry_sig[i] { ep = candles[i].close * (1.0 + SLIPPAGE); in_pos = true; held = 0; }
        }
    }
    if in_pos { pnls.push(((candles[n-1].close / ep - 1.0) - 2.0 * FEE) * 100.0); }
    pnls
}

fn backtest_v13(candles: &[Candle], entry: &[bool], strat: &str, coin: &str) -> BacktestResult {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let sma20 = sma(&closes, 20);
    let z = compute_z_scores(candles);
    stats_from_pnls(&backtest_v13_inner(candles, entry, &sma20, &z), strat, coin)
}

// ═══════════════════════════════════════════
// Backtester: REGIME (breadth + V13)
// ═══════════════════════════════════════════

fn compute_breadth(all_z: &[Vec<f64>]) -> Vec<bool> {
    let n = all_z[0].len();
    let mut long_ok = vec![false; n];
    for i in 0..n {
        let mut valid = 0; let mut bearish = 0;
        for cz in all_z { if i < cz.len() && !cz[i].is_nan() { valid += 1; if cz[i] < -1.0 { bearish += 1; } } }
        if valid > 0 { long_ok[i] = bearish as f64 / valid as f64 <= BREADTH_MAX; }
    }
    let pct = long_ok.iter().filter(|&&b| b).count() as f64 / n as f64 * 100.0;
    println!("  Breadth: {:.1}% of bars in LONG mode", pct);
    long_ok
}

fn backtest_regime(candles: &[Candle], entry: &[bool], long_ok: &[bool], strat: &str, coin: &str) -> BacktestResult {
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let sma20 = sma(&closes, 20);
    let z = compute_z_scores(candles);
    let n = candles.len();
    let filtered: Vec<bool> = (0..n).map(|i| entry[i] && i < long_ok.len() && long_ok[i]).collect();
    stats_from_pnls(&backtest_v13_inner(candles, &filtered, &sma20, &z), strat, coin)
}

// ═══════════════════════════════════════════
// Reporting
// ═══════════════════════════════════════════

fn print_summary(label: &str, results: &[BacktestResult]) {
    for strat in STRATEGIES {
        let mut sr: Vec<&BacktestResult> = results.iter().filter(|r| r.strategy == *strat).collect();
        sr.sort_by(|a, b| b.win_rate.partial_cmp(&a.win_rate).unwrap());
        println!("━━━ {} ({}) ━━━", strat.to_uppercase(), label);
        for r in &sr {
            let m = if r.win_rate >= 70.0 && r.total_trades >= 5 { "✓" }
                    else if r.win_rate >= 60.0 && r.total_trades >= 5 { "~" }
                    else if r.profit_factor >= 1.0 && r.total_trades >= 5 { "+" }
                    else { " " };
            println!("  {} {}", m, r);
        }
        let tt: usize = sr.iter().map(|r| r.total_trades).sum();
        let tw: usize = sr.iter().map(|r| r.winning).sum();
        let wr = if tt > 0 { tw as f64 / tt as f64 * 100.0 } else { 0.0 };
        let act: Vec<&&BacktestResult> = sr.iter().filter(|r| r.total_trades > 0).collect();
        let pf = if !act.is_empty() { act.iter().map(|r| r.profit_factor).sum::<f64>() / act.len() as f64 } else { 0.0 };
        let pnl = sr.iter().map(|r| r.total_pnl_pct).sum::<f64>() / sr.len().max(1) as f64;
        let c70 = sr.iter().filter(|r| r.win_rate >= 70.0 && r.total_trades >= 5).count();
        let c_prof = sr.iter().filter(|r| r.profit_factor >= 1.0 && r.total_trades >= 5).count();
        println!("  ── {} trd | WR {:.1}% | PF {:.2} | P&L {:+.1}% | ≥70%: {} | PF≥1: {} (of {})\n",
            tt, wr, pf, pnl, c70, c_prof, COINS.len());
    }
}

// ═══════════════════════════════════════════
// Main
// ═══════════════════════════════════════════

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║   RUN2 VALIDATION — 7 strategies × 19 coins — RAW / V13 / REGIME       ║");
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("║  NEW: VWAP Reversion, Dual RSI, RSI Divergence                          ║");
    println!("║  HELD: Mean Reversion, BB Bounce, Williams %R, ADR Reversal             ║");
    println!("║  RAW = RUN2 params (SL=2%, TP=1.5%) | V13 = SL=0.3% + signal exit      ║");
    println!("║  REGIME = V13 + breadth ≤20%                                            ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");

    let coin_data: Vec<(&str, Vec<Candle>)> = COINS.par_iter()
        .map(|&coin| (coin, load_csv(coin))).collect();
    println!("Loaded {} coins ({} candles)\n", coin_data.len(),
        coin_data.iter().map(|(_, c)| c.len()).sum::<usize>());

    let all_z: Vec<Vec<f64>> = coin_data.iter().map(|(_, c)| compute_z_scores(c)).collect();
    let long_ok = compute_breadth(&all_z);

    // Pre-compute signals
    struct PC { coin: String, strat: String, entry: Vec<bool>, exit: Vec<bool> }
    let precomp: Vec<PC> = STRATEGIES.iter().flat_map(|&s| {
        coin_data.iter().map(move |(c, candles)| {
            let (e, x) = get_signals(s, candles);
            PC { coin: c.to_string(), strat: s.to_string(), entry: e, exit: x }
        })
    }).collect();

    // ── RUN2 RAW ──
    println!("\n{}\n  PART 1: RAW (RUN2 params: SL=2%, TP=1.5%)\n{}\n", "=".repeat(70), "=".repeat(70));
    let raw: Vec<BacktestResult> = precomp.par_iter().map(|pc| {
        let c = &coin_data.iter().find(|(n, _)| *n == pc.coin).unwrap().1;
        backtest_run2(c, &pc.entry, &pc.exit, &pc.strat, &pc.coin)
    }).collect();
    print_summary("RAW", &raw);

    // ── V13 ──
    println!("{}\n  PART 2: V13 (SL=0.3%, SMA20/Z exit)\n{}\n", "=".repeat(70), "=".repeat(70));
    let v13: Vec<BacktestResult> = precomp.par_iter().map(|pc| {
        let c = &coin_data.iter().find(|(n, _)| *n == pc.coin).unwrap().1;
        backtest_v13(c, &pc.entry, &pc.strat, &pc.coin)
    }).collect();
    print_summary("V13", &v13);

    // ── REGIME ──
    println!("{}\n  PART 3: REGIME (breadth + V13)\n{}\n", "=".repeat(70), "=".repeat(70));
    let reg: Vec<BacktestResult> = precomp.par_iter().map(|pc| {
        let c = &coin_data.iter().find(|(n, _)| *n == pc.coin).unwrap().1;
        backtest_regime(c, &pc.entry, &long_ok, &pc.strat, &pc.coin)
    }).collect();
    print_summary("REGIME", &reg);

    // ── Aggregate ──
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║                         AGGREGATE COMPARISON                            ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");

    println!("{:>16}  {:>7} {:>6} {:>6} {:>8} {:>3}  {:>7} {:>6} {:>6} {:>8} {:>3}  {:>7} {:>6} {:>6} {:>8} {:>3}",
        "Strategy", "RAW", "WR", "PF", "P&L", "70",  "V13", "WR", "PF", "P&L", "70",  "REG", "WR", "PF", "P&L", "70");
    println!("{}", "─".repeat(130));

    for strat in STRATEGIES {
        for (label, results) in [("RAW", &raw), ("V13", &v13), ("REG", &reg)] {
            let sr: Vec<&BacktestResult> = results.iter().filter(|r| r.strategy == *strat).collect();
            let tt: usize = sr.iter().map(|r| r.total_trades).sum();
            let tw: usize = sr.iter().map(|r| r.winning).sum();
            let wr = if tt > 0 { tw as f64 / tt as f64 * 100.0 } else { 0.0 };
            let act: Vec<&&BacktestResult> = sr.iter().filter(|r| r.total_trades > 0).collect();
            let pf = if !act.is_empty() { act.iter().map(|r| r.profit_factor).sum::<f64>() / act.len() as f64 } else { 0.0 };
            let pnl = sr.iter().map(|r| r.total_pnl_pct).sum::<f64>() / sr.len().max(1) as f64;
            let c70 = sr.iter().filter(|r| r.win_rate >= 70.0 && r.total_trades >= 5).count();
            if label == "RAW" {
                print!("{:>16}  {:>7} {:>5.1}% {:>5.2} {:>+7.1}% {:>3}", strat, tt, wr, pf, pnl, c70);
            } else {
                print!("  {:>7} {:>5.1}% {:>5.2} {:>+7.1}% {:>3}", tt, wr, pf, pnl, c70);
            }
        }
        println!();
    }

    // ── REGIME top 20 ──
    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║              REGIME TOP 20 (by PF, ≥5 trades)                           ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");

    let mut qual: Vec<&BacktestResult> = reg.iter().filter(|r| r.total_trades >= 5).collect();
    qual.sort_by(|a, b| b.profit_factor.partial_cmp(&a.profit_factor).unwrap());

    println!("{:>3}  {:>5} {:>16}  {:>4} {:>6} {:>6} {:>8} {:>7} {:>7} {:>6}",
        "#", "Coin", "Strategy", "Trd", "WR%", "PF", "P&L%", "AvgW%", "AvgL%", "DD%");
    println!("{}", "─".repeat(85));
    for (i, r) in qual.iter().take(20).enumerate() {
        let m = if r.profit_factor >= 1.0 { "+" } else { " " };
        println!("{} {:>2}  {:>5} {:>16}  {:>4} {:>5.1}% {:>5.2} {:>+7.2}% {:>+6.2}% {:>+6.2}% {:>5.2}%",
            m, i+1, r.coin, r.strategy, r.total_trades, r.win_rate, r.profit_factor,
            r.total_pnl_pct, r.avg_win_pct, r.avg_loss_pct, r.max_drawdown);
    }

    // ── RUN2 original claims vs now ──
    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║                  RUN2 CLAIMS vs REGIME (5mo data)                       ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");

    let claims: &[(&str, &str, f64, usize, f64)] = &[
        // coin, strat, run2_wr, run2_trades, run2_pf
        ("NEAR", "mean_reversion", 93.8, 16, 14.58),
        ("ETH",  "mean_reversion", 90.9, 11, 3.88),
        ("NEAR", "bb_bounce",      100.0, 9, 8.10),
        ("BTC",  "bb_bounce",      87.5, 8, 5.46),
        ("AVAX", "bb_bounce",      100.0, 5, 3.94),
        ("BTC",  "dual_rsi",       85.7, 14, 11.84),
        ("NEAR", "dual_rsi",       88.9, 9, 20.98),
        ("ATOM", "dual_rsi",       88.9, 9, 4.08),
        ("AVAX", "williams_r",     84.6, 13, 4.73),
        ("LTC",  "adr_reversal",   90.9, 11, 5.76),
    ];

    println!("  {:>5} {:>16}  {:>7} {:>5} {:>6}  {:>7} {:>5} {:>6}",
        "Coin", "Strategy", "R2 WR%", "Trd", "PF", "REG WR%", "Trd", "PF");
    println!("  {}", "─".repeat(68));
    for (coin, strat, r2wr, r2trd, r2pf) in claims {
        if let Some(r) = reg.iter().find(|r| r.coin == *coin && r.strategy == *strat) {
            let d = r.win_rate - r2wr;
            let a = if d > 2.0 { "▲" } else if d < -2.0 { "▼" } else { "≈" };
            println!("  {:>5} {:>16}  {:>6.1}% {:>5} {:>5.2}  {:>6.1}% {:>5} {:>5.2} {}",
                coin, strat, r2wr, r2trd, r2pf, r.win_rate, r.total_trades, r.profit_factor, a);
        }
    }
    println!("\n  RUN2 tested on ~7 days. Now on 5 months with regime filter.\n");
    println!("Done.");
}
