/// RUN89 — Market-Wide ADX Confirmation
///
/// When market-wide average ADX is high, suppress regime entries:
/// - avg_adx >= SUPPRESS_ALL: block ALL regime entries
/// - avg_adx >= SUPPRESS_LONG: block LONG entries (ISO_SHORT allowed)
///
/// Grid: SUPPRESS_LONG [20, 22, 25, 28] × SUPPRESS_ALL [26, 29, 32, 35]
/// Total: 4 × 4 = 16 + baseline = 17 configs
///
/// Run: cargo run --release --features run89 -- --run89

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

// Standard breadth thresholds from COINCLAW v16
const BREADTH_MAX_LONG: f64 = 0.20;
const BREADTH_MAX_ISO: f64 = 0.20;
const BREADTH_MIN_SHORT: f64 = 0.50;

#[derive(Clone, Debug)]
struct AdxCfg {
    suppress_long: f64,
    suppress_all: f64,
    is_baseline: bool,
}

impl AdxCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("SL{:.0}_SA{:.0}", self.suppress_long, self.suppress_all) }
    }
}

fn build_grid() -> Vec<AdxCfg> {
    let mut grid = vec![AdxCfg { suppress_long: 0.0, suppress_all: 999.0, is_baseline: true }];
    let sl_vals = [20.0, 22.0, 25.0, 28.0];
    let sa_vals = [26.0, 29.0, 32.0, 35.0];
    for &sl in &sl_vals {
        for &sa in &sa_vals {
            if sa > sl {
                grid.push(AdxCfg { suppress_long: sl, suppress_all: sa, is_baseline: false });
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
    zscore: Vec<f64>,
    adx: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, adx_suppressed: usize }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, adx_suppression_rate: f64,
    long_entries: usize, iso_entries: usize, short_entries: usize,
    coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_15m(coin: &str) -> Option<CoinData15m> {
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
        opens.push(oo); closes.push(cc); highs.push(hh); lows.push(ll);
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

    // Compute ADX(14) with Wilder smoothing
    let period = 14usize;
    let mut tr = vec![0.0; n];
    let mut dm_plus = vec![0.0; n];
    let mut dm_minus = vec![0.0; n];

    for i in 1..n {
        tr[i] = (highs[i] - lows[i]).max((closes[i-1] - highs[i]).abs()).max((closes[i-1] - lows[i]).abs());
        let up_move = highs[i] - highs[i-1];
        let down_move = lows[i-1] - lows[i];
        if up_move > down_move && up_move > 0.0 { dm_plus[i] = up_move; dm_minus[i] = 0.0; }
        else if down_move > up_move && down_move > 0.0 { dm_plus[i] = 0.0; dm_minus[i] = down_move; }
        else { dm_plus[i] = 0.0; dm_minus[i] = 0.0; }
    }

    let mut adx = vec![f64::NAN; n];
    let mut dx = vec![f64::NAN; n];
    let mut smooth_tr = 0.0;
    let mut smooth_dm_plus = 0.0;
    let mut smooth_dm_minus = 0.0;
    for i in 1..=period {
        smooth_tr += tr[i];
        smooth_dm_plus += dm_plus[i];
        smooth_dm_minus += dm_minus[i];
    }

    if smooth_tr > 0.0 {
        let di_plus = (smooth_dm_plus / smooth_tr) * 100.0;
        let di_minus = (smooth_dm_minus / smooth_tr) * 100.0;
        let di_sum = di_plus + di_minus;
        dx[period] = if di_sum > 0.0 { (di_plus - di_minus).abs() / di_sum * 100.0 } else { 0.0 };
        adx[period] = dx[period];
    }

    for i in (period+1)..n {
        smooth_tr = smooth_tr - smooth_tr / period as f64 + tr[i];
        smooth_dm_plus = smooth_dm_plus - smooth_dm_plus / period as f64 + dm_plus[i];
        smooth_dm_minus = smooth_dm_minus - smooth_dm_minus / period as f64 + dm_minus[i];
        let di_plus = if smooth_tr > 0.0 { (smooth_dm_plus / smooth_tr) * 100.0 } else { 0.0 };
        let di_minus = if smooth_tr > 0.0 { (smooth_dm_minus / smooth_tr) * 100.0 } else { 0.0 };
        let di_sum = di_plus + di_minus;
        dx[i] = if di_sum > 0.0 { (di_plus - di_minus).abs() / di_sum * 100.0 } else { 0.0 };
        adx[i] = adx[i-1] - adx[i-1] / period as f64 + dx[i];
    }

    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, zscore, adx })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn compute_breadth(data: &[&CoinData15m]) -> Vec<f64> {
    let n = data[0].closes.len();
    let mut breadth = vec![f64::NAN; n];
    for i in 20..n {
        let mut count = 0usize;
        let mut valid = 0usize;
        for d in data {
            if !d.zscore[i].is_nan() {
                valid += 1;
                if d.zscore[i] < -1.5 { count += 1; }
            }
        }
        breadth[i] = if valid > 0 { count as f64 / valid as f64 } else { 0.0 };
    }
    breadth
}

fn simulate(cfg: &AdxCfg, coin_data: &[CoinData15m], breadth: &[f64]) -> ConfigResult {
    let n = coin_data[0].closes.len();
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldowns = vec![0usize; N_COINS];
    let mut wins = vec![0usize; N_COINS];
    let mut losses = vec![0usize; N_COINS];
    let mut flats = vec![0usize; N_COINS];
    let mut adx_suppressed = vec![0usize; N_COINS];

    let mut long_entries = 0usize;
    let mut iso_entries = 0usize;
    let mut short_entries = 0usize;
    let mut total_adx_suppressed = 0usize;

    for i in 1..n {
        // Compute average ADX across all coins
        let mut adx_sum = 0.0;
        let mut adx_count = 0usize;
        for d in coin_data {
            if !d.adx[i].is_nan() {
                adx_sum += d.adx[i];
                adx_count += 1;
            }
        }
        let avg_adx = if adx_count > 0 { adx_sum / adx_count as f64 } else { 0.0 };

        // Market mode from breadth
        let mode = if breadth[i].is_nan() {
            1 // default ISO_SHORT
        } else if breadth[i] <= BREADTH_MAX_LONG {
            0 // LONG
        } else if breadth[i] > BREADTH_MIN_SHORT {
            2 // SHORT
        } else {
            1 // ISO_SHORT
        };

        // ADX suppression check (only for non-baseline configs)
        let suppress_all = !cfg.is_baseline && cfg.suppress_all < 999.0 && avg_adx >= cfg.suppress_all;
        let suppress_long = !cfg.is_baseline && cfg.suppress_long > 0.0 && avg_adx >= cfg.suppress_long;

        for c in 0..N_COINS {
            let d = &coin_data[c];

            if let Some((dir, entry)) = positions[c] {
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
                    let net = balances[c] * POSITION_SIZE * LEVERAGE * exit_pct;
                    balances[c] += net;
                    if net > 1e-10 { wins[c] += 1; }
                    else if net < -1e-10 { losses[c] += 1; }
                    else { flats[c] += 1; }
                    positions[c] = None;
                    cooldowns[c] = COOLDOWN;
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
            } else {
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    // Check ADX suppression
                    let suppressed = match (mode, dir) {
                        (0, 1) if suppress_all || suppress_long => { total_adx_suppressed += 1; adx_suppressed[c] += 1; true },
                        _ => false,
                    };
                    if !suppressed && i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 {
                            positions[c] = Some((dir, entry_price));
                            if mode == 0 { long_entries += 1; }
                            else if mode == 1 { iso_entries += 1; }
                            else { short_entries += 1; }
                        }
                    }
                }
            }
        }
    }

    let total_pnl: f64 = balances.iter().map(|&b| b - INITIAL_BAL).sum();
    let total_wins: usize = wins.iter().sum();
    let total_losses: usize = losses.iter().sum();
    let total_flats: usize = flats.iter().sum();
    let total_trades = total_wins + total_losses + total_flats;
    let portfolio_wr = if total_trades > 0 { total_wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let gross = total_wins as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
    let losses_f = total_losses as f64;
    let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };
    let adx_suppression_rate = if (total_trades + total_adx_suppressed) > 0 {
        total_adx_suppressed as f64 / (total_trades + total_adx_suppressed) as f64 * 100.0
    } else { 0.0 };

    let coins: Vec<CoinResult> = (0..N_COINS).map(|c| {
        let pnl = balances[c] - INITIAL_BAL;
        let trades = wins[c] + losses[c] + flats[c];
        let wr = if trades > 0 { wins[c] as f64 / trades as f64 * 100.0 } else { 0.0 };
        CoinResult { coin: coin_data[c].name.clone(), pnl, trades, wins: wins[c], losses: losses[c], wr, adx_suppressed: adx_suppressed[c] }
    }).collect();

    ConfigResult {
        label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf,
        is_baseline: cfg.is_baseline, adx_suppression_rate,
        long_entries, iso_entries, short_entries, coins,
    }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN89 — Market-Wide ADX Confirmation\n");
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
    let coin_refs: Vec<&CoinData15m> = coin_data.iter().collect();

    let breadth = compute_breadth(&coin_refs);

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins (portfolio-level)", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, adx_suppression_rate: 0.0,
                long_entries: 0, iso_entries: 0, short_entries: 0, coins: vec![]
            };
        }
        let result = simulate(cfg, &coin_data, &breadth);
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  ADXsupp={:>5.1}%",
            d, total_cfgs, result.label, result.total_pnl, result.portfolio_wr,
            result.total_trades, result.adx_suppression_rate);
        result
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN89 Market-Wide ADX Confirmation Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}  ADXsupp={:.1}%",
        baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades, baseline.adx_suppression_rate);
    println!("\n{:>3}  {:<18} {:>8} {:>8} {:>6} {:>7} {:>8} {:>10}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF", "ADXsupp%");
    println!("{}", "-".repeat(85));
    for (i, r) in sorted.iter().enumerate().take(20) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<18} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2} {:>9.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf, r.adx_suppression_rate);
    }
    println!("{}", "=".repeat(85));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN89 market ADX. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2}, ADXsupp={:.1}%)",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl, best.adx_suppression_rate);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run89_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run89_1_results.json");
}
