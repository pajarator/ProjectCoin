/// RUN72 — Scalp Choppy Mode
///
/// Grid: CHOPPY_ATR_THRESHOLD [0.0010, 0.0015, 0.0020, 0.0025] × CHOPPY_BARS [4, 8, 12]
/// Total: 4 × 3 = 12 configs + baseline = 13
///
/// Suppress scalp entries when market-wide ATR is below threshold for sustained period.
///
/// Run: cargo run --release --features run72 -- --run72

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const INITIAL_BAL: f64 = 100.0;
const SCALP_SL: f64 = 0.001;
const SCALP_TP: f64 = 0.008;
const BASE_POSITION_SIZE: f64 = 0.02;
const LEVERAGE: f64 = 5.0;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct ChoppyCfg {
    atr_threshold: f64,
    choppy_bars: usize,
}

impl ChoppyCfg {
    fn label(&self) -> String {
        format!("AT{:.4}_CB{}", self.atr_threshold, self.choppy_bars)
    }
}

fn build_grid() -> Vec<ChoppyCfg> {
    let mut grid = vec![ChoppyCfg { atr_threshold: 0.0, choppy_bars: 0 }]; // baseline
    let thresholds = [0.0010, 0.0015, 0.0020, 0.0025];
    let bars = [4, 8, 12];
    for &at in &thresholds {
        for &cb in &bars {
            grid.push(ChoppyCfg { atr_threshold: at, choppy_bars: cb });
        }
    }
    grid
}

struct CoinData1m {
    closes: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    volumes: Vec<f64>,
    atr: Vec<f64>,
    rsi: Vec<f64>,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    pf: f64,
    is_baseline: bool,
    choppy_rate: f64,
    blocked: usize,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn load_1m(coin: &str) -> Option<CoinData1m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut closes = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    let mut volumes = Vec::new();

    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let _ts = it.next()?;
        let _o: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let vv: f64 = it.next()?.parse().ok()?;
        if cc.is_nan() || hh.is_nan() || ll.is_nan() || vv.is_nan() { continue; }
        closes.push(cc); highs.push(hh); lows.push(ll); volumes.push(vv);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();

    // ATR(14) - True Range
    let mut tr = vec![f64::NAN; n];
    let mut atr = vec![f64::NAN; n];
    for i in 1..n {
        let h = highs[i];
        let l = lows[i];
        let prev_c = closes[i-1];
        tr[i] = (h - l).max((h - prev_c).abs()).max((l - prev_c).abs());
    }
    for i in 14..n {
        atr[i] = tr[i+1-14..=i].iter().filter(|&&x| !x.is_nan()).sum::<f64>() / 14.0;
    }

    // ATR% = ATR / close
    let mut atr_pct = vec![f64::NAN; n];
    for i in 14..n {
        atr_pct[i] = atr[i] / closes[i];
    }

    // RSI(14)
    let mut rsi = vec![f64::NAN; n];
    for i in 14..n {
        let mut gain_sum = 0.0;
        let mut loss_sum = 0.0;
        for j in i+1-14..=i {
            let delta = closes[j] - closes[j-1];
            if delta > 0.0 { gain_sum += delta; }
            else { loss_sum += -delta; }
        }
        let avg_gain = gain_sum / 14.0;
        let avg_loss = loss_sum / 14.0;
        rsi[i] = if avg_loss == 0.0 { 100.0 } else { 100.0 - 100.0/(1.0 + avg_gain/avg_loss) };
    }

    Some(CoinData1m { closes, highs, lows, volumes, atr, rsi })
}

// Check if market-wide ATR is "choppy" (below threshold for sustained period)
fn is_choppy(atr_pcts: &[f64], threshold: f64, choppy_bars: usize, i: usize) -> bool {
    if i < choppy_bars { return false; }
    for j in (i+1-choppy_bars)..=i {
        let at = atr_pcts[j];
        if !at.is_nan() && at >= threshold { return false; }
    }
    true
}

fn vol_spike(volumes: &[f64], i: usize, vol_ma: f64) -> bool {
    if vol_ma <= 0.0 { return false; }
    volumes[i] > vol_ma * 1.5
}

// Volume MA (20-period)
fn vol_ma(volumes: &[f64], i: usize) -> f64 {
    if i < 20 { return f64::NAN; }
    volumes[i+1-20..=i].iter().sum::<f64>() / 20.0
}

fn scalp_entry(d: &CoinData1m, i: usize) -> Option<i8> {
    if i < 25 { return None; }
    let rsi = d.rsi[i];
    let z_thresh = 2.0;
    // Quick z-score approximation using RSI
    let z_long = (rsi - 30.0) / 10.0; // rough
    let spike = vol_spike(&d.volumes, i, vol_ma(&d.volumes, i));

    // LONG: oversold + volume spike
    if z_long < -z_thresh && rsi < 30.0 && spike {
        return Some(1);
    }
    // SHORT: overbought + volume spike
    if z_long > z_thresh && rsi > 70.0 && spike {
        return Some(-1);
    }
    None
}

// Portfolio-level simulation with choppy mode
fn simulate_portfolio(data: &[CoinData1m], cfg: ChoppyCfg) -> (f64, usize, usize, usize, usize, f64) {
    let n = data[0].closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Vec<Option<(i8, f64)>> = vec![None; N_COINS];
    let mut choppy_bars = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut blocked = 0usize;
    let mut _total_choppy = 0usize;

    // Step through at 5-bar intervals for speed
    let step = 5;
    for i in (step..n).step_by(step) {
        // Check if market is choppy
        let is_chop = if cfg.atr_threshold > 0.0 {
            is_choppy_scale(data, cfg.atr_threshold, cfg.choppy_bars, i)
        } else { false };

        if is_chop { choppy_bars += 1; }

        // Process exits
        for ci in 0..N_COINS {
            let d = &data[ci];
            if let Some((dir, entry)) = pos[ci] {
                let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
                let mut closed = false;
                let mut exit_pct = 0.0;
                if pct >= SCALP_TP { exit_pct = SCALP_TP; closed = true; }
                else if pct <= -SCALP_SL { exit_pct = -SCALP_SL; closed = true; }
                if closed {
                    let net = (bal * BASE_POSITION_SIZE * LEVERAGE) * exit_pct;
                    bal += net;
                    if net > 1e-10 { wins += 1; }
                    else if net < -1e-10 { losses += 1; }
                    else { flats += 1; }
                    pos[ci] = None;
                }
            }
        }

        // Process entries
        if !is_chop || cfg.atr_threshold == 0.0 {
            for ci in 0..N_COINS {
                if pos[ci].is_some() { continue; }
                let d = &data[ci];
                if let Some(dir) = scalp_entry(d, i) {
                    if i + 1 < n {
                        let entry_price = d.closes[i + 1];
                        if entry_price > 0.0 { pos[ci] = Some((dir, entry_price)); }
                    }
                }
            }
        } else {
            // Choppy mode: count blocked entries (approximate)
            for ci in 0..N_COINS {
                if pos[ci].is_some() { continue; }
                let d = &data[ci];
                if scalp_entry(d, i).is_some() {
                    blocked += 1;
                }
            }
        }
    }

    let total_bars = n / step;
    let choppy_rate = if total_bars > 0 { choppy_bars as f64 / total_bars as f64 * 100.0 } else { 0.0 };
    (bal - INITIAL_BAL, wins, losses, flats, blocked, choppy_rate)
}

fn is_choppy_scale(data: &[CoinData1m], threshold: f64, bars: usize, i: usize) -> bool {
    if i < bars { return false; }
    for ci in 0..N_COINS {
        let d = &data[ci];
        if i >= d.atr.len() { continue; }
        let at = d.atr[i] / d.closes[i];
        if at.is_nan() { continue; }
        if at >= threshold { return false; }
    }
    // All valid coins are below threshold
    let mut all_below = true;
    for ci in 0..N_COINS {
        let d = &data[ci];
        if i >= d.atr.len() { continue; }
        let at = d.atr[i] / d.closes[i];
        if at.is_nan() { continue; }
        if at >= threshold { all_below = false; break; }
    }
    all_below
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN72 — Scalp Choppy Mode\n");
    eprintln!("Loading 1m data for {} coins...", N_COINS);
    let mut raw_data: Vec<Option<CoinData1m>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_1m(name);
        if let Some(ref data) = loaded { eprintln!("  {} — {} bars", name, data.closes.len()); }
        raw_data.push(loaded);
    }
    if !raw_data.iter().all(|r| r.is_some()) { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }
    let coin_data: Vec<CoinData1m> = raw_data.into_iter().map(|r| r.unwrap()).collect();
    let grid = build_grid();
    eprintln!("\nGrid: {} configs × portfolio", grid.len());

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0,
                total_trades: 0, pf: 0.0, is_baseline: cfg.atr_threshold == 0.0,
                choppy_rate: 0.0, blocked: 0,
            };
        }
        let (pnl, wins, losses, flats, blocked, choppy_rate) = simulate_portfolio(&coin_data, *cfg);
        let total_trades = wins + losses + flats;
        let wr = if total_trades > 0 { wins as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins as f64 * SCALP_TP * BASE_POSITION_SIZE * LEVERAGE;
        let pf = if losses > 0 { gross / (losses as f64 * SCALP_SL * BASE_POSITION_SIZE * LEVERAGE) } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  choppy={:.1}%  blocked={}",
            d, total_cfgs, cfg.label(), pnl, wr, total_trades, choppy_rate, blocked);

        ConfigResult {
            label: cfg.label(),
            total_pnl: pnl,
            portfolio_wr: wr,
            total_trades,
            pf,
            is_baseline: cfg.atr_threshold == 0.0,
            choppy_rate,
            blocked,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN72 Scalp Choppy Mode Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<20} {:>8} {:>8} {:>6}  {:>6}  {:>7}  {:>8}", "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "Choppy%", "Blocked");
    println!("{}", "-".repeat(80));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<20} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6}  {:>6.1}%  {:>7}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.choppy_rate, r.blocked);
    }
    println!("{}", "=".repeat(80));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN72 scalp choppy. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run72_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run72_1_results.json");
}