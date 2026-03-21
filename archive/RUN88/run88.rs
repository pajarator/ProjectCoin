/// RUN88 — Trailing Z-Score Exit
///
/// Exit when z-score has recovered Z_RECOVERY_PCT of the way back to 0:
/// - Enter at z = -2.0, exit at z = -0.7 when Z_RECOVERY_PCT = 0.65
/// - Min hold requirement before recovery exit can fire
///
/// Grid: Z_RECOVERY_PCT [0.50, 0.60, 0.65, 0.70, 0.75] × MIN_HOLD [4, 8, 12]
/// Total: 5 × 3 = 15 + baseline = 16 configs per coin
///
/// Run: cargo run --release --features run88 -- --run88

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
struct TzCfg {
    recovery_pct: f64,
    min_hold: usize,
    is_baseline: bool,
}

impl TzCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("RP{:.2}_MH{}", self.recovery_pct, self.min_hold) }
    }
}

fn build_grid() -> Vec<TzCfg> {
    let mut grid = vec![TzCfg { recovery_pct: 0.0, min_hold: 0, is_baseline: true }];
    let rps = [0.50, 0.60, 0.65, 0.70, 0.75];
    let mhs = [4usize, 8, 12];
    for &rp in &rps {
        for &mh in &mhs {
            grid.push(TzCfg { recovery_pct: rp, min_hold: mh, is_baseline: false });
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
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, z_recovery_exits: usize }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, z_recovery_exit_rate: f64,
    coins: Vec<CoinResult>,
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

fn simulate(d: &CoinData15m, cfg: &TzCfg) -> (f64, usize, usize, usize, usize, f64) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None; // dir, entry_price, entry_bar, z_at_entry
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut z_recovery_exits = 0usize;

    for i in 1..n {
        if let Some((dir, entry, entry_bar, z_entry)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;
            let bars_held = i - entry_bar;

            // Z-recovery exit: z has recovered recovery_pct toward 0
            if !closed && !cfg.is_baseline && cfg.recovery_pct > 0.0 && bars_held >= cfg.min_hold {
                let z_target = z_entry * (1.0 - cfg.recovery_pct);
                let z_curr = d.zscore[i];
                if !z_curr.is_nan() {
                    let recovered = if dir == 1 {
                        // LONG: entered negative, want z to increase toward 0
                        z_curr >= z_target
                    } else {
                        // SHORT: entered positive, want z to decrease toward 0
                        z_curr <= z_target
                    };
                    if recovered {
                        exit_pct = pct;
                        closed = true;
                        z_recovery_exits += 1;
                    }
                }
            }

            // SL check
            if !closed && pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }

            // Z0 exit (crossback to 0)
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }

            // End of data
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
                if i + 1 < n {
                    let entry_price = d.opens[i + 1];
                    if entry_price > 0.0 {
                        let z_entry = d.zscore[i];
                        pos = Some((dir, entry_price, i, z_entry));
                    }
                }
            }
        }
    }

    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let z_recovery_rate = if total_trades > 0 { z_recovery_exits as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    (pnl, total_trades, wins, losses, z_recovery_exits, z_recovery_rate)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN88 — Trailing Z-Score Exit\n");
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
    let total_cfgs = grid.len();
    let total_sims = grid.len() * N_COINS;

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, z_recovery_exit_rate: 0.0, coins: vec![]
            };
        }
        let coin_results: Vec<CoinResult> = (0..N_COINS).map(|c| {
            let (pnl, trades, wins, losses, z_rec, z_rec_rate) = simulate(&coin_data[c], cfg);
            CoinResult {
                coin: coin_data[c].name.clone(),
                pnl,
                trades,
                wins,
                losses,
                wr: if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 },
                z_recovery_exits: z_rec,
            }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = total_wins as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
        let losses_f = total_losses as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };
        let total_z_rec: usize = coin_results.iter().map(|c| c.z_recovery_exits).sum();
        let z_recovery_exit_rate = if total_trades > 0 { total_z_rec as f64 / total_trades as f64 * 100.0 } else { 0.0 };

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  ZRec%={:>5.1}%",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, z_recovery_exit_rate);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf,
            is_baseline: cfg.is_baseline, z_recovery_exit_rate: z_recovery_exit_rate,
            coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN88 Trailing Z-Score Exit Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}  ZRec%={:.1}%",
        baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades, baseline.z_recovery_exit_rate);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>6} {:>7} {:>8} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF", "ZRec%");
    println!("{}", "-".repeat(80));
    for (i, r) in sorted.iter().enumerate().take(20) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<20} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2} {:>7.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf, r.z_recovery_exit_rate);
    }
    println!("{}", "=".repeat(80));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN88 trailing z-exit. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2}, ZRec%={:.1}%)",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl, best.z_recovery_exit_rate);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run88_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run88_1_results.json");
}
