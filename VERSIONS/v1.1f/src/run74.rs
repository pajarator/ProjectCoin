/// RUN74 — Daily Equity Compounding with Per-Coin Reset
///
/// Each coin's balance resets to $100 at daily/weekly/monthly UTC boundaries.
/// Optionally carries accumulated profits across resets.
///
/// Grid: RESET_FREQ [1=daily, 7=weekly, 30=monthly] × CARRY [true, false] + baseline
/// Total: 3 × 2 + 1 = 7 configs
///
/// Run: cargo run --release --features run74 -- --run74

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: usize = 2;
const COOLDOWN: usize = 2;
const BASE_POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Debug)]
struct CompoundCfg {
    reset_freq: u32,    // 0=disabled(baseline), 1=daily, 7=weekly, 30=monthly
    carry_profits: bool,
    is_baseline: bool,
}

impl CompoundCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else {
            let freq = match self.reset_freq {
                1 => "DLY".to_string(),
                7 => "WKLY".to_string(),
                30 => "MTHLY".to_string(),
                _ => format!("F{}", self.reset_freq),
            };
            let carry = if self.carry_profits { "_CARRY" } else { "_NOCARRY" };
            format!("{}{}", freq, carry)
        }
    }
}

fn build_grid() -> Vec<CompoundCfg> {
    let mut grid = vec![CompoundCfg { reset_freq: 0, carry_profits: false, is_baseline: true }];
    for &freq in &[1, 7, 30] {
        for &carry in &[true, false] {
            grid.push(CompoundCfg { reset_freq: freq, carry_profits: carry, is_baseline: false });
        }
    }
    grid
}

struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    zscore: Vec<f64>,
    days: Vec<u32>,   // UTC day number for each bar
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

fn parse_day(ts: &str) -> u32 {
    // Format: "2025-10-15 17:30:00"
    let day_str = &ts[..10]; // "2025-10-15"
    let parts: Vec<u32> = day_str.split('-').filter_map(|s| s.parse().ok()).collect();
    if parts.len() == 3 {
        // Approximate day number: year*10000 + month*100 + day
        (parts[0] * 10000 + parts[1] * 100 + parts[2]) as u32
    } else {
        0
    }
}

fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut closes = Vec::new();
    let mut days = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let _hh: f64 = it.next()?.parse().ok()?;
        let _ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || cc.is_nan() { continue; }
        opens.push(oo);
        closes.push(cc);
        days.push(parse_day(ts));
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
    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore, days })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// Portfolio-level simulation with compounding reset
fn simulate_portfolio(data: &[CoinData15m], cfg: &CompoundCfg) -> (f64, usize, usize, usize, Vec<CoinResult>) {
    let n = data[0].closes.len();
    // Per-coin balances
    let mut bals: Vec<f64> = vec![INITIAL_BAL; N_COINS];
    // Accumulated carried profits (per coin)
    let mut carried: Vec<f64> = vec![0.0; N_COINS];
    // Per-coin last reset day (UTC day number)
    let mut last_reset_day: Vec<u32> = vec![0; N_COINS];
    // Initialize last_reset_day to first day in data
    for ci in 0..N_COINS {
        if !data[ci].days.is_empty() {
            last_reset_day[ci] = data[ci].days[0];
        }
    }
    // Positions: (dir, entry_price)
    let mut pos: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldown: Vec<usize> = vec![0; N_COINS];
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    // Per-coin stats
    let mut coin_wins: Vec<usize> = vec![0; N_COINS];
    let mut coin_losses: Vec<usize> = vec![0; N_COINS];
    let mut coin_flats: Vec<usize> = vec![0; N_COINS];
    let mut coin_final_pnl: Vec<f64> = vec![0.0; N_COINS];

    for i in 1..n {
        let current_day = data[0].days[i];

        // Process each coin
        for ci in 0..N_COINS {
            let d = &data[ci];
            let bal = &mut bals[ci];
            let carried_profits = &mut carried[ci];
            let last_day = &mut last_reset_day[ci];

            // Check compound reset
            if cfg.reset_freq > 0 && *last_day > 0 {
                let days_elapsed = if current_day >= *last_day {
                    current_day - *last_day
                } else {
                    continue; // day went backwards (data gap), skip
                };

                if days_elapsed >= cfg.reset_freq {
                    // Reset triggered: lock in position PnL
                    if let Some((dir, entry)) = pos[ci] {
                        let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
                        let net = (*bal * BASE_POSITION_SIZE * LEVERAGE) * pct;
                        *bal += net;
                        if net > 1e-10 { coin_wins[ci] += 1; wins += 1; }
                        else if net < -1e-10 { coin_losses[ci] += 1; losses += 1; }
                        else { coin_flats[ci] += 1; flats += 1; }
                        pos[ci] = None;
                    }
                    // Carry profits if enabled
                    if !cfg.carry_profits {
                        *carried_profits = 0.0;
                    }
                    // Reset balance to initial
                    *bal = INITIAL_BAL;
                    *last_day = current_day;
                    cooldown[ci] = 0;
                }
            }

            // Process exits
            if let Some((dir, entry)) = pos[ci] {
                let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
                let mut closed = false;
                let mut exit_pct = 0.0;

                if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }

                if !closed {
                    let new_dir = regime_signal(d.zscore[i]);
                    if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
                }

                // Force close on last bar
                if !closed && i >= n - 1 { exit_pct = pct; closed = true; }

                if closed {
                    let net = (*bal * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
                    *bal += net;
                    if net > 1e-10 { coin_wins[ci] += 1; wins += 1; }
                    else if net < -1e-10 { coin_losses[ci] += 1; losses += 1; }
                    else { coin_flats[ci] += 1; flats += 1; }
                    pos[ci] = None;
                    cooldown[ci] = COOLDOWN;
                }
            } else if cooldown[ci] > 0 {
                cooldown[ci] -= 1;
            } else {
                // Process entries
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            pos[ci] = Some((dir, entry_price));
                        }
                    }
                }
            }
        }
    }

    // Final PnL calculation (including carried profits)
    let mut total_pnl = 0.0;
    for ci in 0..N_COINS {
        let final_bal = bals[ci] + carried[ci];
        coin_final_pnl[ci] = final_bal - INITIAL_BAL;
        total_pnl += coin_final_pnl[ci];
    }

    let coin_results: Vec<CoinResult> = (0..N_COINS).map(|ci| {
        let trades = coin_wins[ci] + coin_losses[ci] + coin_flats[ci];
        let wr = if trades > 0 { coin_wins[ci] as f64 / trades as f64 * 100.0 } else { 0.0 };
        CoinResult {
            coin: data[ci].name.clone(),
            pnl: coin_final_pnl[ci],
            trades,
            wins: coin_wins[ci],
            losses: coin_losses[ci],
            wr,
        }
    }).collect();

    (total_pnl, wins, losses, flats, coin_results)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN74 — Daily Equity Compounding with Per-Coin Reset\n");
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
    eprintln!("\nGrid: {} configs × portfolio", grid.len());

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![],
            };
        }
        let (total_pnl, wins, losses, flats, coin_results) = simulate_portfolio(&coin_data, cfg);
        let total_trades = wins + losses + flats;
        let portfolio_wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins as f64 * SL_PCT * BASE_POSITION_SIZE * LEVERAGE;
        let losses_f = losses as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades,
            pf, is_baseline: cfg.is_baseline, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN74 Daily Compounding Reset Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<18} {:>8} {:>8} {:>6} {:>7} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(65));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<18} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("{}", "=".repeat(65));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN74 daily compounding reset. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run74_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run74_1_results.json");
}