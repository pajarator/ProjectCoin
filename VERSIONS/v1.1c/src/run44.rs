/// RUN44 — Multi-Timeframe ISO Short Confirmation
///
/// ISO short entries require 1h and 4h RSI overbought confirmation.
/// Since only 15m data is available, simulate higher timeframes:
///   1h: aggregate every 4 × 15m bars
///   4h: aggregate every 16 × 15m bars
///
/// Grid:
///   ISO_1H_RSI_THRESH: [60, 65, 70]  (1h RSI must exceed this for ISO short)
///   ISO_4H_RSI_THRESH: [65, 70, 75]  (4h RSI must exceed this for ISO short)
///   = 9 configs
///
/// Run: cargo run --release --features run44 -- --run44

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: u32 = 2;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct MTFCfg {
    rsi_1h_thresh: f64,
    rsi_4h_thresh: f64,
}

impl MTFCfg {
    fn label(&self) -> String {
        format!("H{:.0}_F{:.0}", self.rsi_1h_thresh, self.rsi_4h_thresh)
    }
}

fn build_grid() -> Vec<MTFCfg> {
    let mut grid = Vec::new();
    // Baseline: no MTF filter
    grid.push(MTFCfg { rsi_1h_thresh: 999.0, rsi_4h_thresh: 999.0 });
    let h_vals = [60.0f64, 65.0, 70.0];
    let f_vals = [65.0f64, 70.0, 75.0];
    for h in h_vals {
        for f in f_vals {
            grid.push(MTFCfg { rsi_1h_thresh: h, rsi_4h_thresh: f });
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
    rsi_15m: Vec<f64>,
    // Aggregated 1h RSI (every 4 bars)
    rsi_1h: Vec<f64>,
    // Aggregated 4h RSI (every 16 bars)
    rsi_4h: Vec<f64>,
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    pnl: f64,
    trades: usize,
    wins: usize,
    losses: usize,
    wr: f64,
    pf: f64,
    delta_pnl: f64,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    total_delta: f64,
    portfolio_wr: f64,
    total_trades: usize,
    pf: f64,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn rsi_calc(c: &[f64], period: usize) -> Vec<f64> {
    let n = c.len(); let mut out = vec![f64::NAN; n];
    if n < period+1 { return out; }
    let mut gains = vec![0.0; n]; let mut losses = vec![0.0; n];
    for i in 1..n { let d=c[i]-c[i-1]; if d>0.0{gains[i]=d;}else{losses[i]=-d;} }
    let mut sum_g = 0.0; let mut sum_l = 0.0;
    for i in 1..=period { sum_g += gains[i]; sum_l += losses[i]; }
    for i in period..n {
        if i > period { sum_g = sum_g - gains[i-period] + gains[i]; sum_l = sum_l - losses[i-period] + losses[i]; }
        let rs = if sum_l == 0.0 { 100.0 } else { sum_g/sum_l };
        out[i] = 100.0 - 100.0/(1.0 + rs);
    }
    out
}

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
    if closes.len() < 100 { return None; }

    let rsi_15m = rsi_calc(&closes, 14);

    // Aggregate to 1h (4 bars)
    let n = closes.len();
    let n_1h = n / 4;
    let mut agg_1h_close = vec![0.0f64; n_1h];
    let mut agg_1h_open = vec![0.0f64; n_1h];
    let mut agg_1h_high = vec![f64::NEG_INFINITY; n_1h];
    let mut agg_1h_low = vec![f64::INFINITY; n_1h];
    for i in 0..n_1h {
        let base = i * 4;
        agg_1h_open[i] = opens[base];
        for j in 0..4 {
            if base+j < n {
                agg_1h_high[i] = agg_1h_high[i].max(highs[base+j]);
                agg_1h_low[i] = agg_1h_low[i].min(lows[base+j]);
                agg_1h_close[i] = closes[base+j]; // last close in the hour
            }
        }
    }
    let rsi_1h = rsi_calc(&agg_1h_close, 14);

    // Aggregate to 4h (16 bars)
    let n_4h = n / 16;
    let mut agg_4h_close = vec![0.0f64; n_4h];
    let mut agg_4h_open = vec![0.0f64; n_4h];
    let mut agg_4h_high = vec![f64::NEG_INFINITY; n_4h];
    let mut agg_4h_low = vec![f64::INFINITY; n_4h];
    for i in 0..n_4h {
        let base = i * 16;
        agg_4h_open[i] = opens[base];
        for j in 0..16 {
            if base+j < n {
                agg_4h_high[i] = agg_4h_high[i].max(highs[base+j]);
                agg_4h_low[i] = agg_4h_low[i].min(lows[base+j]);
                agg_4h_close[i] = closes[base+j];
            }
        }
    }
    let rsi_4h = rsi_calc(&agg_4h_close, 14);

    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, rsi_15m, rsi_1h, rsi_4h })
}

fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    // Simplified: price below lower band → long, above → short
    // Use RSI as proxy for overbought/oversold
    let rsi = d.rsi_15m[i];
    if rsi.is_nan() { return None; }
    if rsi < 30.0 { return Some(1); }  // oversold → potential long
    if rsi > 70.0 { return Some(-1); } // overbought → ISO short
    None
}

// ISO short signal: 15m RSI > 70 + 1h RSI above threshold + 4h RSI above threshold
fn iso_short_signal(d: &CoinData15m, i: usize, cfg: MTFCfg) -> bool {
    if cfg.rsi_1h_thresh >= 999.0 { return true; } // baseline, no filter
    let rsi_15 = d.rsi_15m[i];
    if rsi_15.is_nan() || rsi_15 < 70.0 { return false; }
    // Map 15m bar i to 1h bar (every 4 bars) and 4h bar (every 16 bars)
    let i1h = i / 4;
    let i4h = i / 16;
    let rsi_1h_val = if i1h < d.rsi_1h.len() { d.rsi_1h[i1h] } else { f64::NAN };
    let rsi_4h_val = if i4h < d.rsi_4h.len() { d.rsi_4h[i4h] } else { f64::NAN };
    let pass_1h = !rsi_1h_val.is_nan() && rsi_1h_val >= cfg.rsi_1h_thresh;
    let pass_4h = !rsi_4h_val.is_nan() && rsi_4h_val >= cfg.rsi_4h_thresh;
    pass_1h && pass_4h
}

fn simulate(d: &CoinData15m, cfg: MTFCfg) -> (f64, usize, usize, usize) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64)> = None;
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;

    for i in 1..n {
        if let Some((dir, entry)) = pos {
            let pct = if dir == 1 {
                (d.closes[i] - entry) / entry
            } else {
                (entry - d.closes[i]) / entry
            };
            let mut closed = false;
            let mut exit_pct = 0.0;
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            if !closed && i > 0 {
                let new_dir = regime_signal(d, i);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; }
            }
            if !closed && i >= 20 { exit_pct = pct; closed = true; }
            if closed {
                let net = (bal * POSITION_SIZE * LEVERAGE) * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                pos = None; cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            // ISO short: 15m overbought with 1h/4h confirmation
            let rsi_15 = d.rsi_15m[i];
            if !rsi_15.is_nan() && rsi_15 > 70.0 {
                // Check if this is a valid ISO short with MTF confirmation
                if iso_short_signal(d, i, cfg) || cfg.rsi_1h_thresh >= 999.0 {
                    if i+1 < n {
                        let entry_price = d.opens[i+1];
                        if entry_price > 0.0 {
                            pos = Some((-1, entry_price));
                        }
                    }
                }
            }
            // Also allow regime LONG signals
            if let Some(1) = regime_signal(d, i) {
                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 {
                        pos = Some((1, entry_price));
                    }
                }
            }
        }
    }
    let total_trades = wins + losses + flats;
    let pnl = bal - INITIAL_BAL;
    (pnl, wins, losses, total_trades)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN44 — Multi-Timeframe ISO Short Confirmation\n");

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

    let grid = build_grid();
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult { label: cfg.label(), total_pnl: 0.0, total_delta: 0.0, portfolio_wr: 0.0, total_trades: 0, pf: 0.0, is_baseline: cfg.rsi_1h_thresh >= 999.0, coins: vec![] };
        }

        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, trades) = simulate(d, *cfg);
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, wr, pf: 0.0, delta_pnl: 0.0 }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let is_baseline = cfg.rsi_1h_thresh >= 999.0;

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+7.2}  WR={:>5.1}%  trades={}", d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades);

        ConfigResult { label: cfg.label(), total_pnl, total_delta: 0.0, portfolio_wr, total_trades, pf: 0.0, is_baseline, coins: coin_results }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Interrupted — saving partial...");
        let output = Output { notes: "RUN44 interrupted".to_string(), configs: results };
        std::fs::write("/home/scamarena/ProjectCoin/run44_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
        return;
    }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN44 Multi-Timeframe ISO Short Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<15} {:>8} {:>8} {:>8} {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(60));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<15} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}", i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(60));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    let best_delta = best.total_pnl - baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });
    println!("  (best ΔPnL={:+.2})", best_delta);

    let notes = format!("RUN44 MTF ISO short. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best_delta);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run44_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run44_1_results.json");
}
