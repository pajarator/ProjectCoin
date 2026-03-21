/// RUN38 — Volume-Volatility Event Proxy: Targeted Mean Reversion in High-Energy Windows
///
/// Hypothesis: COINCLAW regime trades have higher WR% and PF during "event proxy
/// windows" — periods where both volume AND volatility spike simultaneously.
/// These windows are detected purely from OHLCV data as simultaneous
/// volume and ATR anomalies, serving as a proxy for crypto event energy.
///
/// Grid search:
///   EVENT_VOL_MULT: [1.5, 2.0, 2.5, 3.0]
///   EVENT_ATR_MULT: [1.0, 1.5, 2.0]
///   EVENT_HOLD_BARS: [4, 8, 12]
///
/// Per coin:
///   1. Label all bars as in_event_window or normal
///   2. Run baseline COINCLAW v13 strategy (all bars)
///   3. Run strategy ONLY during event windows
///   4. Compare WR%, PF, P&L, max DD, trade count
///
/// Scoring: edge × sqrt(event_trades) (rewards both high edge and sufficient sample)
///
/// Run: cargo run --release --features run38 -- --run38

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// ── Constants ────────────────────────────────────────────────────────────────
const INITIAL_BAL: f64 = 1000.0;
const SL: f64 = 0.003;       // 0.3% stop loss (COINCLAW v13 default)
const MIN_HOLD_BARS: u32 = 2; // regime trades must hold ≥2 bars

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

// ── COINCLAW v13 per-coin strategy assignments ───────────────────────────────
// (From RUN11 / COINCLAW v13 configuration)
const COIN_STRATS: [&str; N_COINS] = [
    "OuMeanRev",  // DASH
    "BB_Bounce",  // UNI
    "VWAPRev",    // NEAR
    "ADRRev",     // ADA
    "BollingerBounce", // LTC
    "RSI_Rev",    // SHIB
    "BB_Bounce",  // LINK
    "VWAPRev",    // ETH
    "ADRRev",     // DOT
    "VWAPRev",    // XRP
    "RSI_Rev",    // ATOM
    "BB_Bounce",  // SOL
    "BollingerBounce", // DOGE
    "VWAPRev",    // XLM
    "BB_Bounce",  // AVAX
    "RSI_Rev",    // ALGO
    "BollingerBounce", // BNB
    "VWAPRev",    // BTC
];

// ── Grid config ─────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
struct EventGridEntry {
    vol_mult: f64,
    atr_mult: f64,
    hold_bars: u32,
}

impl EventGridEntry {
    fn label(&self) -> String {
        format!("vol{:.1}_atr{:.1}_hold{}", self.vol_mult, self.atr_mult, self.hold_bars)
    }
}

fn build_grid() -> Vec<EventGridEntry> {
    let mut grid = Vec::new();
    // Baseline (no event filter)
    grid.push(EventGridEntry { vol_mult: 0.0, atr_mult: 0.0, hold_bars: 0 });

    for vm in [1.5, 2.0, 2.5, 3.0] {
        for am in [1.0, 1.5, 2.0] {
            for hb in [4, 8, 12] {
                grid.push(EventGridEntry { vol_mult: vm, atr_mult: am, hold_bars: hb });
            }
        }
    }
    grid
}

// ── Data structures ─────────────────────────────────────────────────────────
struct CoinData {
    name: &'static str,
    close: Vec<f64>,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    volume: Vec<f64>,
    // 15m indicators
    rsi: Vec<f64>,
    sma20: Vec<f64>,
    bb_upper: Vec<f64>,
    bb_lower: Vec<f64>,
    atr: Vec<f64>,
    atr_ma: Vec<f64>,
    vol_ma: Vec<f64>,
    adx: Vec<f64>,
    vwap: Vec<f64>,
    zscore: Vec<f64>,
    // Market regime per bar: 1=long, -1=short, 0=neutral
    regime: Vec<i8>,
}

struct RegimePos {
    dir: i8,        // 1 = long, -1 = short
    entry: f64,
    entry_bar: usize,
    notional: f64,
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    strat: String,
    // Baseline
    base_trades: usize,
    base_wins: usize,
    base_losses: usize,
    base_flats: usize,
    base_wr: f64,
    base_pf: f64,
    base_pnl: f64,
    base_max_dd: f64,
    base_avg_win: f64,
    base_avg_loss: f64,
    // Event-filtered
    evt_trades: usize,
    evt_wins: usize,
    evt_losses: usize,
    evt_flats: usize,
    evt_wr: f64,
    evt_pf: f64,
    evt_pnl: f64,
    evt_max_dd: f64,
    evt_avg_win: f64,
    evt_avg_loss: f64,
    // Event window stats
    event_coverage_pct: f64,
    pct_trades_in_event: f64,
    wr_edge: f64,
    score: f64,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    vol_mult: f64,
    atr_mult: f64,
    hold_bars: u32,
    base_portfolio_pnl: f64,
    evt_portfolio_pnl: f64,
    pnl_delta: f64,
    base_portfolio_wr: f64,
    evt_portfolio_wr: f64,
    base_portfolio_trades: usize,
    evt_portfolio_trades: usize,
    coins_above_be: Vec<String>,
    coins: Vec<CoinResult>,
}

#[derive(Serialize)]
struct Output {
    notes: String,
    configs: Vec<ConfigResult>,
}

// ── Rolling helpers ──────────────────────────────────────────────────────────
fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let mut sum = 0.0;
    for i in 0..n {
        sum += data[i];
        if i >= w { sum -= data[i - w]; }
        if i + 1 >= w { out[i] = sum / w as f64; }
    }
    out
}

fn rstd(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        let s = &data[i + 1 - w..=i];
        let m = s.iter().sum::<f64>() / w as f64;
        let v = s.iter().map(|x| (x - m).powi(2)).sum::<f64>() / w as f64;
        out[i] = v.sqrt();
    }
    out
}

fn rmin(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::INFINITY, f64::min);
    }
    out
}

fn rmax(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w - 1)..n {
        out[i] = data[i + 1 - w..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    }
    out
}

fn rsi_calc(c: &[f64], period: usize) -> Vec<f64> {
    let n = c.len();
    let mut out = vec![f64::NAN; n];
    if n < period + 1 { return out; }
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        let d = c[i] - c[i - 1];
        if d > 0.0 { gains[i] = d; } else { losses[i] = -d; }
    }
    let ag = rmean(&gains, period);
    let al = rmean(&losses, period);
    for i in 0..n {
        if !ag[i].is_nan() && !al[i].is_nan() {
            out[i] = if al[i] == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag[i] / al[i]) };
        }
    }
    out
}

// ── ATR / ADX / VWAP ───────────────────────────────────────────────────────
fn compute_atr(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let n = high.len();
    let mut tr = vec![f64::NAN; n];
    // TR[i] for i >= 1: max(high-low, |high-close_prev|, |low-close_prev|)
    // TR[0] is NaN placeholder — rmean_nan will skip it
    for i in 1..n {
        let hl = (high[i] - low[i]).abs();
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }
    // Use rmean_nan so NaN at index 0 doesn't corrupt the rolling window
    rmean_nan(&tr, 14)
}

fn rmean_nan(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let mut sum = 0.0;
    let mut count = 0usize;
    for i in 0..n {
        if !data[i].is_nan() { sum += data[i]; count += 1; }
        if i >= w && !data[i - w].is_nan() { sum -= data[i - w]; count -= 1; }
        if i + 1 >= w && count > 0 { out[i] = sum / count as f64; }
    }
    out
}

fn compute_adx(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let n = high.len();
    let mut plus_dm = vec![0.0; n];
    let mut minus_dm = vec![0.0; n];
    for i in 1..n {
        let hh = high[i] - high[i - 1];
        let ll = low[i - 1] - low[i];
        if hh > ll && hh > 0.0 { plus_dm[i] = hh; }
        if ll > hh && ll > 0.0 { minus_dm[i] = ll; }
    }
    let atr = compute_atr(high, low, close);
    let pdm_avg = rmean_nan(&plus_dm, 14);
    let mdm_avg = rmean_nan(&minus_dm, 14);
    let mut dx = vec![f64::NAN; n];
    for i in 14..n {
        if atr[i] > 0.0 && !pdm_avg[i].is_nan() && !mdm_avg[i].is_nan() {
            let pdi = 100.0 * pdm_avg[i] / atr[i];
            let mdi = 100.0 * mdm_avg[i] / atr[i];
            let dsum = pdi + mdi;
            if dsum > 0.0 { dx[i] = 100.0 * (pdi - mdi).abs() / dsum; }
        }
    }
    rmean_nan(&dx, 14)
}

fn compute_vwap(high: &[f64], low: &[f64], close: &[f64], vol: &[f64]) -> Vec<f64> {
    let n = close.len();
    let mut out = vec![f64::NAN; n];
    let mut cumulative_tpv = 0.0;
    let mut cumulative_vol = 0.0;
    for i in 0..n {
        let typical = (high[i] + low[i] + close[i]) / 3.0;
        cumulative_tpv += typical * vol[i];
        cumulative_vol += vol[i];
        if cumulative_vol > 0.0 {
            out[i] = cumulative_tpv / cumulative_vol;
        }
    }
    out
}

// ── Regime direction (COINCLAW v13 style) ────────────────────────────────────
fn regime_direction(adx: f64, close: f64, sma20: f64) -> i8 {
    if adx.is_nan() { return 0; }
    if adx >= 25.0 {
        if close > sma20 { 1 } else { -1 }
    } else {
        0 // choppy / neutral
    }
}

// ── CSV loader ───────────────────────────────────────────────────────────────
fn load_15m(coin: &str) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
    let path = format!(
        "/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin
    );
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => { eprintln!("  Missing {}: {}", path, e); return None; }
    };
    let mut opens = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    let mut closes = Vec::new();
    let mut vols = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next();
        let o: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let h: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let l: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let c: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        let v: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        if o.is_nan()||h.is_nan()||l.is_nan()||c.is_nan()||v.is_nan() { continue; }
        opens.push(o); highs.push(h); lows.push(l); closes.push(c); vols.push(v);
    }
    if closes.len() < 100 { return None; }
    Some((opens, highs, lows, closes, vols))
}

// ── Compute 15m indicators ───────────────────────────────────────────────────
fn compute_15m_data(
    name: &'static str,
    o: Vec<f64>, h: Vec<f64>, l: Vec<f64>, c: Vec<f64>, v: Vec<f64>,
) -> CoinData {
    let n = c.len();

    let rsi = rsi_calc(&c, 14);
    let sma20 = rmean(&c, 20);

    let bb_std = rstd(&c, 20);
    let mut bb_upper = vec![f64::NAN; n];
    let mut bb_lower = vec![f64::NAN; n];
    for i in 0..n {
        if !sma20[i].is_nan() && !bb_std[i].is_nan() {
            bb_upper[i] = sma20[i] + 2.0 * bb_std[i];
            bb_lower[i] = sma20[i] - 2.0 * bb_std[i];
        }
    }

    let atr = compute_atr(&h, &l, &c);
    let atr_ma = rmean_nan(&atr, 20);
    let vol_ma = rmean(&v, 20);
    let adx = compute_adx(&h, &l, &c);
    let vwap = compute_vwap(&h, &l, &c, &v);

    // Z-score (20-bar rolling)
    let c_mean = rmean(&c, 20);
    let c_std = rstd(&c, 20);
    let mut zscore = vec![f64::NAN; n];
    for i in 0..n {
        if !c_mean[i].is_nan() && !c_std[i].is_nan() && c_std[i] > 0.0 {
            zscore[i] = (c[i] - c_mean[i]) / c_std[i];
        }
    }

    // Regime direction per bar
    let mut regime = vec![0i8; n];
    for i in 20..n {
        if !sma20[i].is_nan() {
            regime[i] = regime_direction(adx[i], c[i], sma20[i]);
        }
    }

    CoinData {
        name,
        close: c, open: o, high: h, low: l, volume: v,
        rsi, sma20, bb_upper, bb_lower,
        atr, atr_ma, vol_ma, adx, vwap, zscore,
        regime,
    }
}

// ── Event window labeling (pure) ─────────────────────────────────────────────
// bar i is in an event window if:
//   vol[i] >= vol_ma[i] * vol_mult  AND  atr[i] >= atr_ma[i] * atr_mult
// AND the window extends for hold_bars after the trigger bar
// Windows must be at least 20 bars apart (merge nearby spikes)
fn compute_event_windows(data: &CoinData, entry: &EventGridEntry) -> Vec<u8> {
    let n = data.close.len();
    if entry.vol_mult == 0.0 && entry.atr_mult == 0.0 {
        // Baseline: all bars are "in event"
        return vec![1u8; n];
    }

    let mut out = vec![0u8; n];
    let mut window_end = 0usize; // exclusive end of active window

    for i in 20..n {
        let vol_ok = !data.vol_ma[i].is_nan() && data.volume[i] >= data.vol_ma[i] * entry.vol_mult;
        let atr_ok = !data.atr_ma[i].is_nan() && data.atr[i] >= data.atr_ma[i] * entry.atr_mult;

        if vol_ok && atr_ok && i >= window_end {
            // Start of new event window
            window_end = (i + entry.hold_bars as usize).min(n);
        }

        out[i] = if i < window_end { 1 } else { 0 };
    }
    out
}

// ── Entry signals (COINCLAW v13 regime strategies) ─────────────────────────
// Returns 1=long, -1=short, 0=no signal
fn long_entry(data: &CoinData, strat: &str, i: usize) -> i8 {
    if i < 25 { return 0; }
    let c = data.close[i];
    let rsi = data.rsi[i];
    let bb_u = data.bb_upper[i];
    let bb_l = data.bb_lower[i];
    let z = data.zscore[i];
    let vwap = data.vwap[i];

    match strat {
        "VWAPRev" => {
            if !vwap.is_nan() && !rsi.is_nan() {
                let vwap_dev = (c - vwap) / vwap * 100.0;
                if vwap_dev < -0.5 && rsi < 40.0 { return 1; }
                if vwap_dev > 0.5 && rsi > 60.0 { return -1; }
            }
        }
        "BB_Bounce" | "BollingerBounce" => {
            if !bb_l.is_nan() && !bb_u.is_nan() && !rsi.is_nan() {
                if c <= bb_l && rsi < 30.0 { return 1; }
                if c >= bb_u && rsi > 70.0 { return -1; }
            }
        }
        "ADRRev" => {
            // Williams %R style — oversold/overbought
            let hh = rmax(&data.close, 14);
            let ll = rmin(&data.close, 14);
            if !hh[i].is_nan() && !ll[i].is_nan() && (hh[i] - ll[i]) > 0.0 {
                let wr = -100.0 * (hh[i] - c) / (hh[i] - ll[i]);
                if !rsi.is_nan() {
                    if rsi < 25.0 && wr < -80.0 { return 1; }
                    if rsi > 75.0 && wr > -20.0 { return -1; }
                }
            }
        }
        "RSI_Rev" => {
            if !rsi.is_nan() {
                if rsi < 30.0 { return 1; }
                if rsi > 70.0 { return -1; }
            }
        }
        "OuMeanRev" => {
            // DASH special: Z-score + OU regime detection
            if !z.is_nan() && !data.adx[i].is_nan() {
                let regime = data.regime[i];
                if z < -2.0 && regime == 1 { return 1; }
                if z > 2.0 && regime == -1 { return -1; }
            }
        }
        _ => {}
    }
    0
}

// ── Simulation ──────────────────────────────────────────────────────────────
struct SimResult {
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    pnl: f64,
    max_dd: f64,
    win_pnls: Vec<f64>,
    loss_pnls: Vec<f64>,
}

fn simulate_coin(data: &CoinData, strat: &str, event_window: Option<&[u8]>) -> SimResult {
    let n = data.close.len();
    let mut bal = INITIAL_BAL;
    let mut peak = INITIAL_BAL;
    let mut max_dd = 0.0;
    let mut pos: Option<RegimePos> = None;
    let mut cooldown = 0usize;

    let mut win_pnls = Vec::new();
    let mut loss_pnls = Vec::new();
    let mut flats = 0usize;

    for i in 1..n {
        // Update peak and drawdown
        if bal > peak { peak = bal; }
        let dd = (peak - bal) / peak * 100.0;
        if dd > max_dd { max_dd = dd; }

        // Position management
        if let Some(ref p) = pos {
            let pnl_pct = if p.dir == 1 {
                (data.close[i] - p.entry) / p.entry
            } else {
                (p.entry - data.close[i]) / p.entry
            };

            let bars_held = i - p.entry_bar;
            let exit = pnl_pct <= -SL || bars_held as u32 >= MIN_HOLD_BARS;

            if exit {
                let gross = p.notional * pnl_pct;
                let net = gross; // fees handled separately in real system
                bal += net;

                if net > 1e-8 {
                    win_pnls.push(net);
                } else if net < -1e-8 {
                    loss_pnls.push(net);
                } else {
                    flats += 1;
                }
                pos = None;
                cooldown = 3;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            // Entry check
            let dir = long_entry(data, strat, i);
            if dir != 0 {
                // Event filter: skip if in event window and event_window says so
                let in_event = event_window.map_or(true, |ew| ew[i] != 0);
                if !in_event {
                    // Not in event window — skip
                } else {
                    let regime_ok = if dir == 1 { data.regime[i] == 1 || data.regime[i] == 0 }
                                   else { data.regime[i] == -1 || data.regime[i] == 0 };
                    if regime_ok && i + 1 < n {
                        pos = Some(RegimePos {
                            dir,
                            entry: data.open[i + 1], // realistic fill at next bar open
                            entry_bar: i + 1,
                            notional: bal * 0.02, // 2% position size
                        });
                    }
                }
            }
        }
    }

    // Force close
    if let Some(ref p) = pos {
        let pnl_pct = if p.dir == 1 {
            (data.close[n - 1] - p.entry) / p.entry
        } else {
            (p.entry - data.close[n - 1]) / p.entry
        };
        let net = p.notional * pnl_pct;
        bal += net;
        if net > 1e-8 { win_pnls.push(net); }
        else if net < -1e-8 { loss_pnls.push(net); }
        else { flats += 1; }
    }

    let total = win_pnls.len() + loss_pnls.len() + flats;
    SimResult {
        trades: total,
        wins: win_pnls.len(),
        losses: loss_pnls.len(),
        flats,
        pnl: bal - INITIAL_BAL,
        max_dd,
        win_pnls,
        loss_pnls,
    }
}

// ── Entry point ─────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN38 — Volume-Volatility Event Proxy: Targeted Mean Reversion in High-Energy Windows");
    eprintln!("Grid: EVENT_VOL_MULT × EVENT_ATR_MULT × EVENT_HOLD_BARS");
    eprintln!();

    // Phase 1: Load data
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw: Vec<Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded {
            eprintln!("  {} — {} bars", name, d.3.len());
        }
        raw.push(loaded);
    }

    let mut all_ok = true;
    for (i, r) in raw.iter().enumerate() {
        if r.is_none() {
            eprintln!("ERROR: failed to load {}", COIN_NAMES[i]);
            all_ok = false;
        }
    }
    if !all_ok { return; }
    if shutdown.load(Ordering::SeqCst) { return; }

    // Phase 2: Compute indicators
    eprintln!("\nComputing 15m indicators...");
    let start = std::time::Instant::now();
    let coin_data: Vec<CoinData> = raw.into_par_iter().enumerate()
        .map(|(ci, r)| {
            let (o, h, l, c, v) = r.unwrap();
            compute_15m_data(COIN_NAMES[ci], o, h, l, c, v)
        })
        .collect();
    eprintln!("Indicators computed in {:.1}s", start.elapsed().as_secs_f64());

    // Quick diagnostic: check event coverage for vol=1.5,atr=1.0,hold=4 on BTC
    {
        let test_cfg = EventGridEntry { vol_mult: 1.5, atr_mult: 1.0, hold_bars: 4 };
        let btc_cd = &coin_data[17]; // BTC is index 17
        let ew = compute_event_windows(btc_cd, &test_cfg);
        let n = btc_cd.close.len();
        let bars_in_event: usize = ew.iter().map(|&v| v as usize).sum();
        eprintln!("\nDIAGNOSTIC: BTC vol=1.5 atr=1.0 hold=4: {}/{} bars in event ({:.1}%)",
            bars_in_event, n, bars_in_event as f64 / n as f64 * 100.0);
        // Check ATR and vol values
        eprintln!("  BTC vol_ma[100]={:.2}, atr_ma[100]={:.4}", btc_cd.vol_ma[100], btc_cd.atr_ma[100]);
        eprintln!("  BTC vol[100]={:.2}, atr[100]={:.4}", btc_cd.volume[100], btc_cd.atr[100]);
        // Check a range of atr_ma
        for i in [33, 50, 100, 200, 500, 1000] {
            eprintln!("  BTC atr_ma[{}]={:.4}, atr[{}]={:.4}", i, btc_cd.atr_ma[i], i, btc_cd.atr[i]);
        }
    }

    if shutdown.load(Ordering::SeqCst) { return; }

    // Phase 3: Run grid
    let grid = build_grid();
    eprintln!("\nSimulating {} configs × {} coins...", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(),
                vol_mult: cfg.vol_mult,
                atr_mult: cfg.atr_mult,
                hold_bars: cfg.hold_bars,
                base_portfolio_pnl: 0.0,
                evt_portfolio_pnl: 0.0,
                pnl_delta: 0.0,
                base_portfolio_wr: 0.0,
                evt_portfolio_wr: 0.0,
                base_portfolio_trades: 0,
                evt_portfolio_trades: 0,
                coins_above_be: vec![],
                coins: vec![],
            };
        }

        let coin_results: Vec<CoinResult> = coin_data.iter().zip(COIN_STRATS.iter())
            .map(|(cd, strat)| {
                // Compute event windows for this config
                let evt_windows = compute_event_windows(cd, cfg);
                let n = cd.close.len();

                // Baseline: all bars
                let base = simulate_coin(cd, strat, None);
                // Event-filtered: only event windows
                let evt = simulate_coin(cd, strat, Some(&evt_windows));

                let base_wr = if base.trades > 0 { base.wins as f64 / base.trades as f64 * 100.0 } else { 0.0 };
                let evt_wr  = if evt.trades > 0  { evt.wins as f64  / evt.trades as f64  * 100.0 } else { 0.0 };

                let base_avg_win = if base.wins > 0 { base.win_pnls.iter().sum::<f64>() / base.wins as f64 } else { 0.0 };
                let base_avg_loss = if base.losses > 0 { base.loss_pnls.iter().sum::<f64>() / base.losses as f64 } else { 0.0 };
                let evt_avg_win = if evt.wins > 0 { evt.win_pnls.iter().sum::<f64>() / evt.wins as f64 } else { 0.0 };
                let evt_avg_loss = if evt.losses > 0 { evt.loss_pnls.iter().sum::<f64>() / evt.losses as f64 } else { 0.0 };

                let base_pf = if base_avg_loss.abs() > 1e-8 { base_avg_win / base_avg_loss.abs() } else { 0.0 };
                let evt_pf = if evt_avg_loss.abs() > 1e-8 { evt_avg_win / evt_avg_loss.abs() } else { 0.0 };

                // Event coverage: % of bars in event window
                let event_bars: usize = evt_windows.iter().map(|&v| v as usize).sum();
                let event_coverage_pct = event_bars as f64 / n as f64 * 100.0;

                // % of baseline trades that fell in event windows
                let pct_trades_in_event = if base.trades > 0 { evt.trades as f64 / base.trades as f64 * 100.0 } else { 0.0 };

                let wr_edge = evt_wr - base_wr;
                // Score: edge × sqrt(event_trades) — rewards both high edge and sufficient sample
                let score = if evt.trades >= 10 { wr_edge * (evt.trades as f64).sqrt() } else { 0.0 };

                CoinResult {
                    coin: cd.name.to_string(),
                    strat: strat.to_string(),
                    base_trades: base.trades,
                    base_wins: base.wins,
                    base_losses: base.losses,
                    base_flats: base.flats,
                    base_wr,
                    base_pf,
                    base_pnl: base.pnl,
                    base_max_dd: base.max_dd,
                    base_avg_win,
                    base_avg_loss,
                    evt_trades: evt.trades,
                    evt_wins: evt.wins,
                    evt_losses: evt.losses,
                    evt_flats: evt.flats,
                    evt_wr,
                    evt_pf,
                    evt_pnl: evt.pnl,
                    evt_max_dd: evt.max_dd,
                    evt_avg_win,
                    evt_avg_loss,
                    event_coverage_pct,
                    pct_trades_in_event,
                    wr_edge,
                    score,
                }
            })
            .collect();

        let base_pnl_sum: f64 = coin_results.iter().map(|c| c.base_pnl).sum();
        let evt_pnl_sum: f64 = coin_results.iter().map(|c| c.evt_pnl).sum();
        let base_trades_sum: usize = coin_results.iter().map(|c| c.base_trades).sum();
        let evt_trades_sum: usize = coin_results.iter().map(|c| c.evt_trades).sum();
        let base_wins_sum: usize = coin_results.iter().map(|c| c.base_wins).sum();
        let evt_wins_sum: usize = coin_results.iter().map(|c| c.evt_wins).sum();
        let base_wr_sum = if base_trades_sum > 0 { base_wins_sum as f64 / base_trades_sum as f64 * 100.0 } else { 0.0 };
        let evt_wr_sum = if evt_trades_sum > 0 { evt_wins_sum as f64 / evt_trades_sum as f64 * 100.0 } else { 0.0 };

        // Coins with positive P&L delta (event-filtered vs baseline)
        let coins_above_be: Vec<String> = coin_results.iter()
            .filter(|c| c.base_trades >= 10 && c.evt_trades >= 10 && c.evt_pnl > c.base_pnl)
            .map(|c| format!("{}(ΔPnl={:+.1}, evt_WR={:.1}%, base_WR={:.1}%)", c.coin, c.evt_pnl - c.base_pnl, c.evt_wr, c.base_wr))
            .collect();

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {:<22} base_PnL={:>+8.1}  evt_PnL={:>+8.1}  ΔPnL={:>+7.1}  base_WR={:>5.1}%  evt_WR={:>5.1}%  edge={:>+5.1}pp  score={:>6.1}",
            d, total_cfgs, cfg.label(),
            base_pnl_sum, evt_pnl_sum, evt_pnl_sum - base_pnl_sum,
            base_wr_sum, evt_wr_sum, evt_wr_sum - base_wr_sum,
            coin_results.iter().map(|c| c.score).sum::<f64>());

        ConfigResult {
            label: cfg.label(),
            vol_mult: cfg.vol_mult,
            atr_mult: cfg.atr_mult,
            hold_bars: cfg.hold_bars,
            base_portfolio_pnl: base_pnl_sum,
            evt_portfolio_pnl: evt_pnl_sum,
            pnl_delta: evt_pnl_sum - base_pnl_sum,
            base_portfolio_wr: base_wr_sum,
            evt_portfolio_wr: evt_wr_sum,
            base_portfolio_trades: base_trades_sum,
            evt_portfolio_trades: evt_trades_sum,
            coins_above_be,
            coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("\nInterrupted — partial results not saved.");
        return;
    }

    // Phase 4: Summary
    eprintln!();

    // Sort by portfolio P&L delta
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a, b| b.pnl_delta.partial_cmp(&a.pnl_delta).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN38 Event Proxy Grid Search Results ===");
    println!("Sorted by P&L delta (event-filtered - baseline)");
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>8} {:>7} {:>7} {:>7} {:>7}",
        "#", "Config", "BasePnL", "EvtPnL", "ΔPnL", "BaseWR", "EvtWR", "Edge", "Score");
    println!("{}", "-".repeat(80));
    for (i, r) in sorted.iter().enumerate() {
        println!("{:>3}  {:<22} {:>+8.1} {:>+8.1} {:>+8.1} {:>6.1}% {:>6.1}% {:>+6.1}pp {:>7.1}",
            i + 1, r.label,
            r.base_portfolio_pnl, r.evt_portfolio_pnl, r.pnl_delta,
            r.base_portfolio_wr, r.evt_portfolio_wr,
            r.evt_portfolio_wr - r.base_portfolio_wr,
            r.coins.iter().map(|c| c.score).sum::<f64>());
    }
    println!("{}", "=".repeat(80));

    // Top config per-coin breakdown
    if let Some(best) = sorted.first() {
        println!("\n--- Per-coin breakdown: {} (best config) ---", best.label);
        println!("{:>6} {:>8} {:>7} {:>7} {:>7} {:>8} {:>7} {:>7} {:>7}",
            "Coin", "Strat", "EvtTrd", "EvtWR%", "EvtPF", "EvtPnL", "EvtDD%", "Edgepp", "Score");
        println!("{}", "-".repeat(75));
        for c in &best.coins {
            println!("{:>6} {:>8} {:>7} {:>6.1}% {:>7.2} {:>+8.1} {:>6.1}% {:>+7.1}pp {:>7.1}",
                c.coin, c.strat, c.evt_trades, c.evt_wr, c.evt_pf,
                c.evt_pnl, c.evt_max_dd, c.wr_edge, c.score);
        }
    }

    // Save
    let notes = format!(
        "RUN38 Event Proxy: vol+ATR spike windows as regime trade filter. {} configs. Grid: vol_mult × atr_mult × hold_bars. Baseline=COINCLAW v13 all bars.",
        results.len()
    );
    let output = Output { notes, configs: results };
    let json = serde_json::to_string_pretty(&output).unwrap();
    let path = "/home/scamarena/ProjectCoin/run38_1_results.json";
    std::fs::write(path, &json).ok();
    eprintln!("\nSaved → {}", path);
}
