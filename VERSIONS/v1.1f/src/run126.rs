/// RUN126 — Ichimoku Cloud Confirmation: Multi-Component Entry Filter
///
/// Grid: CLOUD [true, false] × CROSS [true, false] × CHIKOU [true, false]
/// 8 configs × 18 coins = 144 simulations (parallel per coin)
///
/// Run: cargo run --release --features run126 -- --run126

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
struct IchCfg {
    cloud_confirm: bool,
    cross_confirm: bool,
    chikou_confirm: bool,
    is_baseline: bool,
}

impl IchCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else {
            let c = if self.cloud_confirm { "C1" } else { "C0" };
            let x = if self.cross_confirm { "X1" } else { "X0" };
            let k = if self.chikou_confirm { "K1" } else { "K0" };
            format!("{}_{}_{}", c, x, k)
        }
    }
}

fn build_grid() -> Vec<IchCfg> {
    let mut grid = vec![IchCfg { cloud_confirm: false, cross_confirm: false, chikou_confirm: false, is_baseline: true }];
    for &cc in &[true, false] {
        for &xc in &[true, false] {
            for &kc in &[true, false] {
                grid.push(IchCfg { cloud_confirm: cc, cross_confirm: xc, chikou_confirm: kc, is_baseline: false });
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
    tenkan: Vec<f64>,
    kijun: Vec<f64>,
    senkou_a: Vec<f64>,
    senkou_b: Vec<f64>,
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
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn highest_lowest(data: &[f64], start: usize, end: usize) -> (f64, f64) {
    let mut high = f64::MIN;
    let mut low = f64::MAX;
    for i in start..=end {
        if data[i] > high { high = data[i]; }
        if data[i] < low { low = data[i]; }
    }
    (high, low)
}

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

    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>()/20.0;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }

    let tenkan_period = 9;
    let kijun_period = 26;
    let mut tenkan = vec![f64::NAN; n];
    let mut kijun = vec![f64::NAN; n];
    let mut senkou_a = vec![f64::NAN; n];
    let mut senkou_b = vec![f64::NAN; n];

    for i in kijun_period..n {
        // Tenkan = (highest high + lowest low) / 2 over period 9
        let t_start = i.saturating_sub(tenkan_period - 1);
        let (th, tl) = highest_lowest(&highs, t_start, i);
        tenkan[i] = (th + tl) / 2.0;

        // Kijun = (highest high + lowest low) / 2 over period 26
        let k_start = i.saturating_sub(kijun_period - 1);
        let (kh, kl) = highest_lowest(&highs, k_start, i);
        kijun[i] = (kh + kl) / 2.0;

        // Senkou Span A = (Tenkan + Kijun) / 2, projected forward KIJUN_PERIOD bars
        senkou_a[i] = (tenkan[i] + kijun[i]) / 2.0;

        // Senkou Span B = (highest high + lowest low) over (tenkan+kijun)/2 period, projected forward
        let sb_period = (tenkan_period + kijun_period) / 2;
        let sb_start = i.saturating_sub(sb_period - 1);
        let (sbh, sbl) = highest_lowest(&highs, sb_start, i);
        senkou_b[i] = (sbh + sbl) / 2.0;
    }

    Some(CoinData { name: coin.to_string(), opens, closes, highs, lows, zscore, tenkan, kijun, senkou_a, senkou_b })
}

fn simulate(d: &CoinData, cfg: &IchCfg) -> CoinResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize)> = None;
    let mut cooldown = 0usize;

    let mut wins = 0usize; let mut losses = 0usize; let mut flats = 0usize;
    let mut entries_filtered = 0usize;

    let kijun_period = 26;

    for i in kijun_period..n {
        cooldown = cooldown.saturating_sub(1);

        if let Some((dir, entry, entry_bar)) = pos.as_mut() {
            let pct = if *dir == 1 { (d.closes[i]-*entry)/ *entry } else { (*entry-d.closes[i])/ *entry };
            let mut closed = false;
            let mut exit_pct = 0.0;

            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(*dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; }
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
                if cfg.is_baseline {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            pos = Some((dir, entry_price, i));
                        }
                    }
                } else {
                    let price = d.closes[i];
                    let tenkan = d.tenkan[i];
                    let kijun = d.kijun[i];
                    let sa = d.senkou_a[i];
                    let sb = d.senkou_b[i];
                    let passes_filter = if dir == 1 {
                        // LONG: price below cloud + Tenkan > Kijun + Chikou above prior price
                        let mut ok = true;
                        // Cloud: price must be below both cloud boundaries
                        if cfg.cloud_confirm && (!sa.is_nan() && !sb.is_nan()) {
                            let cloud_lower = sa.min(sb);
                            if price >= cloud_lower { ok = false; }
                        }
                        // Tenkan/Kijun cross: Tenkan must be above Kijun
                        if ok && cfg.cross_confirm && (!tenkan.is_nan() && !kijun.is_nan()) {
                            if tenkan <= kijun { ok = false; }
                        }
                        // Chikou: current close > close KIJUN bars ago
                        if ok && cfg.chikou_confirm && i >= kijun_period {
                            let prior_price = d.closes[i - kijun_period];
                            if price <= prior_price { ok = false; }
                        }
                        ok
                    } else {
                        // SHORT: price above cloud + Tenkan < Kijun + Chikou below prior price
                        let mut ok = true;
                        if cfg.cloud_confirm && (!sa.is_nan() && !sb.is_nan()) {
                            let cloud_upper = sa.max(sb);
                            if price <= cloud_upper { ok = false; }
                        }
                        if ok && cfg.cross_confirm && (!tenkan.is_nan() && !kijun.is_nan()) {
                            if tenkan >= kijun { ok = false; }
                        }
                        if ok && cfg.chikou_confirm && i >= kijun_period {
                            let prior_price = d.closes[i - kijun_period];
                            if price >= prior_price { ok = false; }
                        }
                        ok
                    };

                    if passes_filter {
                        if i + 1 < n {
                            let entry_price = d.opens[i + 1];
                            if entry_price > 0.0 {
                                pos = Some((dir, entry_price, i));
                            }
                        }
                    } else {
                        entries_filtered += 1;
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

    CoinResult { coin: d.name.clone(), pnl, trades: total_trades, wins, losses, flats, wr, pf, entries_filtered }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN126 — Ichimoku Cloud Confirmation Grid Search\n");
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
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, wins: 0, losses: 0, pf: 0.0, is_baseline: cfg.is_baseline, coins: vec![], entries_filtered_total: 0, filter_rate: 0.0 };
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
        let entries_filtered_total: usize = coin_results.iter().map(|c| c.entries_filtered).sum();
        let total_signals = total_trades + entries_filtered_total;
        let filter_rate = if total_signals > 0 { entries_filtered_total as f64 / total_signals as f64 * 100.0 } else { 0.0 };

        let d = done.fetch_add(N_COINS, Ordering::SeqCst) + N_COINS;
        eprintln!("  [{:>4}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  filtered={} ({:.1}%)",
            d, total_sims, cfg.label(), total_pnl, portfolio_wr, total_trades, entries_filtered_total, filter_rate);

        ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, wins: total_wins, losses: total_losses, pf, is_baseline: cfg.is_baseline, coins: coin_results, entries_filtered_total, filter_rate }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = all_results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = all_results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN126 Ichimoku Cloud Confirmation Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>6} {:>7} {:>8} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "Filtered", "FilterRate");
    println!("{}", "-".repeat(78));
    for (i, r) in sorted.iter().enumerate().take(15) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<20} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8} {:>7.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.entries_filtered_total, r.filter_rate);
    }
    println!("{}", "=".repeat(78));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN126 Ichimoku filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        all_results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: all_results };
    std::fs::write("/home/scamarena/ProjectCoin/run126_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run126_1_results.json");
}
