/// RUN39 — Asymmetric Win/Loss Cooldown: Consecutive-Loss Escalation
///
/// Hypothesis: Wins and losses should have different cooldown periods:
///   - After a WIN (non-SL exit): shorter cooldown (1 bar) — market may continue
///   - After a LOSS (SL exit): longer cooldown (3 bars) — regime may have shifted
///   - Consecutive SLs: faster escalation than ISO-only 60-bar rule
///
/// Grid:
///   WIN_COOLDOWN:        [1, 2, 3]
///   LOSS_COOLDOWN:       [2, 3, 4, 5]
///   CONSEC_LOSS_2_CD:    [4, 6, 8, 12]
///   CONSEC_LOSS_3_CD:    [8, 12, 16, 20]
///
/// Per coin: 192 configs × 18 coins = 3,456 backtests
///
/// Run: cargo run --release --features run39 -- --run39

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// ── Constants ────────────────────────────────────────────────────────────────
const INITIAL_BAL: f64 = 1000.0;
const SL: f64 = 0.003;       // 0.3% stop loss (COINCLAW v13 default)
const MIN_HOLD_BARS: u32 = 2;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

const COIN_STRATS: [&str; N_COINS] = [
    "OuMeanRev", "BB_Bounce", "VWAPRev", "ADRRev", "BollingerBounce",
    "RSI_Rev", "BB_Bounce", "VWAPRev", "ADRRev", "VWAPRev",
    "RSI_Rev", "BB_Bounce", "BollingerBounce", "VWAPRev", "BB_Bounce",
    "RSI_Rev", "BollingerBounce", "VWAPRev",
];

// ── Grid config ─────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
struct CooldownGridEntry {
    win_cd: u32,
    loss_cd: u32,
    consec_2_cd: u32,
    consec_3_cd: u32,
}

impl CooldownGridEntry {
    fn label(&self) -> String {
        format!("W{}_L{}_C2_{}_C3_{}", self.win_cd, self.loss_cd, self.consec_2_cd, self.consec_3_cd)
    }
}

fn build_grid() -> Vec<CooldownGridEntry> {
    let mut grid = Vec::new();
    // Baseline: uniform cooldown = 2 (current COINCLAW v16 behavior)
    grid.push(CooldownGridEntry { win_cd: 2, loss_cd: 2, consec_2_cd: 60, consec_3_cd: 60 });

    for win_cd in [1u32, 2, 3] {
        for loss_cd in [2u32, 3, 4, 5] {
            for c2 in [4u32, 6, 8, 12] {
                for c3 in [8u32, 12, 16, 20] {
                    grid.push(CooldownGridEntry {
                        win_cd, loss_cd, consec_2_cd: c2, consec_3_cd: c3
                    });
                }
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
    rsi: Vec<f64>,
    sma20: Vec<f64>,
    bb_upper: Vec<f64>,
    bb_lower: Vec<f64>,
    atr: Vec<f64>,
    adx: Vec<f64>,
    vwap: Vec<f64>,
    zscore: Vec<f64>,
    regime: Vec<i8>,
}

struct RegimePos {
    dir: i8,
    entry: f64,
    entry_bar: usize,
    notional: f64,
}

struct CooldownState {
    cooldown: usize,
    last_was_win: Option<bool>,
    consec_loss_streak: u32,
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    strat: String,
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    wr: f64,
    pf: f64,
    pnl: f64,
    max_dd: f64,
    avg_win: f64,
    avg_loss: f64,
    cascade_count: usize,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    win_cd: u32,
    loss_cd: u32,
    consec_2_cd: u32,
    consec_3_cd: u32,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    total_cascades: usize,
    pf: f64,
    coins: Vec<CoinResult>,
    is_baseline: bool,
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

fn compute_atr(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let n = high.len();
    let mut tr = vec![f64::NAN; n];
    for i in 1..n {
        let hl = (high[i] - low[i]).abs();
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }
    rmean_nan(&tr, 14)
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

fn regime_direction(adx: f64, close: f64, sma20: f64) -> i8 {
    if adx.is_nan() { return 0; }
    if adx >= 25.0 {
        if close > sma20 { 1 } else { -1 }
    } else { 0 }
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
        if cumulative_vol > 0.0 { out[i] = cumulative_tpv / cumulative_vol; }
    }
    out
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
    let adx = compute_adx(&h, &l, &c);
    let vwap = compute_vwap(&h, &l, &c, &v);

    let c_mean = rmean(&c, 20);
    let c_std = rstd(&c, 20);
    let mut zscore = vec![f64::NAN; n];
    for i in 0..n {
        if !c_mean[i].is_nan() && !c_std[i].is_nan() && c_std[i] > 0.0 {
            zscore[i] = (c[i] - c_mean[i]) / c_std[i];
        }
    }

    let mut regime = vec![0i8; n];
    for i in 20..n {
        if !sma20[i].is_nan() {
            regime[i] = regime_direction(adx[i], c[i], sma20[i]);
        }
    }

    CoinData { name, close: c, open: o, high: h, low: l, volume: v,
        rsi, sma20, bb_upper, bb_lower, atr, adx, vwap, zscore, regime }
}

// ── Entry signals ────────────────────────────────────────────────────────────
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
            if !z.is_nan() && !data.adx[i].is_nan() {
                let reg = data.regime[i];
                if z < -2.0 && reg == 1 { return 1; }
                if z > 2.0 && reg == -1 { return -1; }
            }
        }
        _ => {}
    }
    0
}

// ── Simulation (cooldown-aware) ───────────────────────────────────────────────
struct SimResult {
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    pnl: f64,
    max_dd: f64,
    win_pnls: Vec<f64>,
    loss_pnls: Vec<f64>,
    cascade_count: usize,
}

fn simulate_coin(data: &CoinData, strat: &str, cfg: &CooldownGridEntry) -> SimResult {
    let n = data.close.len();
    let mut bal = INITIAL_BAL;
    let mut peak = INITIAL_BAL;
    let mut max_dd = 0.0;
    let mut pos: Option<RegimePos> = None;
    let mut cd = CooldownState {
        cooldown: 0,
        last_was_win: None,
        consec_loss_streak: 0,
    };
    let mut prev_entry_bar = 0usize;
    let mut bars_since_prev_entry = 0usize;

    let mut win_pnls = Vec::new();
    let mut loss_pnls = Vec::new();
    let mut flats = 0usize;
    let mut cascade_count = 0usize;

    for i in 1..n {
        if bal > peak { peak = bal; }
        let dd = (peak - bal) / peak * 100.0;
        if dd > max_dd { max_dd = dd; }

        if prev_entry_bar > 0 {
            bars_since_prev_entry = i - prev_entry_bar;
        }

        if let Some(ref p) = pos {
            let pnl_pct = if p.dir == 1 {
                (data.close[i] - p.entry) / p.entry
            } else {
                (p.entry - data.close[i]) / p.entry
            };

            let bars_held = i - p.entry_bar;
            let hit_sl = pnl_pct <= -SL;
            let min_hold_done = bars_held as u32 >= MIN_HOLD_BARS;
            let exit = hit_sl || min_hold_done;

            if exit {
                let gross = p.notional * pnl_pct;
                let net = gross;
                bal += net;
                prev_entry_bar = p.entry_bar;

                let is_win = net > 1e-8;
                let is_loss = net < -1e-8;

                if is_win {
                    win_pnls.push(net);
                } else if is_loss {
                    loss_pnls.push(net);
                } else {
                    flats += 1;
                }

                // Cascade: win→loss within 3 bars
                if is_loss && cd.last_was_win == Some(true) && bars_since_prev_entry <= 3 {
                    cascade_count += 1;
                }

                if is_loss {
                    cd.consec_loss_streak += 1;
                } else {
                    cd.consec_loss_streak = 0;
                }
                cd.last_was_win = Some(is_win);

                // Compute cooldown
                cd.cooldown = if cd.consec_loss_streak == 0 {
                    if !is_loss { cfg.win_cd as usize } else { cfg.loss_cd as usize }
                } else if cd.consec_loss_streak == 1 {
                    cfg.loss_cd as usize
                } else if cd.consec_loss_streak == 2 {
                    cfg.consec_2_cd as usize
                } else {
                    cfg.consec_3_cd as usize
                };

                pos = None;
            }
        } else if cd.cooldown > 0 {
            cd.cooldown -= 1;
        } else {
            let dir = long_entry(data, strat, i);
            if dir != 0 {
                let regime_ok = if dir == 1 { data.regime[i] == 1 || data.regime[i] == 0 }
                               else { data.regime[i] == -1 || data.regime[i] == 0 };
                if regime_ok && i + 1 < n {
                    pos = Some(RegimePos {
                        dir,
                        entry: data.open[i + 1],
                        entry_bar: i + 1,
                        notional: bal * 0.02,
                    });
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
        trades: total, wins: win_pnls.len(), losses: loss_pnls.len(), flats,
        pnl: bal - INITIAL_BAL, max_dd, win_pnls, loss_pnls, cascade_count,
    }
}

// ── Entry point ─────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN39 — Asymmetric Win/Loss Cooldown Grid Search");
    eprintln!("WIN_COOLDOWN × LOSS_COOLDOWN × CONSEC_LOSS_2_CD × CONSEC_LOSS_3_CD");
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
                win_cd: cfg.win_cd, loss_cd: cfg.loss_cd,
                consec_2_cd: cfg.consec_2_cd, consec_3_cd: cfg.consec_3_cd,
                total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                total_cascades: 0, pf: 0.0, coins: vec![],
                is_baseline: cfg.win_cd == 2 && cfg.loss_cd == 2 && cfg.consec_2_cd == 60,
            };
        }

        let coin_results: Vec<CoinResult> = coin_data.iter().zip(COIN_STRATS.iter())
            .map(|(cd, strat)| {
                let r = simulate_coin(cd, strat, cfg);
                let wr = if r.trades > 0 { r.wins as f64 / r.trades as f64 * 100.0 } else { 0.0 };
                let avg_win = if r.wins > 0 { r.win_pnls.iter().sum::<f64>() / r.wins as f64 } else { 0.0 };
                let avg_loss = if r.losses > 0 { r.loss_pnls.iter().sum::<f64>() / r.losses as f64 } else { 0.0 };
                let pf = if avg_loss.abs() > 1e-8 { avg_win / avg_loss.abs() } else { 0.0 };
                CoinResult {
                    coin: cd.name.to_string(),
                    strat: strat.to_string(),
                    trades: r.trades, wins: r.wins, losses: r.losses, flats: r.flats,
                    wr, pf, pnl: r.pnl, max_dd: r.max_dd,
                    avg_win, avg_loss,
                    cascade_count: r.cascade_count,
                }
            })
            .collect();

        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_cascades: usize = coin_results.iter().map(|c| c.cascade_count).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let wins_total: usize = coin_results.iter().filter(|c| c.wins > 0).map(|c| c.wins).sum();
        let losses_total: usize = coin_results.iter().filter(|c| c.losses > 0).map(|c| c.losses).sum();
        let avg_win_all = if wins_total > 0 {
            coin_results.iter().filter(|c| c.wins > 0)
                .map(|c| c.avg_win * c.wins as f64).sum::<f64>() / wins_total as f64
        } else { 0.0 };
        let avg_loss_all = if losses_total > 0 {
            coin_results.iter().filter(|c| c.losses > 0)
                .map(|c| c.avg_loss * c.losses as f64).sum::<f64>() / losses_total as f64
        } else { 0.0 };
        let pf = if avg_loss_all.abs() > 1e-8 { avg_win_all / avg_loss_all.abs() } else { 0.0 };

        let is_baseline = cfg.win_cd == 2 && cfg.loss_cd == 2 && cfg.consec_2_cd == 60;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {:<28} trades={:>5}  WR={:>5.1}%  PnL={:>+8.1}  PF={:.3}  cascades={:>4}",
            d, total_cfgs, cfg.label(), total_trades, portfolio_wr, total_pnl, pf, total_cascades);

        ConfigResult {
            label: cfg.label(),
            win_cd: cfg.win_cd, loss_cd: cfg.loss_cd,
            consec_2_cd: cfg.consec_2_cd, consec_3_cd: cfg.consec_3_cd,
            total_pnl, portfolio_wr, total_trades,
            total_cascades, pf,
            coins: coin_results,
            is_baseline,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("\nInterrupted — saving partial results...");
        let notes = format!("RUN39 interrupted at {} configs", results.len());
        let output = Output { notes, configs: results };
        let json = serde_json::to_string_pretty(&output).unwrap();
        std::fs::write("/home/scamarena/ProjectCoin/run39_1_results.json", &json).ok();
        return;
    }

    // Phase 4: Summary
    eprintln!();

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();

    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a, b| {
        let delta_a = a.total_pnl - baseline.total_pnl;
        let delta_b = b.total_pnl - baseline.total_pnl;
        delta_b.partial_cmp(&delta_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("\n=== RUN39 Cooldown Grid Results ===");
    println!("Baseline: W=2 L=2 C2=60 C3=60 → PnL={:+.1}  WR={:.1}%  PF={:.3}  cascades={}",
        baseline.total_pnl, baseline.portfolio_wr, baseline.pf, baseline.total_cascades);
    println!("\n{:>3}  {:<28} {:>8} {:>7} {:>8} {:>8} {:>7} {:>6}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "PF", "Casc", "ΔCasc");
    println!("{}", "-".repeat(80));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        let delta_casc = r.total_cascades as i32 - baseline.total_cascades as i32;
        println!("{:>3}  {:<28} {:>+8.1} {:>+8.1} {:>6.1}% {:>8.3} {:>6} {:>+6}",
            i + 1, r.label, r.total_pnl, delta, r.portfolio_wr,
            r.pf, r.total_cascades, delta_casc);
        if i >= 19 { println!("  ... ({} more configs)", sorted.len() - 20); break; }
    }
    println!("{}", "=".repeat(80));

    let best = sorted.first().unwrap();
    if !best.is_baseline {
        println!("\n--- Per-coin: {} (best) vs baseline ---", best.label);
        println!("{:>6} {:>8} {:>7} {:>7} {:>7} {:>8} {:>7}",
            "Coin", "Strat", "Trades", "WR%", "PF", "PnL", "Casc");
        println!("{}", "-".repeat(55));
        for c in &best.coins {
            println!("{:>6} {:>8} {:>7} {:>6.1}% {:>7.3} {:>+8.1} {:>7}",
                c.coin, c.strat, c.trades, c.wr, c.pf, c.pnl, c.cascade_count);
        }
    }

    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {} (best ΔPnL={:+.1})",
        if is_positive { "POSITIVE — proceed to walk-forward" } else { "NEGATIVE — no config beats baseline" },
        best.total_pnl - baseline.total_pnl);

    let notes = format!(
        "RUN39 asymmetric cooldown grid. {} configs. Baseline (W=2,L=2,C2=60,C3=60): PnL={:+.1}, WR={:.1}%, PF={:.3}. Best: {} (PnL={:+.1}, Δ={:+.1})",
        results.len(), baseline.total_pnl, baseline.portfolio_wr, baseline.pf,
        best.label, best.total_pnl, best.total_pnl - baseline.total_pnl
    );
    let output = Output { notes, configs: results };
    let json = serde_json::to_string_pretty(&output).unwrap();
    let path = "/home/scamarena/ProjectCoin/run39_1_results.json";
    std::fs::write(path, &json).ok();
    eprintln!("\nSaved → {}", path);
}
