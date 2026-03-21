/// RUN93 — Consecutive Wins Streak Boost
///
/// Scale position size after consecutive win/loss streaks:
/// - After STREAK_WIN_THRESHOLD wins: increase RISK by STREAK_RISK_MULT
/// - After STREAK_LOSS_THRESHOLD losses: decrease RISK by STREAK_LOSS_MULT
///
/// Grid: WIN_THR [2, 3, 4] × LOSS_THR [2, 3, 4] × WIN_MULT [1.2, 1.3, 1.5] × LOSS_MULT [0.6, 0.7, 0.8]
/// Total: 3^4 = 81 + baseline = 82 configs
///
/// Run: cargo run --release --features run93 -- --run93

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const COOLDOWN: usize = 2;
const BASE_POSITION_SIZE: f64 = 0.02;  // 2% base
const LEVERAGE: f64 = 5.0;
const RISK_MIN: f64 = 0.005;
const RISK_MAX: f64 = 0.15;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct StreakCfg {
    win_thr: usize,
    loss_thr: usize,
    win_mult: f64,
    loss_mult: f64,
    is_baseline: bool,
}

impl StreakCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("WT{}_LT{}_WM{:.1}_LM{:.1}", self.win_thr, self.loss_thr, self.win_mult, self.loss_mult) }
    }
}

fn build_grid() -> Vec<StreakCfg> {
    let mut grid = vec![StreakCfg { win_thr: 0, loss_thr: 0, win_mult: 1.0, loss_mult: 1.0, is_baseline: true }];
    let wts = [2usize, 3, 4];
    let lts = [2usize, 3, 4];
    let wms = [1.2, 1.3, 1.5];
    let lms = [0.6, 0.7, 0.8];
    for &wt in &wts {
        for &lt in &lts {
            for &wm in &wms {
                for &lm in &lms {
                    grid.push(StreakCfg { win_thr: wt, loss_thr: lt, win_mult: wm, loss_mult: lm, is_baseline: false });
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
    zscore: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64 }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new(); let mut closes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ','); let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); closes.push(cc);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>()/20.0;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }
    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData15m, cfg: &StreakCfg) -> (f64, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut consec_wins: usize = 0;
    let mut consec_losses: usize = 0;

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
                let effective_risk = if consec_wins >= cfg.win_thr {
                    (BASE_POSITION_SIZE * cfg.win_mult).clamp(RISK_MIN, RISK_MAX)
                } else if consec_losses >= cfg.loss_thr {
                    (BASE_POSITION_SIZE * cfg.loss_mult).clamp(RISK_MIN, RISK_MAX)
                } else {
                    BASE_POSITION_SIZE
                };
                let net = bal * effective_risk * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; consec_wins += 1; consec_losses = 0; }
                else if net < -1e-10 { losses += 1; consec_losses += 1; consec_wins = 0; }
                else { flats += 1; consec_wins = 0; consec_losses = 0; }
                pos = None;
                cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d.zscore[i]) {
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 { pos = Some((dir, entry_price)); }
                }
            }
        }
    }

    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    (pnl, total_trades, wins, losses)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN93 — Consecutive Wins Streak Boost\n");
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
    eprintln!("\nGrid: {} configs × {} coins = {} simulations", grid.len(), N_COINS, grid.len() * N_COINS);

    let done = AtomicUsize::new(0);
    let total_sims = grid.len() * N_COINS;

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![]
            };
        }
        let coin_results: Vec<CoinResult> = (0..N_COINS).map(|c| {
            let (pnl, trades, wins, losses) = simulate(&coin_data[c], cfg);
            CoinResult {
                coin: coin_data[c].name.clone(),
                pnl,
                trades,
                wins,
                losses,
                wr: if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 },
            }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = total_wins as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE;
        let losses_f = total_losses as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf,
            is_baseline: cfg.is_baseline, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN93 Consecutive Wins Streak Boost Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<30} {:>8} {:>8} {:>6} {:>7} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(75));
    for (i, r) in sorted.iter().enumerate().take(20) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<30} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("{}", "=".repeat(75));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN93 streak boost. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run93_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run93_1_results.json");
}
