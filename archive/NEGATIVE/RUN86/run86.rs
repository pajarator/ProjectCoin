/// RUN86 — Coin Correlation Clustering
///
/// When 3+ coins in a correlation cluster signal simultaneously, only allow top Sharpe.
/// Suppresses lower-quality signals to reduce correlated drawdown concentration.
///
/// Grid: THRESHOLD [0.60, 0.70, 0.80] × MIN_SIZE [3, 4] × MAX_COINS [1, 2] × COOLDOWN [4, 8, 12]
/// Total: 3 × 2 × 2 × 3 = 36 + baseline = 37 configs
///
/// Run: cargo run --release --features run86 -- --run86

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
struct CorrCfg {
    threshold: f64,
    min_size: usize,
    max_coins: usize,
    suppress_cd: usize,
    is_baseline: bool,
}

impl CorrCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else { format!("T{:.2}_MS{}_MC{}_CD{}", self.threshold, self.min_size, self.max_coins, self.suppress_cd) }
    }
}

fn build_grid() -> Vec<CorrCfg> {
    let mut grid = vec![CorrCfg { threshold: 0.0, min_size: 0, max_coins: 99, suppress_cd: 0, is_baseline: true }];
    let thresholds = [0.60, 0.70, 0.80];
    let min_sizes = [3usize, 4];
    let max_coins_vals = [1usize, 2];
    let suppress_cds = [4usize, 8, 12];
    for &t in &thresholds {
        for &ms in &min_sizes {
            for &mc in &max_coins_vals {
                for &sc in &suppress_cds {
                    grid.push(CorrCfg { threshold: t, min_size: ms, max_coins: mc, suppress_cd: sc, is_baseline: false });
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
    returns: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, suppressed: usize }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, suppressed_entries: usize, coins: Vec<CoinResult>,
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
    let mut returns = vec![f64::NAN; n];
    for i in 1..n {
        returns[i] = if closes[i-1] > 0.0 { (closes[i] - closes[i-1]) / closes[i-1] } else { 0.0 };
    }
    Some(CoinData15m { name: coin.to_string(), closes, opens, zscore, returns })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn pearson_corr(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.len() < 5 { return 0.0; }
    let n = a.len() as f64;
    let (sum_a, sum_b, sum_ab, sum_a2, sum_b2) = a.iter().zip(b.iter())
        .fold((0.0, 0.0, 0.0, 0.0, 0.0), |(sa,sb,sab,sa2,sb2), (x,y)| {
            (sa+x, sb+y, sab+x*y, sa2+x*x, sb2+y*y)
        });
    let num = n*sum_ab - sum_a*sum_b;
    let den = ((n*sum_a2-sum_a*sum_a)*(n*sum_b2-sum_b*sum_b)).sqrt();
    if den == 0.0 { 0.0 } else { num/den }
}

fn trailing_sharpe(trades: &[(f64, f64)], lookback: usize) -> f64 {
    let start = trades.len().saturating_sub(lookback);
    let rets: Vec<f64> = trades[start..].iter().map(|&(pnl, _)| pnl).collect();
    if rets.len() < 3 { return 0.0; }
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let std = (rets.iter().map(|x| (x-mean).powi(2)).sum::<f64>() / rets.len() as f64).sqrt();
    if std == 0.0 { 0.0 } else { mean / std * (252f64.sqrt()) }
}

fn compute_corr_matrix(data: &[&CoinData15m], bar: usize, lookback: usize) -> Vec<Vec<f64>> {
    let n = data.len();
    let mut corr = vec![vec![0.0; n]; n];
    for i in 0..n {
        corr[i][i] = 1.0;
        for j in (i+1)..n {
            let start = bar.saturating_sub(lookback);
            let len = (bar - start).min(data[i].returns.len());
            if len < 5 { continue; }
            let a = &data[i].returns[start..start+len];
            let b = &data[j].returns[start..start+len];
            let r = pearson_corr(a, b);
            corr[i][j] = r;
            corr[j][i] = r;
        }
    }
    corr
}

fn find_clusters(corr: &[Vec<f64>], threshold: f64, min_size: usize) -> Vec<Vec<usize>> {
    let n = corr.len();
    let mut visited = vec![false; n];
    let mut clusters = Vec::new();
    for i in 0..n {
        if visited[i] { continue; }
        let mut cluster = vec![i];
        visited[i] = true;
        for j in (i+1)..n {
            if !visited[j] && corr[i][j] > threshold {
                cluster.push(j);
                visited[j] = true;
            }
        }
        if cluster.len() >= min_size {
            clusters.push(cluster);
        }
    }
    clusters
}

fn simulate(cfg: &CorrCfg, coin_data: &[CoinData15m]) -> ConfigResult {
    let n = coin_data[0].closes.len();
    let mut balances = vec![INITIAL_BAL; N_COINS];
    let mut positions: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut cooldowns = vec![0usize; N_COINS];
    let mut suppress_until = vec![0usize; N_COINS];
    let mut wins = vec![0usize; N_COINS];
    let mut losses = vec![0usize; N_COINS];
    let mut flats = vec![0usize; N_COINS];
    let mut suppressed_total = 0usize;
    let mut all_trades: Vec<Vec<(f64, f64)>> = vec![Vec::new(); N_COINS];

    // Correlation lookback: 20 bars
    let corr_lookback = 20usize;

    for i in 1..n {
        // Compute correlation matrix for this bar (every 5 bars to save compute)
        let corr = if i % 5 == 0 && i >= corr_lookback {
            let data_refs: Vec<&CoinData15m> = coin_data.iter().collect();
            Some(compute_corr_matrix(&data_refs, i, corr_lookback))
        } else { None };

        // Determine suppressed coins due to correlation clustering
        if !cfg.is_baseline {
            if let Some(ref cm) = corr {
                let clusters = find_clusters(cm, cfg.threshold, cfg.min_size);
                for cluster in clusters {
                    // Find which coins in cluster are signaling
                    let mut signaling = Vec::new();
                    for &ci in &cluster {
                        if positions[ci].is_none() && cooldowns[ci] == 0 && suppress_until[ci] <= i {
                            if regime_signal(coin_data[ci].zscore[i]).is_some() {
                                let sharpe = trailing_sharpe(&all_trades[ci], 20);
                                signaling.push((ci, sharpe));
                            }
                        }
                    }
                    if signaling.len() >= cfg.min_size {
                        signaling.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                        for &(ci, _) in signaling.iter().skip(cfg.max_coins) {
                            suppress_until[ci] = i + cfg.suppress_cd;
                        }
                    }
                }
            }
        }

        // Process each coin
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
                    all_trades[c].push((exit_pct, 0.0));
                    positions[c] = None;
                    cooldowns[c] = COOLDOWN;
                }
            } else if cooldowns[c] > 0 {
                cooldowns[c] -= 1;
            } else if suppress_until[c] > i {
                // suppressed
            } else {
                if let Some(dir) = regime_signal(d.zscore[i]) {
                    if i + 1 < n {
                        let entry_price = d.opens[i + 1];
                        if entry_price > 0.0 { positions[c] = Some((dir, entry_price)); }
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

    let coins: Vec<CoinResult> = (0..N_COINS).map(|c| {
        let pnl = balances[c] - INITIAL_BAL;
        let trades = wins[c] + losses[c] + flats[c];
        let wr = if trades > 0 { wins[c] as f64 / trades as f64 * 100.0 } else { 0.0 };
        CoinResult { coin: coin_data[c].name.clone(), pnl, trades, wins: wins[c], losses: losses[c], wr, suppressed: 0 }
    }).collect();

    ConfigResult { label: cfg.label(), total_pnl, portfolio_wr, total_trades, pf, is_baseline: cfg.is_baseline, suppressed_entries: suppressed_total, coins }
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN86 — Coin Correlation Clustering\n");
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
    eprintln!("\nGrid: {} configs × {} coins (portfolio-level)", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.is_baseline, suppressed_entries: 0, coins: vec![] };
        }
        let result = simulate(cfg, &coin_data);
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}",
            d, total_cfgs, result.label, result.total_pnl, result.portfolio_wr, result.total_trades);
        result
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN86 Coin Correlation Clustering Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<22} {:>8} {:>8} {:>6} {:>7} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF");
    println!("{}", "-".repeat(70));
    for (i, r) in sorted.iter().enumerate().take(30) {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<22} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf);
    }
    println!("... ({} total)", sorted.len());
    println!("{}", "=".repeat(70));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN86 corr cluster. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run86_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run86_1_results.json");
}
