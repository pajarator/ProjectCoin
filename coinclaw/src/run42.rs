/// RUN42 — Dynamic Leverage by Volatility Regime: Risk-Adjusted Position Sizing
///
/// Hypothesis: Adjust leverage downward during HighVol/Squeeze regimes and upward
/// during Ranging/calm regimes. This changes position size without changing SL%.
///
/// Grid:
///   LEVERAGE_RANGING:  [5.0, 6.0, 7.0, 8.0]
///   LEVERAGE_HIGHVOL:  [1.5, 2.0, 2.5, 3.0]
///   LEVERAGE_SQUEEZE:  [2.0, 3.0, 4.0]
///   LEVERAGE_STRONGTREND: [2.0, 3.0, 4.0]
///   LEVERAGE_WEAKTREND: 5.0 (baseline anchor)
///   = 144 configs × 18 coins = 2,592 backtests
///
/// Run: cargo run --release --features run42 -- --run42

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// ── Constants ────────────────────────────────────────────────────────────────
const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const RISK: f64 = 0.10;
const MIN_HOLD_BARS: u32 = 2;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02; // 2% of balance (fixed for fair comparison)

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

// ── Regime detection ────────────────────────────────────────────────────────
// Simplified: based on z-score of price vs 20-bar SMA
#[derive(Clone, Copy, PartialEq, Debug)]
enum Regime {
    Ranging,
    WeakTrend,
    StrongTrend,
    HighVol,
    Squeeze,
}

fn detect_regime(z: f64, atr_ratio: f64) -> Regime {
    // z: how many std deviations price is from 20-bar SMA
    // atr_ratio: current ATR / 20-bar ATR (>1 = high vol)
    if z.abs() < 1.0 && atr_ratio < 1.2 { Regime::Ranging }
    else if z.abs() < 2.0 { Regime::WeakTrend }
    else if z > 2.0 && atr_ratio < 1.2 { Regime::StrongTrend }
    else if atr_ratio > 1.5 { Regime::HighVol }
    else { Regime::Squeeze }
}

// ── Grid ────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
struct LevCfg {
    ranging: f64,
    highvol: f64,
    squeeze: f64,
    strongtrend: f64,
}

impl LevCfg {
    fn label(&self) -> String {
        format!("R{:.0}_H{:.1}_S{:.0}_T{:.0}",
            self.ranging, self.highvol, self.squeeze, self.strongtrend)
    }
    fn lev_for(&self, r: Regime) -> f64 {
        match r {
            Regime::Ranging => self.ranging,
            Regime::HighVol => self.highvol,
            Regime::Squeeze => self.squeeze,
            Regime::StrongTrend => self.strongtrend,
            Regime::WeakTrend => 5.0, // baseline anchor
        }
    }
}

fn build_grid() -> Vec<LevCfg> {
    let mut grid = Vec::new();
    // Baseline: all 5.0
    grid.push(LevCfg { ranging: 5.0, highvol: 5.0, squeeze: 5.0, strongtrend: 5.0 });
    let ranging_vals = [5.0f64, 6.0, 7.0, 8.0];
    let highvol_vals = [1.5f64, 2.0, 2.5, 3.0];
    let squeeze_vals = [2.0f64, 3.0, 4.0];
    let strongtrend_vals = [2.0f64, 3.0, 4.0];
    for r in ranging_vals {
        for h in highvol_vals {
            for s in squeeze_vals {
                for t in strongtrend_vals {
                    grid.push(LevCfg { ranging: r, highvol: h, squeeze: s, strongtrend: t });
                }
            }
        }
    }
    grid
}

// ── Data structures ─────────────────────────────────────────────────────────
struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    atr: Vec<f64>,
    atr_ma: Vec<f64>,
    zscore: Vec<f64>,
}

struct TradeResult {
    pnl: f64,
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    max_dd: f64,
    win_pnls: Vec<f64>,
    loss_pnls: Vec<f64>,
    returns: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    pnl: f64,
    sharpe: f64,
    max_dd: f64,
    wr: f64,
    pf: f64,
    trades: usize,
    wins: usize,
    losses: usize,
    avg_win: f64,
    avg_loss: f64,
    delta_sharpe: f64,
    delta_pnl: f64,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_sharpe: f64,
    portfolio_wr: f64,
    portfolio_max_dd: f64,
    total_trades: usize,
    pf: f64,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

// ── Helpers ─────────────────────────────────────────────────────────────────
fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n]; let mut sum = 0.0;
    for i in 0..n { sum += data[i]; if i>=w { sum -= data[i-w]; } if i+1>=w { out[i]=sum/w as f64; } }
    out
}
fn rstd(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { let s=&data[i+1-w..=i]; let m=s.iter().sum::<f64>()/w as f64; let v=s.iter().map(|x|(x-m).powi(2)).sum::<f64>()/w as f64; out[i]=v.sqrt(); }
    out
}
fn rmin(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { out[i]=data[i+1-w..=i].iter().cloned().fold(f64::INFINITY, f64::min); }
    out
}
fn rmax(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { out[i]=data[i+1-w..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max); }
    out
}
fn compute_atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let n = close.len();
    let mut tr = vec![f64::NAN; n];
    tr[0] = high[0] - low[0];
    for i in 1..n {
        tr[i] = high[i].max(close[i-1]) - low[i].min(close[i-1]);
    }
    rmean(&tr, period)
}

// ── CSV loader ────────────────────────────────────────────────────────────────
fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    let mut closes = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next()?;
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let _vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || hh.is_nan() || ll.is_nan() || cc.is_nan() { continue; }
        opens.push(oo); highs.push(hh); lows.push(ll); closes.push(cc);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();
    let sma = rmean(&closes, 20);
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        if !sma[i].is_nan() {
            let window = &closes[i+1-20..=i];
            let mean = window.iter().sum::<f64>()/20.0;
            let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
            zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
        }
    }
    let atr = compute_atr(&highs, &lows, &closes, 14);
    let atr_ma = rmean(&atr, 20);
    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, atr, atr_ma, zscore })
}

// ── Regime signal (COINCLAW v13) ─────────────────────────────────────────────
fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let z = d.zscore[i];
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// ── Simulation ────────────────────────────────────────────────────────────────
fn simulate(d: &CoinData15m, cfg: LevCfg) -> TradeResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut peak = INITIAL_BAL;
    let mut pos: Option<(i8, f64, f64)> = None; // (dir, entry, notional)
    let mut cooldown = 0usize;
    let mut win_pnls = Vec::new();
    let mut loss_pnls = Vec::new();
    let mut returns = Vec::new();
    let mut flats = 0usize;

    for i in 1..n {
        // Regime at bar i
        let z = d.zscore[i];
        let atr_r = if !d.atr_ma[i].is_nan() && d.atr_ma[i] > 0.0 {
            d.atr[i] / d.atr_ma[i]
        } else { 1.0 };
        let regime = if z.is_nan() { Regime::Ranging } else { detect_regime(z, atr_r) };

        if let Some((dir, entry, _notional)) = pos {
            let pct = if dir == 1 {
                (d.closes[i] - entry) / entry
            } else {
                (entry - d.closes[i]) / entry
            };
            let mut closed = false;
            let mut exit_pct = 0.0;
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed {
                // Hold for min bars then exit on signal reversal
                let new_dir = regime_signal(d, i);
                if new_dir.is_some() && new_dir != Some(dir) {
                    exit_pct = pct; closed = true;
                }
            }
            if !closed && i >= 20 { exit_pct = pct; closed = true; }

            if closed {
                let lev = cfg.lev_for(regime);
                let trade_amt = bal * RISK;
                let notional = trade_amt * lev;
                let net = notional * exit_pct;
                bal += net;
                if !entry.is_nan() && entry > 0.0 {
                    let ret = exit_pct * lev;
                    returns.push(ret);
                }
                if net > 1e-10 { win_pnls.push(net); }
                else if net < -1e-10 { loss_pnls.push(net); }
                else { flats += 1; }
                pos = None; cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            if let Some(dir) = regime_signal(d, i) {
                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 {
                        let lev = cfg.lev_for(regime);
                        let trade_amt = bal * RISK;
                        let notional = trade_amt * lev;
                        pos = Some((dir, entry_price, notional));
                    }
                }
            }
        }

        if bal > peak { peak = bal; }
    }

    let total_trades = win_pnls.len() + loss_pnls.len() + flats;
    let max_dd = if peak > 0.0 { (peak - bal) / peak } else { 0.0 };
    TradeResult {
        pnl: bal - INITIAL_BAL,
        trades: total_trades,
        wins: win_pnls.len(),
        losses: loss_pnls.len(),
        flats,
        max_dd,
        win_pnls,
        loss_pnls,
        returns,
    }
}

fn sharpe(returns: &[f64]) -> f64 {
    if returns.len() < 2 { return 0.0; }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    let std = variance.sqrt();
    if std < 1e-10 { return 0.0; }
    // Annualize: 15m bars → 35040 bars/year
    let annualized_mean = mean * 35040.0;
    let annualized_std = std * (35040.0f64).sqrt();
    annualized_mean / annualized_std
}

// ── Entry point ──────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN42 — Dynamic Leverage by Volatility Regime Grid Search");
    eprintln!("LEVERAGE_RANGING × LEVERAGE_HIGHVOL × LEVERAGE_SQUEEZE × LEVERAGE_STRONGTREND\n");

    // Load data
    eprintln!("Loading 15m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData15m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_15m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.closes.len()); }
        raw_data.push(loaded);
    }
    let all_ok = raw_data.iter().all(|r| r.is_some());
    if !all_ok { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }

    let coin_data: Vec<CoinData15m> = raw_data.into_iter().map(|r| r.unwrap()).collect();

    // Build grid
    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins = {} backtests",
        grid.len(), N_COINS, grid.len() * N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_sharpe: 0.0,
                portfolio_wr: 0.0, portfolio_max_dd: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.ranging == 5.0 && cfg.highvol == 5.0,
                coins: vec![],
            };
        }

        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let r = simulate(d, *cfg);
            let total = r.trades;
            let wins_sum = r.wins;
            let wr = if total > 0 { wins_sum as f64 / total as f64 * 100.0 } else { 0.0 };
            let avg_win = if r.wins > 0 { r.win_pnls.iter().sum::<f64>() / r.wins as f64 } else { 0.0 };
            let avg_loss = if r.losses > 0 { r.loss_pnls.iter().sum::<f64>() / r.losses as f64 } else { 0.0 };
            let pf = if avg_loss.abs() > 1e-8 { avg_win / avg_loss.abs() } else { 0.0 };
            let sh = sharpe(&r.returns);
            CoinResult {
                coin: d.name.clone(),
                pnl: r.pnl,
                sharpe: sh,
                max_dd: r.max_dd,
                wr,
                pf,
                trades: total,
                wins: r.wins,
                losses: r.losses,
                avg_win,
                avg_loss,
                delta_sharpe: 0.0, // computed below after baseline
                delta_pnl: 0.0,
            }
        }).collect();

        // Compute baseline for delta
        let baseline_cfg = LevCfg { ranging: 5.0, highvol: 5.0, squeeze: 5.0, strongtrend: 5.0 };
        let baseline_results: Vec<TradeResult> = coin_data.iter().map(|d| simulate(d, baseline_cfg)).collect();
        let mut final_coins = Vec::new();
        for (i, cr) in coin_results.iter().enumerate() {
            let br = &baseline_results[i];
            let b_total = br.trades;
            let b_wins = br.wins;
            let _b_wr = if b_total > 0 { b_wins as f64 / b_total as f64 * 100.0 } else { 0.0 };
            let b_sharpe = sharpe(&br.returns);
            let b_pnl = br.pnl;
            final_coins.push(CoinResult {
                coin: cr.coin.clone(),
                pnl: cr.pnl,
                sharpe: cr.sharpe,
                max_dd: cr.max_dd,
                wr: cr.wr,
                pf: cr.pf,
                trades: cr.trades,
                wins: cr.wins,
                losses: cr.losses,
                avg_win: cr.avg_win,
                avg_loss: cr.avg_loss,
                delta_sharpe: cr.sharpe - b_sharpe,
                delta_pnl: cr.pnl - b_pnl,
            });
        }

        let total_pnl: f64 = final_coins.iter().map(|c| c.pnl).sum();
        let all_returns: Vec<f64> = baseline_results.iter().flat_map(|r| r.returns.iter()).cloned().collect();
        let portfolio_sharpe = sharpe(&all_returns);
        let total_trades: usize = final_coins.iter().map(|c| c.trades).sum();
        let wins_sum: usize = final_coins.iter().map(|c| c.wins).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let portfolio_max_dd = final_coins.iter().map(|c| c.max_dd).fold(0.0f64, f64::max);
        let wins_total: usize = final_coins.iter().filter(|c|c.wins>0).map(|c|c.wins).sum();
        let losses_total: usize = final_coins.iter().filter(|c|c.losses>0).map(|c|c.losses).sum();
        let avg_win_all = if wins_total>0 { final_coins.iter().filter(|c|c.wins>0).map(|c|c.avg_win*c.wins as f64).sum::<f64>()/wins_total as f64 } else { 0.0 };
        let avg_loss_all = if losses_total>0 { final_coins.iter().filter(|c|c.losses>0).map(|c|c.avg_loss*c.losses as f64).sum::<f64>()/losses_total as f64 } else { 0.0 };
        let pf = if avg_loss_all.abs()>1e-8 { avg_win_all/avg_loss_all.abs() } else { 0.0 };
        let is_baseline = cfg.ranging == 5.0 && cfg.highvol == 5.0;

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+7.2}  Sharpe={:.3}  WR={:>5.1}%  MaxDD={:>5.1}%  trades={}",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_sharpe, portfolio_wr, portfolio_max_dd*100.0, total_trades);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_sharpe, portfolio_wr,
            portfolio_max_dd, total_trades, pf, is_baseline, coins: final_coins,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Interrupted — saving partial...");
        let output = Output { notes: "RUN42 interrupted".to_string(), configs: results };
        std::fs::write("/home/scamarena/ProjectCoin/run42_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
        return;
    }

    eprintln!();
    let baseline = results.iter().find(|r| r.is_baseline).unwrap();

    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.portfolio_sharpe.partial_cmp(&a.portfolio_sharpe).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN42 Dynamic Leverage Results ===");
    println!("Baseline (5/5/5/5): PnL={:+.2}  Sharpe={:.3}  WR={:.1}%  MaxDD={:.1}%",
        baseline.total_pnl, baseline.portfolio_sharpe, baseline.portfolio_wr, baseline.portfolio_max_dd*100.0);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "#", "Config", "PnL", "Sharpe", "WR%", "MaxDD%", "Trades");
    println!("{}", "-".repeat(75));
    for (i,r) in sorted.iter().enumerate() {
        println!("{:>3}  {:<20} {:>+8.2} {:>8.3} {:>6.1}% {:>7.1}% {:>6}",
            i+1, r.label, r.total_pnl, r.portfolio_sharpe, r.portfolio_wr, r.portfolio_max_dd*100.0, r.total_trades);
        if i >= 19 { println!("  ... ({} more)", sorted.len()-20); break; }
    }
    println!("{}", "=".repeat(75));

    let best = sorted.first().unwrap();
    let is_positive = best.portfolio_sharpe > baseline.portfolio_sharpe;
    let sharre_delta = best.portfolio_sharpe - baseline.portfolio_sharpe;
    println!("\nVERDICT: {} (best Sharpe={:.3} ΔSharpe={:+.3})",
        if is_positive { "POSITIVE" } else { "NEGATIVE" },
        best.portfolio_sharpe, sharre_delta);

    let notes = format!("RUN42 dynamic leverage. {} configs. Baseline Sharpe={:.3}. Best: {} (Sharpe={:.3}, Δ={:+.3})",
        results.len(), baseline.portfolio_sharpe, best.label, best.portfolio_sharpe, sharre_delta);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run42_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run42_1_results.json");
}
