/// RUN106 — Hourly Scalp Cooldown: Session-Scaled Cooldown for Scalp Trades
///
/// Grid: ASIA_COOL [2, 3, 4] × US_COOL [1, 2] × LATE_COOL [2, 3]
/// 18 configs × 18 coins = 324 simulations (1m data, parallel per coin)
///
/// Run: cargo run --release --features run106 -- --run106

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SCALP_TP: f64 = 0.008;
const SCALP_SL: f64 = 0.001;
const SCALP_MAX_HOLD: u32 = 480;
const SCALP_VOL_MULT: f64 = 3.5;
const SCALP_RSI_EXTREME: f64 = 20.0;
const SCALP_STOCH_EXTREME: f64 = 5.0;
const SCALP_BB_SQUEEZE: f64 = 0.4;
const F6_DIR_ROC_3: f64 = -0.195;
const F6_AVG_BODY_3: f64 = 0.072;
const SCALP_RISK: f64 = 0.05;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct SessionCoolCfg {
    asia_cool: usize,
    us_cool: usize,
    late_cool: usize,
    is_baseline: bool,
}

impl SessionCoolCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("AS{}_US{}_LT{}", self.asia_cool, self.us_cool, self.late_cool) }
    }
}

fn build_grid() -> Vec<SessionCoolCfg> {
    let mut grid = vec![SessionCoolCfg { asia_cool: 2, us_cool: 2, late_cool: 2, is_baseline: true }];
    for &as_cool in &[2usize, 3, 4] {
        for &us_cool in &[1usize, 2] {
            for &lt_cool in &[2usize, 3] {
                grid.push(SessionCoolCfg { asia_cool: as_cool, us_cool: us_cool, late_cool: lt_cool, is_baseline: false });
            }
        }
    }
    grid
}

struct CoinData1m {
    name: String,
    close: Vec<f64>,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    vol: Vec<f64>,
    rsi: Vec<f64>,
    vol_ma: Vec<f64>,
    stoch_k: Vec<f64>,
    stoch_d: Vec<f64>,
    bb_upper: Vec<f64>,
    bb_lower: Vec<f64>,
    bb_width: Vec<f64>,
    bb_width_avg: Vec<f64>,
    roc_3: Vec<f64>,
    avg_body_3: Vec<f64>,
    timestamps: Vec<i64>, // Unix timestamp
}

struct ScalpPos { dir: i8, entry: f64, bars_held: u32 }

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    pnl: f64,
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    wr: f64,
    pf: f64,
    scalp_pnl: f64,
}
#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    wins: usize,
    losses: usize,
    pf: f64,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

// ── Rolling helpers ──────────────────────────────────────────────────────────
fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n]; let mut sum = 0.0;
    for i in 0..n { sum += data[i]; if i>=w { sum -= data[i-w]; } if i+1>=w { out[i]=sum/w as f64; } }
    out
}
fn rstd(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { let s=&data[i+1-w..=i]; let m=s.iter().sum::<f64>()/w as f64; let v=s.iter().map(|x|(x-m).powi(2)).sum::<f64>()/w as f64; out[i]=v.sqrt(); }
    out
}
fn rmin(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { out[i]=data[i+1-w..=i].iter().cloned().fold(f64::INFINITY, f64::min); }
    out
}
fn rmax(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { out[i]=data[i+1-w..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max); }
    out
}
fn rsi_calc(c: &[f64], period: usize) -> Vec<f64> {
    let n = c.len(); let mut out = vec![f64::NAN; n];
    if n < period+1 { return out; }
    let mut gains = vec![0.0; n]; let mut losses = vec![0.0; n];
    for i in 1..n { let d=c[i]-c[i-1]; if d>0.0{gains[i]=d;}else{losses[i]=-d;} }
    let ag=rmean(&gains,period); let al=rmean(&losses,period);
    for i in 0..n { if !ag[i].is_nan()&&!al[i].is_nan(){out[i]=if al[i]==0.0{100.0}else{100.0-100.0/(1.0+ag[i]/al[i])}; } }
    out
}

/// Get UTC hour from Unix timestamp (in seconds)
fn utc_hour(ts: i64) -> u32 {
    // Unix timestamp is seconds since epoch
    // UTC hour = (ts / 3600) % 24
    ((ts / 3600) % 24) as u32
}

/// Get session cooldown based on UTC hour
fn session_cooldown(hour: u32, cfg: &SessionCoolCfg) -> usize {
    if hour < 8 {
        cfg.asia_cool
    } else if hour >= 8 && hour < 13 {
        2 // EU: always 2 (baseline)
    } else if hour >= 13 && hour < 20 {
        cfg.us_cool
    } else {
        cfg.late_cool
    }
}

fn load_1m(coin: &str) -> Option<CoinData1m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut o=Vec::new(); let mut h=Vec::new(); let mut l=Vec::new(); let mut c=Vec::new(); let mut v=Vec::new(); let mut ts=Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let ts_str = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan()||hh.is_nan()||ll.is_nan()||cc.is_nan()||vv.is_nan() { continue; }
        let t: i64 = ts_str.parse().unwrap_or(0);
        o.push(oo); h.push(hh); l.push(ll); c.push(cc); v.push(vv); ts.push(t);
    }
    if c.len() < 100 { return None; }
    let n = c.len();
    let rsi = rsi_calc(&c, 14);
    let vol_ma = rmean(&v, 20);
    let ll14 = rmin(&l, 14); let hh14 = rmax(&h, 14);
    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n { if !ll14[i].is_nan() && hh14[i] > ll14[i] { stoch_k[i] = 100.0 * (c[i] - ll14[i]) / (hh14[i] - ll14[i]); } }
    let stoch_d = rmean(&stoch_k, 3);
    let bb_sma = rmean(&c, 20); let bb_std = rstd(&c, 20);
    let mut bb_upper = vec![f64::NAN; n]; let mut bb_lower = vec![f64::NAN; n]; let mut bb_width_raw = vec![f64::NAN; n];
    for i in 0..n { if !bb_sma[i].is_nan() && !bb_std[i].is_nan() { bb_upper[i] = bb_sma[i] + 2.0 * bb_std[i]; bb_lower[i] = bb_sma[i] - 2.0 * bb_std[i]; bb_width_raw[i] = bb_upper[i] - bb_lower[i]; } }
    let bb_width_avg = rmean(&bb_width_raw, 20);
    let mut roc_3 = vec![f64::NAN; n];
    for i in 3..n { if c[i-3] > 0.0 { roc_3[i] = (c[i] - c[i-3]) / c[i-3] * 100.0; } }
    let mut avg_body_3 = vec![f64::NAN; n];
    for i in 2..n {
        let b0 = (c[i]-o[i]).abs() / c[i] * 100.0;
        let b1 = (c[i-1]-o[i-1]).abs() / c[i-1] * 100.0;
        let b2 = (c[i-2]-o[i-2]).abs() / c[i-2] * 100.0;
        avg_body_3[i] = (b0 + b1 + b2) / 3.0;
    }
    Some(CoinData1m { name: coin.to_string(), close: c, open: o, high: h, low: l, vol: v, rsi, vol_ma, stoch_k, stoch_d, bb_upper, bb_lower, bb_width: bb_width_raw, bb_width_avg, roc_3, avg_body_3, timestamps: ts })
}

fn f6_pass(d: &CoinData1m, i: usize, dir: i8) -> bool {
    if d.roc_3[i].is_nan() || d.avg_body_3[i].is_nan() { return false; }
    let sign = if dir == 1 { 1.0 } else { -1.0 };
    d.roc_3[i] * sign < F6_DIR_ROC_3 && d.avg_body_3[i] > F6_AVG_BODY_3
}

fn scalp_signal(d: &CoinData1m, i: usize) -> Option<i8> {
    if i < 40 { return None; }
    if d.vol_ma[i].is_nan() || d.vol_ma[i] <= 0.0 { return None; }
    if d.rsi[i].is_nan() { return None; }
    let vol_r = d.vol[i] / d.vol_ma[i];
    let rsi_lo = SCALP_RSI_EXTREME;
    let rsi_hi = 100.0 - SCALP_RSI_EXTREME;

    if vol_r > SCALP_VOL_MULT {
        if d.rsi[i] < rsi_lo && f6_pass(d, i, 1) { return Some(1); }
        if d.rsi[i] > rsi_hi && f6_pass(d, i, -1) { return Some(-1); }
    }

    if i >= 1 {
        let sk = d.stoch_k[i]; let sd = d.stoch_d[i];
        let skp = d.stoch_k[i-1]; let sdp = d.stoch_d[i-1];
        if !sk.is_nan() && !sd.is_nan() && !skp.is_nan() && !sdp.is_nan() {
            let lo = SCALP_STOCH_EXTREME; let hi = 100.0 - SCALP_STOCH_EXTREME;
            if skp <= sdp && sk > sd && sk < lo && sd < lo && f6_pass(d, i, 1) { return Some(1); }
            if skp >= sdp && sk < sd && sk > hi && sd > hi && f6_pass(d, i, -1) { return Some(-1); }
        }
    }

    if !d.bb_width_avg[i].is_nan() && d.bb_width_avg[i] > 0.0 && !d.bb_upper[i].is_nan() {
        let squeeze = d.bb_width[i] < d.bb_width_avg[i] * SCALP_BB_SQUEEZE;
        if squeeze && vol_r > 2.0 {
            if d.close[i] > d.bb_upper[i] { return Some(1); }
            if d.close[i] < d.bb_lower[i] { return Some(-1); }
        }
    }
    None
}

fn simulate(d: &CoinData1m, cfg: &SessionCoolCfg) -> CoinResult {
    let n = d.close.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<ScalpPos> = None;
    let mut cooldown: usize = 0;
    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;

    for i in 40..n {
        if cooldown > 0 { cooldown -= 1; }

        if let Some(p) = pos.as_mut() {
            let pct = if p.dir == 1 { (d.close[i] - p.entry) / p.entry }
                      else { (p.entry - d.close[i]) / p.entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            if pct <= -SCALP_SL { exit_pct = -SCALP_SL; closed = true; }
            if !closed && pct >= SCALP_TP { exit_pct = SCALP_TP; closed = true; }
            p.bars_held += 1;
            if !closed && p.bars_held >= SCALP_MAX_HOLD { exit_pct = pct; closed = true; }

            if closed {
                let net = bal * SCALP_RISK * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                pos = None;
                // Session-aware cooldown: cooldown = session at EXIT bar
                let exit_hour = utc_hour(d.timestamps[i]);
                cooldown = session_cooldown(exit_hour, cfg);
            }
        } else if cooldown == 0 {
            if let Some(dir) = scalp_signal(d, i) {
                if i + 1 < n {
                    let entry = d.open[i + 1];
                    if entry > 0.0 {
                        pos = Some(ScalpPos { dir, entry, bars_held: 0 });
                    }
                }
            }
        }
    }

    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let avg_win = SCALP_RISK * LEVERAGE * SCALP_TP * wins as f64;
    let avg_loss = SCALP_RISK * LEVERAGE * SCALP_SL * losses as f64;
    let pf = if losses > 0 { avg_win / avg_loss } else { 0.0 };

    CoinResult { coin: d.name.clone(), pnl, trades: total_trades, wins, losses, flats, wr, pf, scalp_pnl: pnl }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN106 — Hourly Scalp Cooldown Grid Search\n");
    eprintln!("Loading 1m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData1m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_1m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.close.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing 1m data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let data: Vec<CoinData1m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins = {} simulations", grid.len(), N_COINS, grid.len() * N_COINS);

    let done = AtomicUsize::new(0);
    let total_sims = grid.len() * N_COINS;

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![] };
        }
        let coin_results: Vec<CoinResult> = data.iter().map(|d| simulate(d, cfg)).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_win_f = SCALP_RISK * LEVERAGE * SCALP_TP * (total_wins as f64);
        let avg_loss_f = SCALP_RISK * LEVERAGE * SCALP_SL * (total_losses as f64);
        let pf = if total_losses > 0 { avg_win_f / avg_loss_f } else { 0.0 };

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, is_baseline: cfg.is_baseline, coins: coin_results }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN106 Hourly Scalp Cooldown Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<18} {:>8} {:>8} {:>6} {:>7}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(55));
    for (i, r) in sorted.iter().enumerate().take(15) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<18} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(55));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN106 hourly scalp cooldown. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run106_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run106_1_results.json");
}
