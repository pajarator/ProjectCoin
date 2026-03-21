/// RUN41 — Session-Based Trade Filter: Asia/Europe/US Conditional Engagement
///
/// Hypothesis: Mean-reversion strategies (COINCLAW's core) work better during
/// Asia session (low volume, ranging) and worse during US session (momentum-driven).
///
/// Grid:
///   SESSION_FILTER_MODE: [0=disabled, 1=Asia, 2=Europe, 3=US, 4=Asia+Europe]
///   15m data, regime trades only (scalp already shown unprofitable in RUN29)
///
/// Run: cargo run --release --features run41 -- --run41

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// ── Constants ────────────────────────────────────────────────────────────────
const INITIAL_BAL: f64 = 100.0;
const SL_PCT: f64 = 0.003;
const MIN_HOLD_BARS: u32 = 2;
const COOLDOWN: usize = 2;
const POSITION_SIZE: f64 = 0.02; // 2% of balance

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

// ── Session definitions ──────────────────────────────────────────────────────
// Asia: 0-8 UTC, Europe: 9-16 UTC, US: 17-23 UTC
fn session_of_hour(hour: u32) -> u8 {
    if hour < 9 { 1 } else if hour < 17 { 2 } else { 3 } // 1=Asia, 2=Europe, 3=US
}

// ── Grid ────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
struct SessionCfg {
    mode: u8, // 0=disabled, 1=Asia, 2=Europe, 3=US, 4=Asia+Europe
}

impl SessionCfg {
    fn label(&self) -> String {
        match self.mode {
            0 => "DISABLED".to_string(),
            1 => "ASIA_ONLY".to_string(),
            2 => "EUROPE_ONLY".to_string(),
            3 => "US_ONLY".to_string(),
            4 => "ASIA_EUR".to_string(),
            _ => format!("MODE{}", self.mode),
        }
    }
    fn allows(&self, hour: u32) -> bool {
        let s = session_of_hour(hour);
        match self.mode {
            0 => true,
            1 => s == 1,
            2 => s == 2,
            3 => s == 3,
            4 => s == 1 || s == 2,
            _ => true,
        }
    }
}

fn build_grid() -> Vec<SessionCfg> {
    (0..=4).map(|m| SessionCfg { mode: m }).collect()
}

// ── Data structures ─────────────────────────────────────────────────────────
struct CoinData15m {
    name: String,
    closes: Vec<f64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    hours: Vec<u32>, // hour of day (0-23) for each bar
}

struct ScalpPos {
    dir: i8,
    entry: f64,
    notional: f64,
    bars_held: u32,
}

#[derive(Serialize)]
struct SessionStats {
    session: String,
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    wr: f64,
    pnl: f64,
    pf: f64,
    avg_win: f64,
    avg_loss: f64,
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    baseline_pnl: f64,
    baseline_trades: usize,
    baseline_wr: f64,
    filtered_pnl: f64,
    filtered_trades: usize,
    filtered_wr: f64,
    filtered_wins: usize,
    filtered_losses: usize,
    delta_pnl: f64,
    session_stats: Vec<SessionStats>,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    total_delta: f64,
    portfolio_wr: f64,
    total_trades: usize,
    is_baseline: bool,
    coins: Vec<CoinResult>,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

// ── Rolling helpers ─────────────────────────────────────────────────────────
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

// ── CSV loader ────────────────────────────────────────────────────────────────
fn load_15m(coin: &str) -> Option<CoinData15m> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = std::fs::read_to_string(&path).ok()?;
    let mut opens = Vec::new();
    let mut highs = Vec::new();
    let mut lows = Vec::new();
    let mut closes = Vec::new();
    let mut vols = Vec::new();
    let mut hours = Vec::new();
    for line in data.lines().skip(1) {
        let mut it = line.splitn(7, ',');
        let ts_str = it.next()?;
        // Parse timestamp: "2025-10-15 17:30:00"
        let hour: u32 = if ts_str.len() >= 13 {
            ts_str[11..13].parse().unwrap_or(12)
        } else { 12 };
        hours.push(hour);
        let oo: f64 = it.next()?.parse().ok()?;
        let hh: f64 = it.next()?.parse().ok()?;
        let ll: f64 = it.next()?.parse().ok()?;
        let cc: f64 = it.next()?.parse().ok()?;
        let vv: f64 = it.next()?.parse().ok()?;
        if oo.is_nan() || hh.is_nan() || ll.is_nan() || cc.is_nan() || vv.is_nan() { continue; }
        opens.push(oo); highs.push(hh); lows.push(ll); closes.push(cc); vols.push(vv);
    }
    if closes.len() < 100 { return None; }
    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, hours })
}

// ── Regime signal (simplified COINCLAW v13) ─────────────────────────────────
// Returns Some(1) for long, Some(-1) for short, None for no signal
fn regime_signal(d: &CoinData15m, i: usize) -> Option<i8> {
    if i < 50 { return None; }
    let c = &d.closes;
    // BB-based reversion using z-score
    let window = 20;
    if i < window { return None; }
    let mean = c[i+1-window..=i].iter().sum::<f64>() / window as f64;
    let std_val = {
        let s = &c[i+1-window..=i];
        let m = s.iter().sum::<f64>() / window as f64;
        (s.iter().map(|x| (x-m).powi(2)).sum::<f64>() / window as f64).sqrt()
    };
    let z = if std_val > 0.0 { (c[i] - mean) / std_val } else { 0.0 };

    // Regime entry: z < -2.0 → long, z > 2.0 → short
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

// ── Session profiling ─────────────────────────────────────────────────────────
struct ProfileResult {
    pnl: f64,
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    session_wins: [usize; 3],
    session_losses: [usize; 3],
    session_trades: [usize; 3],
    session_pnl: [f64; 3],
}

fn profile_sessions(d: &CoinData15m, cfg: SessionCfg) -> ProfileResult {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<ScalpPos> = None;
    let mut cooldown = 0usize;
    let mut win_pnls = Vec::new();
    let mut loss_pnls = Vec::new();
    let mut flats = 0usize;
    let mut session_wins = [0usize; 3];
    let mut session_losses = [0usize; 3];
    let mut session_trades = [0usize; 3];
    let mut session_pnl = [0.0f64; 3];

    for i in 1..n {
        if let Some(ref mut p) = pos {
            let pct = if p.dir == 1 {
                (d.closes[i] - p.entry) / p.entry
            } else {
                (p.entry - d.closes[i]) / p.entry
            };
            let mut closed = false;
            let mut exit_pct = 0.0;
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; }
            p.bars_held += 1;
            if !closed && p.bars_held >= MIN_HOLD_BARS {
                // Exit on signal or max hold
                if let Some(dir) = regime_signal(d, i) {
                    if dir != p.dir { exit_pct = pct; closed = true; }
                }
            }
            if p.bars_held >= 20 { exit_pct = pct; closed = true; } // max hold

            if closed {
                let net = p.notional * exit_pct;
                bal += net;
                let sess_idx = (d.hours[i] as usize / 9).min(2);
                if net > 1e-10 {
                    win_pnls.push(net);
                    session_wins[sess_idx] += 1;
                } else if net < -1e-10 {
                    loss_pnls.push(net);
                    session_losses[sess_idx] += 1;
                } else {
                    flats += 1;
                };
                session_trades[sess_idx] += 1;
                session_pnl[sess_idx] += net;
                pos = None; cooldown = COOLDOWN;
            }
        } else if cooldown > 0 {
            cooldown -= 1;
        } else {
            // Session filter check
            let hour = d.hours[i];
            if !cfg.allows(hour) { continue; }

            if let Some(dir) = regime_signal(d, i) {
                if i+1 < n {
                    let entry_price = d.opens[i+1];
                    if entry_price > 0.0 {
                        pos = Some(ScalpPos {
                            dir,
                            entry: entry_price,
                            notional: bal * POSITION_SIZE,
                            bars_held: 0,
                        });
                    }
                }
            }
        }
    }

    let total_trades = win_pnls.len() + loss_pnls.len() + flats;
    ProfileResult {
        pnl: bal - INITIAL_BAL,
        trades: total_trades,
        wins: win_pnls.len(),
        losses: loss_pnls.len(),
        flats,
        session_wins,
        session_losses,
        session_trades,
        session_pnl,
    }
}

// ── Entry point ──────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN41 — Session-Based Trade Filter");
    eprintln!("Asia/Europe/US Conditional Engagement\n");

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
    eprintln!("\nGrid: {} configs × {} coins...", grid.len(), N_COINS);
    eprintln!("Sessions: Asia(0-8 UTC), Europe(9-16 UTC), US(17-23 UTC)\n");

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    // First pass: profile baseline per-session for each coin
    eprintln!("=== Session Profile (Baseline) ===");
    let session_labels = ["Asia", "Europe", "US"];

    let baseline_profiles: Vec<(String, [f64;3], [usize;3], [usize;3], [usize;3])> = coin_data.iter().map(|d| {
        let r = profile_sessions(d, SessionCfg { mode: 0 });
        let mut all_w = [0usize; 3];
        let mut all_l = [0usize; 3];
        let mut all_t = [0usize; 3];
        let mut all_p = [0.0f64; 3];
        for j in 0..3 {
            all_w[j] = r.session_wins[j] + r.session_losses[j];
            all_l[j] = r.session_losses[j];
            all_t[j] = r.session_trades[j];
            all_p[j] = r.session_pnl[j];
        }
        (d.name.to_string(), all_p, all_t, all_l, all_w)
    }).collect();

    for (_, (name, pnl_s, _trades_s, losses_s, total_s)) in baseline_profiles.iter().enumerate() {
        print!("  {}: ", name);
        for j in 0..3 {
            let wr = if total_s[j] > 0 { (total_s[j] - losses_s[j]) as f64 / total_s[j] as f64 * 100.0 } else { 0.0 };
            eprint!("{}={:.0}trades WR={:.0}% PnL={:+.1}  ", session_labels[j], total_s[j], wr, pnl_s[j]);
        }
        eprintln!();
    }

    if shutdown.load(Ordering::SeqCst) { return; }

    // Grid search
    eprintln!("\n=== Grid Search ===");
    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, total_delta: 0.0, portfolio_wr: 0.0,
                total_trades: 0, is_baseline: cfg.mode == 0, coins: vec![],
            };
        }

        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let r = profile_sessions(d, *cfg);
            let baseline_idx = coin_data.iter().position(|x| x.name == d.name).unwrap_or(0);
            let (_, base_pnl_s, base_trades_s, _, _) = &baseline_profiles[baseline_idx];

            let total_trades = r.trades;
            let wins_sum = r.wins;
            let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };

            // Per-session breakdown
            let mut session_stats = Vec::new();
            for j in 0..3 {
                let sess_trades = r.session_trades[j];
                let sess_wins = r.session_wins[j];
                let sess_losses = r.session_losses[j];
                let sess_wr = if sess_trades > 0 { sess_wins as f64 / sess_trades as f64 * 100.0 } else { 0.0 };
                let avg_win = if sess_wins > 0 { r.session_pnl[j] / sess_wins as f64 } else { 0.0 };
                let avg_loss = if sess_losses > 0 { r.session_pnl[j] / sess_losses as f64 } else { 0.0 };
                let pf = if avg_loss.abs() > 1e-8 { avg_win / avg_loss.abs() } else { 0.0 };
                session_stats.push(SessionStats {
                    session: session_labels[j].to_string(),
                    trades: sess_trades,
                    wins: sess_wins,
                    losses: sess_losses,
                    flats: 0,
                    wr: sess_wr,
                    pnl: r.session_pnl[j],
                    pf,
                    avg_win,
                    avg_loss,
                });
            }

            CoinResult {
                coin: d.name.to_string(),
                baseline_pnl: base_pnl_s.iter().sum(),
                baseline_trades: base_trades_s.iter().sum(),
                baseline_wr: 0.0, // placeholder — computed from session data
                filtered_pnl: r.pnl,
                filtered_trades: r.trades,
                filtered_wr: portfolio_wr,
                filtered_wins: r.wins,
                filtered_losses: r.losses,
                delta_pnl: r.pnl - base_pnl_s.iter().sum::<f64>(),
                session_stats,
            }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.filtered_pnl).sum();
        let baseline_total: f64 = coin_results.iter().map(|c| c.baseline_pnl).sum();
        let total_delta = total_pnl - baseline_total;
        let total_trades: usize = coin_results.iter().map(|c| c.filtered_trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.filtered_wins).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let is_baseline = cfg.mode == 0;

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {:<12} PnL={:>+8.2} Δ={:>+7.2} WR={:>5.1}% trades={}",
            d, total_cfgs, cfg.label(), total_pnl, total_delta, portfolio_wr, total_trades);

        ConfigResult { label: cfg.label(), total_pnl, total_delta, portfolio_wr, total_trades, is_baseline, coins: coin_results }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Interrupted — saving partial...");
        let output = Output { notes: "RUN41 interrupted".to_string(), configs: results };
        std::fs::write("/home/scamarena/ProjectCoin/run41_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
        return;
    }

    eprintln!();
    let baseline = results.iter().find(|r| r.is_baseline).unwrap();

    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_delta.partial_cmp(&a.total_delta).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN41 Session-Based Trade Filter Results ===");
    println!("Baseline (no filter): PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<12} {:>8} {:>8} {:>8} {:>8}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades");
    println!("{}", "-".repeat(60));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.total_delta;
        println!("{:>3}  {:<12} {:>+8.2} {:>+8.2} {:>6.1}% {:>6}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades);
    }
    println!("{}", "=".repeat(60));

    let best = sorted.first().unwrap();
    let is_positive = best.total_delta > 0.0;
    println!("\nVERDICT: {} (best ΔPnL={:+.2})",
        if is_positive { "POSITIVE" } else { "NEGATIVE" },
        best.total_delta);

    let notes = format!("RUN41 session filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_delta);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run41_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run41_1_results.json");
}
