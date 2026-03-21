/// RUN103 — Stochastic Extreme Exit: Exit Regime Trades When Stochastic Reaches Extreme Levels
///
/// Grid: STOCH_OB [75.0, 80.0, 85.0] × STOCH_OS [15.0, 20.0, 25.0] × MIN_HOLD [3, 5, 8]
/// 27 configs × 18 coins = 486 simulations (parallel per coin)
///
/// Run: cargo run --release --features run103 -- --run103

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
struct StochExtCfg {
    ob_thresh: f64,
    os_thresh: f64,
    min_hold: usize,
    is_baseline: bool,
}

impl StochExtCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("OB{:.0}_OS{:.0}_MH{}", self.ob_thresh, self.os_thresh, self.min_hold) }
    }
}

fn build_grid() -> Vec<StochExtCfg> {
    let mut grid = vec![StochExtCfg { ob_thresh: 80.0, os_thresh: 20.0, min_hold: 0, is_baseline: true }];
    for &ob in &[75.0, 80.0, 85.0] {
        for &os in &[15.0, 20.0, 25.0] {
            for &mh in &[3usize, 5, 8] {
                grid.push(StochExtCfg { ob_thresh: ob, os_thresh: os, min_hold: mh, is_baseline: false });
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
    stoch_k: Vec<f64>,
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
    stoch_exits: usize,
    stoch_exit_wins: usize,
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
    stoch_exit_total: usize,
    stoch_exit_wins_total: usize,
    stoch_exit_rate: f64,
    stoch_exit_win_rate: f64,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
    let mut highs = Vec::new(); let mut lows = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); highs.push(hh); lows.push(ll); closes.push(cc);
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

    // Stochastic %K (14-bar)
    let stoch_period = 14;
    let mut stoch_k = vec![f64::NAN; n];
    for i in stoch_period..n {
        let mut lowest = f64::MAX;
        let mut highest = f64::MIN;
        for j in (i+1-stoch_period)..=i {
            if highs[j] < lowest { lowest = highs[j]; }
            if highs[j] > highest { highest = highs[j]; }
        }
        let range = highest - lowest;
        stoch_k[i] = if range > 0.0 { 100.0 * (closes[i] - lowest) / range } else { 50.0 };
    }

    Some(CoinData { name: coin.to_string(), opens, closes, highs, lows, zscore, stoch_k })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData, cfg: &StochExtCfg) -> CoinResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None; // dir, avg_entry, entry_bar, z_at_entry
    let mut cooldown = 0usize;

    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut stoch_exits = 0usize;
    let mut stoch_exit_wins = 0usize;

    for i in 20..n {
        if cooldown > 0 { cooldown -= 1; }

        if let Some((dir, entry, entry_bar, _z_entry)) = pos.as_mut() {
            let pct = if *dir == 1 { (d.closes[i]-*entry)/ *entry } else { (*entry-d.closes[i])/ *entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            // SL
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }

            // Stochastic extreme exit (only for non-baseline configs and when profitable)
            if !closed && !cfg.is_baseline {
                let held_bars = i.saturating_sub(*entry_bar);
                if held_bars >= cfg.min_hold && pct > 0.0 {
                    let sk = d.stoch_k[i];
                    if !sk.is_nan() {
                        let fire = if *dir == 1 && sk >= cfg.ob_thresh {
                            true
                        } else if *dir == -1 && sk <= cfg.os_thresh {
                            true
                        } else {
                            false
                        };
                        if fire {
                            exit_pct = pct;
                            closed = true;
                            stoch_exits += 1;
                            if pct > 0.0 { stoch_exit_wins += 1; }
                        }
                    }
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
            }
        } else if cooldown == 0 {
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

    CoinResult { coin: d.name.clone(), pnl, trades: total_trades, wins, losses, flats, wr, pf, stoch_exits, stoch_exit_wins }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN103 — Stochastic Extreme Exit Grid Search\n");
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
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![], stoch_exit_total: 0, stoch_exit_wins_total: 0, stoch_exit_rate: 0.0, stoch_exit_win_rate: 0.0 };
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
        let stoch_exit_total: usize = coin_results.iter().map(|c| c.stoch_exits).sum();
        let stoch_exit_wins_total: usize = coin_results.iter().map(|c| c.stoch_exit_wins).sum();
        let stoch_exit_rate = if total_trades > 0 { stoch_exit_total as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let stoch_exit_win_rate = if stoch_exit_total > 0 { stoch_exit_wins_total as f64 / stoch_exit_total as f64 * 100.0 } else { 0.0 };

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  stoch_exit_rate={:.1}%",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, stoch_exit_rate);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, is_baseline: cfg.is_baseline, coins: coin_results, stoch_exit_total, stoch_exit_wins_total, stoch_exit_rate, stoch_exit_win_rate }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN103 Stochastic Extreme Exit Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>6} {:>7} {:>8} {:>10}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "StochExits", "StochExitRate");
    println!("{}", "-".repeat(80));
    for (i, r) in sorted.iter().enumerate().take(15) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<22} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8} {:>9.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.stoch_exit_total, r.stoch_exit_rate);
    }
    println!("{}", "=".repeat(80));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });
    if best.stoch_exit_total > 0 {
        println!("Best stoch exit win rate: {:.1}% ({}/{})", best.stoch_exit_win_rate, best.stoch_exit_wins_total, best.stoch_exit_total);
    }

    let notes = format!("RUN103 stochastic extreme exit. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2}, stoch_exit_rate={:.1}%, stoch_exit_WR={:.1}%)",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl,
        best.stoch_exit_rate, best.stoch_exit_win_rate);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run103_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run103_1_results.json");
}
