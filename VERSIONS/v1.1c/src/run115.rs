/// RUN115 — Supertrend Confirmation: Use Supertrend as Entry Filter and Adaptive Trailing Stop
///
/// Grid: MULT [2.0, 3.0, 4.0] × ENTRY_FILTER [true, false] × TRAIL_STOP [true, false]
/// 12 configs × 18 coins = 216 simulations (parallel per coin)
///
/// Run: cargo run --release --features run115 -- --run115

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
struct SupertrendCfg {
    mult: f64,
    entry_filter: bool,
    trail_stop: bool,
    is_baseline: bool,
}

impl SupertrendCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else {
            let ef = if self.entry_filter { "EF" } else { "NE" };
            let ts = if self.trail_stop { "TS" } else { "NT" };
            format!("M{:.1}_{}_{}", self.mult, ef, ts)
        }
    }
}

fn build_grid() -> Vec<SupertrendCfg> {
    let mut grid = vec![SupertrendCfg { mult: 0.0, entry_filter: false, trail_stop: false, is_baseline: true }];
    for &m in &[2.0, 3.0, 4.0] {
        for &ef in &[true, false] {
            for &ts in &[true, false] {
                grid.push(SupertrendCfg { mult: m, entry_filter: ef, trail_stop: ts, is_baseline: false });
            }
        }
    }
    grid
}

#[derive(Clone)]
struct CoinData {
    name: String,
    opens: Vec<f64>,
    closes: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    zscore: Vec<f64>,
    atr: Vec<f64>,
    st_upper: Vec<f64>,
    st_lower: Vec<f64>,
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
    entries_filtered: usize,
    st_exits: usize,
    st_exit_wins: usize,
    st_exit_losses: usize,
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
    entries_filtered_total: usize,
    filter_rate: f64,
    st_exits_total: usize,
    st_exit_wr: f64,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn compute_atr(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Vec<f64> {
    let n = highs.len();
    let mut atr = vec![f64::NAN; n];
    if n < period + 1 { return atr; }
    let mut tr_sum = 0.0;
    for i in 1..n {
        let tr = highs[i].max(closes[i-1]) - lows[i].min(closes[i-1]);
        tr_sum += tr;
        if i >= period {
            if i > period {
                tr_sum -= highs[i-period].max(closes[i-period-1]) - lows[i-period].min(closes[i-period-1]);
            }
            atr[i] = tr_sum / period as f64;
        }
    }
    atr
}

fn compute_supertrend(highs: &[f64], lows: &[f64], closes: &[f64], atr: &[f64], mult: f64) -> (Vec<f64>, Vec<f64>) {
    let n = closes.len();
    let mut upper = vec![f64::NAN; n];
    let mut lower = vec![f64::NAN; n];
    let mut st_direction = 1i8; // 1 = uptrend, -1 = downtrend
    let mut prev_upper = f64::NAN;
    let mut prev_lower = f64::NAN;

    for i in 0..n {
        if atr[i].is_nan() || i < 2 { continue; }
        let mid = (highs[i] + lows[i]) / 2.0;
        let curr_upper = mid + mult * atr[i];
        let curr_lower = mid - mult * atr[i];

        // Initialize on first valid bar
        if i == 2 || prev_upper.is_nan() {
            prev_upper = curr_upper;
            prev_lower = curr_lower;
        }

        // Final upper/lower (standard supertrend logic)
        upper[i] = if curr_upper < prev_upper || closes[i-1] > prev_upper { curr_upper } else { prev_upper };
        lower[i] = if curr_lower > prev_lower || closes[i-1] < prev_lower { curr_lower } else { prev_lower };

        if st_direction == 1 {
            if closes[i] < lower[i] {
                st_direction = -1;
            }
        } else {
            if closes[i] > upper[i] {
                st_direction = 1;
            }
        }

        prev_upper = upper[i];
        prev_lower = lower[i];
    }
    (upper, lower)
}

fn load_15m(coin: &str, mult: f64) -> Option<CoinData> {
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

    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>()/20.0;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }

    let atr = compute_atr(&highs, &lows, &closes, 14);
    let (st_upper, st_lower) = compute_supertrend(&highs, &lows, &closes, &atr, mult);

    Some(CoinData { name: coin.to_string(), opens, closes, highs, lows, zscore, atr, st_upper, st_lower })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData, cfg: &SupertrendCfg) -> CoinResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize)> = None;
    let mut cooldown = 0usize;

    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut entries_filtered = 0usize;
    let mut st_exits = 0usize; let mut st_exit_wins = 0usize; let mut st_exit_losses = 0usize;

    for i in 20..n {
        cooldown = cooldown.saturating_sub(1);

        if let Some((dir, entry, entry_bar)) = pos.as_mut() {
            let pct = if *dir == 1 { (d.closes[i]-*entry)/ *entry } else { (*entry-d.closes[i])/ *entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            // SL
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            // Regime flip
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(*dir) { exit_pct = pct; closed = true; }
            }
            // End of data
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }
            // Z0 + MIN_HOLD
            if !closed && i >= *entry_bar + MIN_HOLD_BARS {
                if (*dir == 1 && d.zscore[i] >= 0.0) || (*dir == -1 && d.zscore[i] <= 0.0) {
                    exit_pct = pct; closed = true;
                }
            }
            // Supertrend trail stop exit
            if !closed && cfg.trail_stop && !cfg.is_baseline {
                let st_up = d.st_upper[i];
                let st_lo = d.st_lower[i];
                if !st_up.is_nan() && !st_lo.is_nan() {
                    let st_exit = if *dir == 1 {
                        d.closes[i] < st_lo
                    } else {
                        d.closes[i] > st_up
                    };
                    if st_exit {
                        exit_pct = pct;
                        closed = true;
                        st_exits += 1;
                        let net = bal * POSITION_SIZE * LEVERAGE * exit_pct;
                        if net > 1e-10 { st_exit_wins += 1; }
                        else if net < -1e-10 { st_exit_losses += 1; }
                    }
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
                // Supertrend entry filter
                let passes_filter = if cfg.is_baseline || !cfg.entry_filter {
                    true
                } else {
                    let st_up = d.st_upper[i];
                    let st_lo = d.st_lower[i];
                    if !st_up.is_nan() && !st_lo.is_nan() {
                        if dir == 1 { d.closes[i] > st_lo } else { d.closes[i] < st_up }
                    } else { true }
                };

                if !passes_filter {
                    entries_filtered += 1;
                } else {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            pos = Some((dir, entry_price, i));
                        }
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

    CoinResult { coin: d.name.clone(), pnl, trades: total_trades, wins, losses, flats, wr, pf, entries_filtered, st_exits, st_exit_wins, st_exit_losses }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN115 — Supertrend Confirmation Grid Search\n");
    let grid = build_grid();

    // Precompute supertrend data for each mult value
    let mults: Vec<f64> = grid.iter().filter(|c| !c.is_baseline).map(|c| c.mult).collect();
    let unique_mults: Vec<f64> = {
        let mut v = mults.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v.dedup();
        v
    };

    eprintln!("Precomputing Supertrend for multipliers: {:?}", unique_mults);

    #[derive(Clone)]
    struct CoinDataSet {
        name: String,
        data_mult: Vec<CoinData>,
    }

    let mut coin_datasets: Vec<CoinDataSet> = Vec::new();
    for &name in &COIN_NAMES {
        let mut data_mults: Vec<CoinData> = Vec::new();
        for &m in &unique_mults {
            if let Some(d) = load_15m(name, m) {
                data_mults.push(d);
            }
        }
        if data_mults.len() == unique_mults.len() {
            coin_datasets.push(CoinDataSet { name: name.to_string(), data_mult: data_mults });
            eprintln!("  {} — {} bars", name, coin_datasets.last().unwrap().data_mult[0].closes.len());
        }
    }

    if coin_datasets.len() != N_COINS { eprintln!("Missing data for some coins!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }

    let done = AtomicUsize::new(0);
    let total_sims = grid.len() * N_COINS;

    let all_results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![], entries_filtered_total: 0, filter_rate: 0.0, st_exits_total: 0, st_exit_wr: 0.0 };
        }

        let mult_idx = unique_mults.iter().position(|&m| (m - cfg.mult).abs() < 0.01).unwrap_or(0);

        let coin_results: Vec<CoinResult> = coin_datasets.iter().map(|cd| {
            simulate(&cd.data_mult[mult_idx], cfg)
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let total_wins: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_losses: usize = coin_results.iter().map(|c| c.losses).sum();
        let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let avg_win_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_wins as f64);
        let avg_loss_f = POSITION_SIZE * LEVERAGE * SL_PCT * (total_losses as f64);
        let pf = if total_losses > 0 { avg_win_f / avg_loss_f } else { 0.0 };
        let entries_filtered_total: usize = coin_results.iter().map(|c| c.entries_filtered).sum();
        let total_signals = total_trades + entries_filtered_total;
        let filter_rate = if total_signals > 0 { entries_filtered_total as f64 / total_signals as f64 * 100.0 } else { 0.0 };
        let st_exits_total: usize = coin_results.iter().map(|c| c.st_exits).sum();
        let st_exit_wr = if st_exits_total > 0 { coin_results.iter().map(|c| c.st_exit_wins).sum::<usize>() as f64 / st_exits_total as f64 * 100.0 } else { 0.0 };

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  filtered={} ({:.1}%)  st_exits={}",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, entries_filtered_total, filter_rate, st_exits_total);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, is_baseline: cfg.is_baseline, coins: coin_results, entries_filtered_total, filter_rate, st_exits_total, st_exit_wr }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN115 Supertrend Confirmation Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>6} {:>7} {:>8} {:>7} {:>9}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "Filtered", "STExits", "STExitWR%");
    println!("{}", "-".repeat(88));
    for (i, r) in sorted.iter().enumerate().take(15) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<20} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8} {:>7} {:>8.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.entries_filtered_total, r.st_exits_total, r.st_exit_wr);
    }
    println!("{}", "=".repeat(88));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN115 Supertrend. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run115_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run115_1_results.json");
}
