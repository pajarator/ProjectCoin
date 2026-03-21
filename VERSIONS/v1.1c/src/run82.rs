/// RUN82 — Regime Decay Detection
///
/// Early exit when ADX rises significantly above entry ADX or regime shifts to StrongTrend:
/// - LONG in StrongTrend → exit (mean reversion invalid)
/// - ADX rise >= ADX_RISE above entry ADX → exit
///
/// Grid: ADX_RISE [10.0, 15.0, 20.0, 25.0] × REGIME_SHIFT [true, false] × GRACE [3, 5, 10]
/// Total: 4 × 2 × 3 = 24 + baseline = 25 configs
///
/// Run: cargo run --release --features run82 -- --run82

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
struct DecayCfg {
    adx_rise: f64,
    regime_shift: bool,
    grace: usize,
    is_baseline: bool,
}

impl DecayCfg {
    fn label(&self) -> String {
        if self.is_baseline { "BASELINE".to_string() }
        else {
            let rs = if self.regime_shift { "T" } else { "F" };
            format!("AR{:.0}_RS{}_G{}", self.adx_rise, rs, self.grace)
        }
    }
}

fn build_grid() -> Vec<DecayCfg> {
    let mut grid = vec![DecayCfg { adx_rise: 0.0, regime_shift: false, grace: 0, is_baseline: true }];
    let adx_rises = [10.0, 15.0, 20.0, 25.0];
    let regimes = [true, false];
    let graces = [3usize, 5, 10];
    for &ar in &adx_rises {
        for &rs in &regimes {
            for &g in &graces {
                grid.push(DecayCfg { adx_rise: ar, regime_shift: rs, grace: g, is_baseline: false });
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
    regime: Vec<i8>, // 0=ranging/weak, 1=strong_trend
}

#[derive(Serialize)]
struct CoinResult { coin: String, pnl: f64, trades: usize, wins: usize, losses: usize, wr: f64, decay_exits: usize, decay_pnl: f64 }
#[derive(Serialize)]
struct ConfigResult {
    label: String, total_pnl: f64, portfolio_wr: f64, total_trades: usize,
    pf: f64, is_baseline: bool, decay_exit_rate: f64,
    decay_exit_pnl_sum: f64, coins: Vec<CoinResult>,
}
#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

fn truerange(h: f64, l: f64, pc: f64) -> f64 {
    (h - l).max((h - pc).abs()).max((l - pc).abs())
}

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
        if oo.is_nan() || cc.is_nan() || hh.is_nan() || ll.is_nan() { continue; }
        opens.push(oo); closes.push(cc); highs.push(hh); lows.push(ll);
    }
    if closes.len() < 50 { return None; }
    let n = closes.len();

    // Z-score
    let mut zscore = vec![f64::NAN; n];
    for i in 20..n {
        let window = &closes[i+1-20..=i];
        let mean = window.iter().sum::<f64>()/20.0;
        let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
        zscore[i] = if std > 0.0 { (closes[i] - mean) / std } else { 0.0 };
    }

    // ADX(14) using Wilder smoothing
    let mut tr = vec![f64::NAN; n];
    let mut dm_plus = vec![f64::NAN; n];
    let mut dm_minus = vec![f64::NAN; n];
    for i in 1..n {
        tr[i] = truerange(highs[i], lows[i], closes[i-1]);
        let up_move = highs[i] - highs[i-1];
        let down_move = lows[i-1] - lows[i];
        if up_move > down_move && up_move > 0.0 { dm_plus[i] = up_move; dm_minus[i] = 0.0; }
        else if down_move > up_move && down_move > 0.0 { dm_plus[i] = 0.0; dm_minus[i] = down_move; }
        else { dm_plus[i] = 0.0; dm_minus[i] = 0.0; }
    }

    // Wilder smooth with alpha = 1/14
    let period = 14;
    let mut adx = vec![f64::NAN; n];
    let mut dx = vec![f64::NAN; n];

    // Initial smoothed values at index period
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
        adx[period] = dx[period]; // first ADX = DX
    }

    // Continue with Wilder smoothing
    for i in (period+1)..n {
        smooth_tr = smooth_tr - smooth_tr / period as f64 + tr[i];
        smooth_dm_plus = smooth_dm_plus - smooth_dm_plus / period as f64 + dm_plus[i];
        smooth_dm_minus = smooth_dm_minus - smooth_dm_minus / period as f64 + dm_minus[i];

        let di_plus = if smooth_tr > 0.0 { (smooth_dm_plus / smooth_tr) * 100.0 } else { 0.0 };
        let di_minus = if smooth_tr > 0.0 { (smooth_dm_minus / smooth_tr) * 100.0 } else { 0.0 };
        let di_sum = di_plus + di_minus;
        dx[i] = if di_sum > 0.0 { (di_plus - di_minus).abs() / di_sum * 100.0 } else { 0.0 };

        // ADX = Wilder smoothed DX
        adx[i] = adx[i-1] - adx[i-1] / period as f64 + dx[i];
    }

    // Regime: ADX >= 25 = strong_trend (1), else ranging/weak (0)
    let mut regime = vec![0i8; n];
    for i in period..n {
        regime[i] = if adx[i] >= 25.0 { 1 } else { 0 };
    }

    Some(CoinData15m { name: coin.to_string(), closes, opens, highs, lows, zscore, adx, regime })
}

fn regime_signal(z: f64) -> Option<i8> {
    if z.is_nan() { return None; }
    if z < -2.0 { return Some(1); }
    if z > 2.0 { return Some(-1); }
    None
}

fn simulate(d: &CoinData15m, cfg: &DecayCfg) -> (f64, usize, usize, usize, usize, f64) {
    let n = d.closes.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<(i8, f64, usize, f64)> = None; // dir, entry_price, entry_bar, entry_adx
    let mut cooldown = 0usize;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut flats = 0usize;
    let mut decay_exits = 0usize;
    let mut decay_pnl_sum = 0.0;

    for i in 1..n {
        if let Some((dir, entry, entry_bar, entry_adx)) = pos {
            let pct = if dir == 1 { (d.closes[i]-entry)/entry } else { (entry-d.closes[i])/entry };
            let mut closed = false;
            let mut exit_pct = 0.0;
            let mut exit_reason = "";

            // SL check
            if pct <= -SL_PCT { exit_pct = -SL_PCT; closed = true; exit_reason = "SL"; }

            // Regime decay check (before SMA/Z0 check)
            if !closed && cfg.is_baseline == false {
                let bars_held = i - entry_bar;
                if bars_held >= cfg.grace {
                    // ADX rise check
                    if cfg.adx_rise > 0.0 && !d.adx[i].is_nan() && !entry_adx.is_nan() {
                        if d.adx[i] >= entry_adx + cfg.adx_rise {
                            exit_pct = pct; closed = true; exit_reason = "DECAY_ADX";
                        }
                    }
                    // Regime shift check (entered in non-strongtrend, now in strongtrend)
                    if !closed && cfg.regime_shift {
                        let entry_was_strong = if dir == 1 {
                            // For LONG: check if at entry the market was strong trend
                            // Actually regime[i] is current, but we need entry regime
                            // We stored entry_adx; use ADX at entry_bar as proxy
                            // ADX >= 25 at entry = strong trend already
                            // If entry ADX < 25 and current >= 25, regime shifted
                            if !entry_adx.is_nan() && entry_adx < 25.0 && d.adx[i] >= 25.0 {
                                true
                            } else { false }
                        } else {
                            if !entry_adx.is_nan() && entry_adx < 25.0 && d.adx[i] >= 25.0 {
                                true
                            } else { false }
                        };
                        // For shorts: if market shifts TO strong trend, that hurts shorts too
                        // Actually for SHORT, strong trend means downtrend - which is GOOD for shorts
                        // Wait - let me re-read the hypothesis:
                        // "LONG in StrongTrend AND position is SHORT → regime is decaying against us"
                        // The hypothesis says: if you're SHORT and regime becomes StrongTrend (downtrend),
                        // that's actually GOOD for shorts. So "decay" for shorts means the opposite -
                        // regime shifts to uptrend (WeakTrend/Ranging). But the original is unclear.
                        // Let me interpret: StrongTrend means the existing trend continues.
                        // For SHORT: if StrongTrend = downtrend, that's good. If uptrend, bad.
                        // But ADX doesn't tell us direction, only strength.
                        // The original says: "position is SHORT → regime is decaying against us"
                        // This suggests: for shorts, regime decay = strong downtrend stops (which is bad for shorts? no)
                        // I think there's a confusion in the hypothesis.
                        // Let me simplify: DECAY = position direction opposite to current strong trend.
                        // LONG in strong uptrend = decay; SHORT in strong downtrend = decay
                        // Since ADX only measures trend STRENGTH not direction, we use the z-score regime.
                        // z > 0 = uptrend bias; z < 0 = downtrend bias
                        // For LONG: if z > 1.0 and ADX strong = uptrend = bad for LONG
                        // For SHORT: if z < -1.0 and ADX strong = downtrend = bad for SHORT
                        if entry_was_strong {
                            // Check if we're now in a trend that hurts our position
                            let current_z = d.zscore[i];
                            if (dir == 1 && current_z > 1.0 && d.regime[i] == 1) ||
                               (dir == -1 && current_z < -1.0 && d.regime[i] == 1) {
                                exit_pct = pct; closed = true; exit_reason = "DECAY_REG";
                            }
                        }
                    }
                }
            }

            // SMA/Z0 exit
            if !closed {
                let new_dir = regime_signal(d.zscore[i]);
                if new_dir.is_some() && new_dir != Some(dir) { exit_pct = pct; closed = true; exit_reason = "SMA"; }
            }
            if !closed && i >= n - 1 { exit_pct = pct; closed = true; exit_reason = "END"; }

            if closed {
                let net = bal * POSITION_SIZE * LEVERAGE * exit_pct;
                bal += net;
                if net > 1e-10 { wins += 1; }
                else if net < -1e-10 { losses += 1; }
                else { flats += 1; }
                if exit_reason == "DECAY_ADX" || exit_reason == "DECAY_REG" {
                    decay_exits += 1;
                    decay_pnl_sum += net;
                }
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
                        let entry_adx = if !d.adx[i].is_nan() { d.adx[i] } else { 20.0 };
                        pos = Some((dir, entry_price, i, entry_adx));
                    }
                }
            }
        }
    }
    (bal - INITIAL_BAL, wins, losses, flats, decay_exits, decay_pnl_sum)
}

pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN82 — Regime Decay Detection\n");
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
    eprintln!("\nGrid: {} configs × {} coins", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0, total_trades: 0,
                pf: 0.0, is_baseline: cfg.is_baseline, decay_exit_rate: 0.0,
                decay_exit_pnl_sum: 0.0, coins: vec![],
            };
        }
        let coin_results: Vec<CoinResult> = coin_data.iter().map(|d| {
            let (pnl, wins, losses, flats, decay_exits, decay_pnl) = simulate(d, cfg);
            let trades = wins + losses + flats;
            let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
            CoinResult { coin: d.name.clone(), pnl, trades, wins, losses, wr, decay_exits, decay_pnl }
        }).collect();

        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let decay_exits_sum: usize = coin_results.iter().map(|c| c.decay_exits).sum();
        let decay_pnl_sum: f64 = coin_results.iter().map(|c| c.decay_pnl).sum();
        let portfolio_wr = if total_trades > 0 { wins_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };
        let gross = wins_sum as f64 * SL_PCT * POSITION_SIZE * LEVERAGE;
        let losses_f = coin_results.iter().map(|c| c.losses).sum::<usize>() as f64;
        let pf = if losses_f > 0.0 { gross / (losses_f * SL_PCT * POSITION_SIZE * LEVERAGE) } else { 0.0 };
        let decay_exit_rate = if total_trades > 0 { decay_exits_sum as f64 / total_trades as f64 * 100.0 } else { 0.0 };

        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>3}/{}] {}  PnL={:>+8.2}  WR={:>5.1}%  trades={}  decay={:.1}%",
            d, total_cfgs, cfg.label(), total_pnl, portfolio_wr, total_trades, decay_exit_rate);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades,
            pf, is_baseline: cfg.is_baseline, decay_exit_rate,
            decay_exit_pnl_sum: decay_pnl_sum, coins: coin_results,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) { return; }

    let baseline = results.iter().find(|r| r.is_baseline).unwrap();
    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN82 Regime Decay Detection Results ===");
    println!("Baseline: PnL={:+.2}  WR={:.1}%  Trades={}", baseline.total_pnl, baseline.portfolio_wr, baseline.total_trades);
    println!("\n{:>3}  {:<16} {:>8} {:>8} {:>6} {:>7} {:>8} {:>9}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "Trades", "PF", "Decay%");
    println!("{}", "-".repeat(70));
    for (i, r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<16} {:>+8.2} {:>+8.2} {:>5.1}%  {:>6} {:>8.2} {:>8.1}%",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.total_trades, r.pf, r.decay_exit_rate);
    }
    println!("{}", "=".repeat(70));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {}", if is_positive { "POSITIVE" } else { "NEGATIVE" });

    let notes = format!("RUN82 regime decay. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:+.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl - baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run82_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run82_1_results.json");
}
