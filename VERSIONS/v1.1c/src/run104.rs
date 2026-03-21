/// RUN104 — Volume Dry-Up Exit: Exit When Volume Collapses During Profitable Trades
///
/// Grid: VOL_THRESH [0.30, 0.40, 0.50] × VOL_BARS [1, 2, 3] × MIN_HOLD [5, 8, 12]
/// 27 configs × 18 coins = 486 simulations (parallel per coin)
///
/// Run: cargo run --release --features run104 -- --run104

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

#[derive(Clone, Copy, PartialEq, Debug)]
struct VolDryupCfg {
    vol_thresh: f64,
    vol_bars: usize,
    min_hold: usize,
    is_baseline: bool,
}

impl VolDryupCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("VT{:.2}_VB{}_MH{}", self.vol_thresh, self.vol_bars, self.min_hold) }
    }
}

fn build_grid() -> Vec<VolDryupCfg> {
    let mut grid = vec![VolDryupCfg { vol_thresh: 999.0, vol_bars: 999, min_hold: 999, is_baseline: true }];
    for &vt in &[0.30, 0.40, 0.50] {
        for &vb in &[1usize, 2, 3] {
            for &mh in &[5usize, 8, 12] {
                grid.push(VolDryupCfg { vol_thresh: vt, vol_bars: vb, min_hold: mh, is_baseline: false });
            }
        }
    }
    grid
}

struct CoinData {
    name: String,
    opens: Vec<f64>,
    closes: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    zscore: Vec<f64>,
    vol: Vec<f64>,
    vol_ma: Vec<f64>,
}

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
    dryup_exits: usize,
    dryup_exit_wins: usize,
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
    dryup_exit_total: usize,
    dryup_exit_wins_total: usize,
    dryup_exit_rate: f64,
    dryup_exit_win_rate: f64,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if w == 0 || n < w { return out; }
    let mut sum = 0.0;
    for i in 0..n {
        if i >= w { sum -= data[i - w]; }
        sum += data[i];
        if i + 1 >= w { out[i] = sum / w as f64; }
    }
    out
}

fn load_15m(coin: &str) -> Option<CoinData> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
    let mut highs = Vec::new(); let mut lows = Vec::new();
    let mut vol = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); highs.push(hh); lows.push(ll); closes.push(cc); vol.push(vv);
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

    // Volume MA (20-bar)
    let vol_ma = rmean(&vol, 20);

    Some(CoinData { name: coin.to_string(), opens, closes, highs, lows, zscore, vol, vol_ma })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData, cfg: &VolDryupCfg) -> CoinResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None;
    let mut cooldown = 0usize;

    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut dryup_exits = 0usize;
    let mut dryup_exit_wins = 0usize;

    // Volume dryup consecutive bar counter
    let mut vol_dryup_consec = 0usize;

    for i in 20..n {
        cooldown = cooldown.saturating_sub(1);

        if let Some((dir, entry, entry_bar, _z_entry)) = pos.as_mut() {
            let pct = if *dir == 1 { (d.closes[i]-*entry)/ *entry } else { (*entry-d.closes[i])/ *entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            // SL
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }

            // Volume dryup exit
            if !closed && !cfg.is_baseline {
                let held_bars = i.saturating_sub(*entry_bar);
                if held_bars >= cfg.min_hold && pct > 0.0 {
                    let vol_ratio = if d.vol_ma[i] > 0.0 { d.vol[i] / d.vol_ma[i] } else { 1.0 };
                    if vol_ratio < cfg.vol_thresh {
                        vol_dryup_consec += 1;
                    } else {
                        vol_dryup_consec = 0;
                    }
                    if vol_dryup_consec >= cfg.vol_bars {
                        exit_pct = pct;
                        closed = true;
                        dryup_exits += 1;
                        if pct > 0.0 { dryup_exit_wins += 1; }
                        vol_dryup_consec = 0;
                    }
                } else {
                    vol_dryup_consec = 0;
                }
            }

            // Regime signal exit
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(*dir) { exit_pct = pct; closed = true; }
            }
            // End of data
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }
            // Min hold + Z0 exit
            if !closed && i >= *entry_bar + MIN_HOLD_BARS {
                if (*dir == 1 && d.zscore[i] >= 0.0) || (*dir == -1 && d.zscore[i] <= 0.0) {
                    exit_pct = pct; closed = true;
                }
            }

            if closed {
                let net = bal * POSITION_SIZE * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                pos = None;
                cooldown = COOLDOWN;
                vol_dryup_consec = 0;
            }
        } else if cooldown == 0 {
            vol_dryup_consec = 0;
            if let Some(dir) = regime_signal(d.zscore[i]) {
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 {
                        pos = Some((dir, entry_price, i, d.zscore[i]));
                    }
                }
            }
        }
    }

    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let avg_win = POSITION_SIZE * LEVERAGE * SL_PCT * wins as f64;
    let avg_loss = POSITION_SIZE * LEVERAGE * SL_PCT * losses as f64;
    let pf = if losses > 0 { avg_win / avg_loss } else { 0.0 };

    CoinResult { coin: d.name.clone(), pnl, trades: total_trades, wins, losses, flats, wr, pf, dryup_exits, dryup_exit_wins }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN104 — Volume Dry-Up Exit Grid Search\n");
    let mut raw_data: Vec<Option<CoinData>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let data: Vec<CoinData> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins = {} simulations", grid.len(), N_COINS, grid.len() * N_COINS);

    let done = AtomicUsize::new(0);
    let total_sims = grid.len() * N_COINS;

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![], dryup_exit_total: 0, dryup_exit_wins_total: 0, dryup_exit_rate: 0.0, dryup_exit_win_rate: 0.0 };
        }
        let coin_results: Vec<CoinResult> = data.iter().map(|d| simulate(d, cfg)).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_win_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_wins as f64);
        let avg_loss_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_losses as f64);
        let pf = if total_losses > 0 { avg_win_f / avg_loss_f } else { 0.0 };
        let dryup_exit_total: usize = coin_results.iter().map(|c| c.dryup_exits).sum();
        let dryup_exit_wins_total: usize = coin_results.iter().map(|c| c.dryup_exit_wins).sum();
        let dryup_exit_rate = if total_trades > 0 { dryup_exit_total as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let dryup_exit_win_rate = if dryup_exit_total > 0 { dryup_exit_wins_total as f64 / dryup_exit_total as f64 * 100.0 } else { 0.0 };

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  dryup_rate={:.1}%",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, dryup_exit_rate);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, is_baseline: cfg.is_baseline, coins: coin_results, dryup_exit_total, dryup_exit_wins_total, dryup_exit_rate, dryup_exit_win_rate }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN104 Volume Dry-Up Exit Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>6} {:>7} {:>8} {:>10}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "DryExits", "DryupRate");
    println!("{}", "-".repeat(80));
    for (i, r) in sorted.iter().enumerate().take(15) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<22} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8} {:>9.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.dryup_exit_total, r.dryup_exit_rate);
    }
    println!("{}", "=".repeat(80));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });
    if best.dryup_exit_total > 0 {
        println!("Best dryup exit win rate: {:.1}% ({}/{})", best.dryup_exit_win_rate, best.dryup_exit_wins_total, best.dryup_exit_total);
    }

    let notes = format!("RUN104 volume dryup exit. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2}, dryup_rate={:.1}%, dryup_WR={:.1}%)",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl,
        best.dryup_exit_rate, best.dryup_exit_win_rate);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run104_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run104_1_results.json");
}
