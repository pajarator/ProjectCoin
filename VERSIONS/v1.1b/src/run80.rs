/// RUN80 — Volume Imbalance Confirmation
///
/// Require volume imbalance direction for regime entries:
/// LONG: imb >= LONG_MIN and rel_vol >= REL_VOL_MIN
/// SHORT: imb <= SHORT_MAX and rel_vol >= REL_VOL_MIN
///
/// Grid: WINDOW [10, 20, 30] × LONG_MIN [0.10, 0.15, 0.20] × SHORT_MAX [-0.10, -0.15, -0.20] × REL_VOL [1.0, 1.2, 1.5]
/// Total: 3 × 3 × 3 × 3 = 81 + baseline = 82 configs
///
/// Run: cargo run --release --features run80 -- --run80

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct VolImbCfg {
    window: usize,
    long_min: f64,
    short_max: f64,
    rel_vol_min: f64,
    is_baseline: bool,
}

impl VolImbCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("W{}_LM{:.2}_SM{:.2}_RV{:.1}", self.window, self.long_min, self.short_max.abs(), self.rel_vol_min) }
    }
}

fn build_grid() -> Vec<VolImbCfg> {
    let mut grid = vec![VolImbCfg { window: 0, long_min: 0.0, short_max: 0.0, rel_vol_min: 0.0, is_baseline: true }];
    let windows = [10, 20, 30];
    let long_mins = [0.10, 0.15, 0.20];
    let short_maxs = [-0.10, -0.15, -0.20];
    let rel_vols = [1.0, 1.2, 1.5];
    for &w in &windows {
        for &lm in &long_mins {
            for &sm in &short_maxs {
                for &rv in &rel_vols {
                    grid.push(VolImbCfg { window: w, long_min: lm, short_max: sm, rel_vol_min: rv, is_baseline: false });
                }
            }
        }
    }
    grid
}

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    volumes: Vec<f64>,
    zscore: Vec<f64>,
    vol_imb: Vec<f64>,   // [-1, +1]
    rel_vol: Vec<f64>,    // vol / vol_ma
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, blocked: usize }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, block_rate: f64, coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
    let mut highs = Vec::new(); let mut lows = Vec::new(); let mut volumes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() || hh.is_nan() || ll.is_nan() || vv.is_nan() { continue; }
        opens.push(oo); closes.push(cc); highs.push(hh); lows.push(ll); volumes.push(vv);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();

    // Z-score
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>()/20.0;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }

    // Volume MA(20)
    let mut vol_ma = vec![f64::NAN; n];
    for i in 20..n {
        vol_ma[i] = volumes[i+1-20..=i].iter().sum::<f64>() / 20.0;
    }

    // Relative volume
    let mut rel_vol = vec![f64::NAN; n];
    for i in 20..n {
        rel_vol[i] = if vol_ma[i] > 0.0 { volumes[i] / vol_ma[i] } else { 1.0 };
    }

    // Volume imbalance (body ratio method)
    let mut vol_imb = vec![f64::NAN; n];
    for i in 20..n {
        let mut buy_vol = 0.0;
        let mut sell_vol = 0.0;
        let window_start = i.saturating_sub(19);
        for j in window_start..=i {
            let body = (closes[j] - opens[j]).abs();
            let range = highs[j] - lows[j];
            if range == 0.0 { continue; }
            let body_ratio = body / range;
            let vol = volumes[j];
            if closes[j] > opens[j] {
                // Up candle
                buy_vol += vol * body_ratio;
                sell_vol += vol * (1.0 - body_ratio);
            } else {
                // Down candle
                sell_vol += vol * body_ratio;
                buy_vol += vol * (1.0 - body_ratio);
            }
        }
        let total = buy_vol + sell_vol;
        vol_imb[i] = if total > 0.0 { (buy_vol - sell_vol) / total } else { 0.0 };
    }

    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, volumes, zscore, vol_imb, rel_vol })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn passes_imb_filter(imb: f64, rel_v: f64, dir: i8, cfg: &VolImbCfg) -> bool {
    if cfg.is_baseline { return true; }
    if rel_v < cfg.rel_vol_min { return false; }
    if dir == 1 && imb < cfg.long_min { return false; }
    if dir == -1 && imb > cfg.short_max { return false; }
    true
}

fn simulate(d: &CoinData15m, cfg: &VolImbCfg) -> (f64, usize, usize, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut total_blocked = 0usize;
    let mut total_entries = 0usize;

    for i in 1..n {
        if let Some((dir, entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }

            if closed {
                let net = bal * POSITION_SIZE * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d.zscore[i]) {
                total_entries += 1;
                let imb = d.vol_imb[i];
                let rel_v = d.rel_vol[i];
                if !passes_imb_filter(imb, rel_v, dir, cfg) { total_blocked += 1; continue; }
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                }
            }
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats, total_blocked, total_entries)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN80 — Volume Imbalance Confirmation\n");
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref data) = loaded { eprintln!("  {} — {} bars", name, data.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, block_rate: 0.0, coins: vec![],
            };
        }
        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, flats, blocked, entries) = simulate(d, cfg);
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, wr, blocked }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_blocked: usize = coin_results.iter().map(|c| c.blocked).sum();
        let total_entries: usize = coin_results.iter().map(|c| c.trades + c.blocked).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins_sum as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
        let losses_f = coin_results.iter().map(|c| c.losses).sum::<usize>() as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };
        let block_rate = if total_entries > 0 { total_blocked as f64 / total_entries as f64 * 100.0 } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  blocked={:.1}%",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades, block_rate);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades,
            pf, is_baseline: cfg.is_baseline, block_rate, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN80 Volume Imbalance Confirmation Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<28} {:>8} {:>8} {:>6} {:>7} {:>8} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF", "Block%");
    println!("{}", "-".repeat(85));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<28} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2} {:>7.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf, r.block_rate);
    }
    println!("{}", "=".repeat(85));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN80 vol imbalance. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run80_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run80_1_results.json");
}
