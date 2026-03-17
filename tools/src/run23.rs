/// RUN23 — Differential Evolution Parameter Optimization
///
/// Optimizes parameters of 3 strategy templates via DE on train (67%)
/// then evaluates best params vs default (midpoint) params on OOS test (33%).
///
/// Fixes over Python stub:
///   - atr_mult was in volatility_breakout bounds but never used; removed
///   - LCG RNG for exact reproducibility (Python used random seed=42)
///
/// Strategy templates:
///   mean_reversion:      rsi_period(7-21) rsi_lo(20-35) rsi_hi(65-80) bb_std(1.5-3.0)
///   momentum:            rsi_period(7-21) rsi_lo(40-60) stoch_lo(20-40) stoch_hi(60-80)
///   volatility_breakout: bb_std(1.5-3.0) vol_thresh(1.0-3.0)
///
/// DE params: pop=15×dims, gens=100, F=0.8, CR=0.7
/// Trade sim: SL=0.3% fee=0.1%/side slip=0.05%/side
/// Split: 67% train / 33% test

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::indicators::{bollinger, rsi as rsi_ind, rolling_max, rolling_mean, rolling_min};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};

const SL:   f64 = 0.003;
const FEE:  f64 = 0.001;
const SLIP: f64 = 0.0005;

// ── LCG RNG ───────────────────────────────────────────────────────────────────
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Rng(seed) }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn f64(&mut self) -> f64 { (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 }
    fn usize(&mut self, n: usize) -> usize { (self.next_u64() % n as u64) as usize }
}

// ── Trade simulation ──────────────────────────────────────────────────────────
fn sim(close: &[f64], entry: &[bool], exit: &[bool]) -> (usize, f64, f64, f64) {
    // (n_trades, win_rate%, profit_factor, total_pnl%)
    let n = close.len();
    let mut pnls: Vec<f64> = Vec::new();
    let mut in_pos = false;
    let mut ep = 0.0f64;

    for i in 0..n {
        if in_pos {
            let ret = (close[i] - ep) / ep;
            if ret <= -SL {
                let ex = ep * (1.0 - SL * (1.0 + SLIP));
                pnls.push((ex - ep) / ep * 100.0 - FEE * 200.0);
                in_pos = false;
            } else if exit[i] {
                let ex = close[i] * (1.0 - SLIP);
                pnls.push((ex - ep) / ep * 100.0 - FEE * 200.0);
                in_pos = false;
            }
        } else if entry[i] {
            ep = close[i] * (1.0 + SLIP);
            in_pos = true;
        }
    }
    if in_pos {
        let ex = *close.last().unwrap();
        pnls.push((ex - ep) / ep * 100.0 - FEE * 200.0);
    }

    let nt = pnls.len();
    if nt == 0 { return (0, 0.0, 0.0, 0.0); }
    let wins: Vec<f64>   = pnls.iter().cloned().filter(|&p| p > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().cloned().filter(|&p| p <= 0.0).map(|p| -p).collect();
    let wr = wins.len() as f64 / nt as f64 * 100.0;
    let gw: f64 = wins.iter().sum();
    let gl: f64 = losses.iter().sum();
    let pf = if gl > 0.0 { gw / gl } else { gw };
    let mut eq = 10_000.0f64;
    for &p in &pnls { eq *= 1.0 + p / 100.0; }
    let total_pnl = (eq - 10_000.0) / 10_000.0 * 100.0;
    (nt, wr, pf, total_pnl)
}

fn fitness_score(close: &[f64], entry: &[bool], exit: &[bool]) -> f64 {
    let (nt, _wr, _pf, _pnl) = sim(close, entry, exit);
    if nt < 8 { return -999.0; }
    // Sharpe × sqrt(trades) — use trade P&L std as proxy
    let mut pnls: Vec<f64> = Vec::new();
    let mut in_pos = false;
    let mut ep = 0.0f64;
    for i in 0..close.len() {
        if in_pos {
            let ret = (close[i] - ep) / ep;
            if ret <= -SL {
                let ex = ep * (1.0 - SL * (1.0 + SLIP));
                pnls.push((ex - ep) / ep * 100.0 - FEE * 200.0);
                in_pos = false;
            } else if exit[i] {
                let ex = close[i] * (1.0 - SLIP);
                pnls.push((ex - ep) / ep * 100.0 - FEE * 200.0);
                in_pos = false;
            }
        } else if entry[i] {
            ep = close[i] * (1.0 + SLIP);
            in_pos = true;
        }
    }
    let n = pnls.len() as f64;
    if n < 8.0 { return -999.0; }
    let mean = pnls.iter().sum::<f64>() / n;
    let std  = (pnls.iter().map(|&p| (p - mean).powi(2)).sum::<f64>() / n).sqrt();
    if std == 0.0 { return -999.0; }
    let sharpe = mean / std;
    sharpe * n.sqrt()
}

// ── Strategy signal generators ────────────────────────────────────────────────
fn sig_mean_rev(
    close: &[f64], high: &[f64], low: &[f64],
    rsi_p: usize, rsi_lo: f64, rsi_hi: f64, bb_std: f64,
) -> (Vec<bool>, Vec<bool>) {
    let n = close.len();
    let rsi14 = rsi_ind(close, rsi_p);
    let (_, bb_mid, bb_lower) = bollinger(close, 20, bb_std);
    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 0..n {
        if rsi14[i].is_nan() || bb_lower[i].is_nan() { continue; }
        entry[i] = rsi14[i] < rsi_lo && close[i] < bb_lower[i];
        exit[i]  = rsi14[i] > rsi_hi || close[i] > bb_mid[i];
    }
    (entry, exit)
}

fn sig_momentum(
    close: &[f64], high: &[f64], low: &[f64],
    rsi_p: usize, rsi_lo: f64, stoch_lo: f64, stoch_hi: f64,
) -> (Vec<bool>, Vec<bool>) {
    let n = close.len();
    let rsi14 = rsi_ind(close, rsi_p);
    let hh = rolling_max(high, 14);
    let ll = rolling_min(low, 14);
    let stoch_k: Vec<f64> = (0..n).map(|i| {
        let r = hh[i] - ll[i];
        if r > 0.0 { (close[i] - ll[i]) / r * 100.0 } else { f64::NAN }
    }).collect();
    // Stoch D = 3-bar SMA of K
    let mut stoch_d = vec![f64::NAN; n];
    for i in 2..n {
        if stoch_k[i].is_nan() || stoch_k[i-1].is_nan() || stoch_k[i-2].is_nan() { continue; }
        stoch_d[i] = (stoch_k[i] + stoch_k[i-1] + stoch_k[i-2]) / 3.0;
    }
    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 0..n {
        if rsi14[i].is_nan() || stoch_k[i].is_nan() || stoch_d[i].is_nan() { continue; }
        entry[i] = rsi14[i] > rsi_lo && stoch_k[i] > stoch_lo && stoch_k[i] > stoch_d[i];
        exit[i]  = rsi14[i] < 50.0 || stoch_k[i] < stoch_hi;
    }
    (entry, exit)
}

fn sig_vol_breakout(
    close: &[f64], _high: &[f64], _low: &[f64], vol: &[f64],
    bb_std: f64, vol_thresh: f64,
) -> (Vec<bool>, Vec<bool>) {
    let n = close.len();
    let (bb_upper, bb_mid, _) = bollinger(close, 20, bb_std);
    let vol_ma = rolling_mean(vol, 20);
    let mut entry = vec![false; n];
    let mut exit  = vec![false; n];
    for i in 0..n {
        if bb_upper[i].is_nan() || vol_ma[i] <= 0.0 { continue; }
        let vr = vol[i] / vol_ma[i];
        entry[i] = close[i] > bb_upper[i] && vr > vol_thresh;
        exit[i]  = close[i] < bb_mid[i];
    }
    (entry, exit)
}

// ── Differential Evolution ────────────────────────────────────────────────────
fn de_optimize<F: Fn(&[f64]) -> f64>(
    objective: F,          // maximise
    bounds: &[(f64, f64)],
    n_gen: usize,
    f_mut: f64,
    cr: f64,
    rng: &mut Rng,
) -> Vec<f64> {
    let d = bounds.len();
    let np = 15 * d;  // standard: 15 × dimensions

    // Initialize population uniformly in bounds
    let mut pop: Vec<Vec<f64>> = (0..np).map(|_| {
        bounds.iter().map(|&(lo, hi)| lo + rng.f64() * (hi - lo)).collect()
    }).collect();
    let mut fits: Vec<f64> = pop.iter().map(|x| objective(x)).collect();

    for _gen in 0..n_gen {
        for i in 0..np {
            // Select 3 distinct indices ≠ i
            let mut r = [0usize; 3];
            let mut used = vec![i];
            for k in 0..3 {
                loop {
                    let idx = rng.usize(np);
                    if !used.contains(&idx) { r[k] = idx; used.push(idx); break; }
                }
            }
            // Mutant vector
            let mutant: Vec<f64> = (0..d).map(|j| {
                let v = pop[r[0]][j] + f_mut * (pop[r[1]][j] - pop[r[2]][j]);
                v.clamp(bounds[j].0, bounds[j].1)
            }).collect();
            // Crossover
            let j_rand = rng.usize(d);
            let trial: Vec<f64> = (0..d).map(|j| {
                if j == j_rand || rng.f64() < cr { mutant[j] } else { pop[i][j] }
            }).collect();
            // Selection
            let trial_fit = objective(&trial);
            if trial_fit > fits[i] {
                pop[i] = trial;
                fits[i] = trial_fit;
            }
        }
    }

    // Return best
    let best_i = fits.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
    pop[best_i].clone()
}

// ── Per-coin processing ───────────────────────────────────────────────────────
#[derive(Debug)]
struct StratResult {
    name:       &'static str,
    de_params:  Vec<(String, f64)>,
    de:         (usize, f64, f64, f64),   // (trades, wr, pf, pnl)
    def:        (usize, f64, f64, f64),
}

fn process_coin(coin: &str, coin_idx: usize) -> Vec<StratResult> {
    let bars  = load_ohlcv(coin);
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
    let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();
    let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();
    let n = close.len();
    let split = n * 2 / 3;

    let tr_c = &close[..split]; let tr_h = &high[..split];
    let tr_l = &low[..split];   let tr_v = &vol[..split];
    let te_c = &close[split..]; let te_h = &high[split..];
    let te_l = &low[split..];   let te_v = &vol[split..];

    let mut rng = Rng::new(coin_idx as u64 * 0xdeadbeef_u64.wrapping_add(42));
    let mut results = Vec::new();

    // ── mean_reversion ───────────────────────────────────────────────────────
    {
        let bounds: Vec<(f64,f64)> = vec![(7.0,21.0),(20.0,35.0),(65.0,80.0),(1.5,3.0)];
        let def_params: Vec<f64> = bounds.iter().map(|&(lo,hi)| (lo+hi)/2.0).collect();

        let best = de_optimize(|p| {
            let (e, x) = sig_mean_rev(tr_c, tr_h, tr_l, p[0].round() as usize, p[1], p[2], p[3]);
            fitness_score(tr_c, &e, &x)
        }, &bounds, 100, 0.8, 0.7, &mut rng);

        let (be, bx) = sig_mean_rev(te_c, te_h, te_l, best[0].round() as usize, best[1], best[2], best[3]);
        let de_res   = sim(te_c, &be, &bx);
        let (de2, dx2) = sig_mean_rev(te_c, te_h, te_l, def_params[0].round() as usize, def_params[1], def_params[2], def_params[3]);
        let def_res  = sim(te_c, &de2, &dx2);

        results.push(StratResult {
            name: "mean_reversion",
            de_params: vec![
                ("rsi_period".into(), best[0].round()),
                ("rsi_lo".into(), best[1]),
                ("rsi_hi".into(), best[2]),
                ("bb_std".into(), best[3]),
            ],
            de: de_res, def: def_res,
        });
    }

    // ── momentum ─────────────────────────────────────────────────────────────
    {
        let bounds: Vec<(f64,f64)> = vec![(7.0,21.0),(40.0,60.0),(20.0,40.0),(60.0,80.0)];
        let def_params: Vec<f64> = bounds.iter().map(|&(lo,hi)| (lo+hi)/2.0).collect();

        let best = de_optimize(|p| {
            let (e, x) = sig_momentum(tr_c, tr_h, tr_l, p[0].round() as usize, p[1], p[2], p[3]);
            fitness_score(tr_c, &e, &x)
        }, &bounds, 100, 0.8, 0.7, &mut rng);

        let (be, bx) = sig_momentum(te_c, te_h, te_l, best[0].round() as usize, best[1], best[2], best[3]);
        let de_res   = sim(te_c, &be, &bx);
        let (de2, dx2) = sig_momentum(te_c, te_h, te_l, def_params[0].round() as usize, def_params[1], def_params[2], def_params[3]);
        let def_res  = sim(te_c, &de2, &dx2);

        results.push(StratResult {
            name: "momentum",
            de_params: vec![
                ("rsi_period".into(), best[0].round()),
                ("rsi_lo".into(), best[1]),
                ("stoch_lo".into(), best[2]),
                ("stoch_hi".into(), best[3]),
            ],
            de: de_res, def: def_res,
        });
    }

    // ── volatility_breakout (fixed: removed unused atr_mult) ─────────────────
    {
        let bounds: Vec<(f64,f64)> = vec![(1.5,3.0),(1.0,3.0)];
        let def_params: Vec<f64> = bounds.iter().map(|&(lo,hi)| (lo+hi)/2.0).collect();

        let best = de_optimize(|p| {
            let (e, x) = sig_vol_breakout(tr_c, tr_h, tr_l, tr_v, p[0], p[1]);
            fitness_score(tr_c, &e, &x)
        }, &bounds, 100, 0.8, 0.7, &mut rng);

        let (be, bx) = sig_vol_breakout(te_c, te_h, te_l, te_v, best[0], best[1]);
        let de_res   = sim(te_c, &be, &bx);
        let (de2, dx2) = sig_vol_breakout(te_c, te_h, te_l, te_v, def_params[0], def_params[1]);
        let def_res  = sim(te_c, &de2, &dx2);

        results.push(StratResult {
            name: "volatility_breakout",
            de_params: vec![
                ("bb_std".into(), best[0]),
                ("vol_thresh".into(), best[1]),
            ],
            de: de_res, def: def_res,
        });
    }

    results
}

// ── Main ──────────────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN23 — Differential Evolution Parameter Optimization");
    println!("================================================================");
    println!("Fix: removed unused atr_mult param from volatility_breakout bounds.");
    println!("Strategies: mean_reversion | momentum | volatility_breakout");
    println!("DE: pop=15×dims gens=100 F=0.8 CR=0.7 | fitness on train (67%)");
    println!("Compare DE-optimized vs default (midpoint) params on OOS test (33%)");
    println!("Trade: SL=0.3% fee=0.1%/side slip=0.05%/side | Breakeven WR ≈ 44%");
    println!();

    let coins: Vec<(&str, &str)> = COIN_STRATEGIES.to_vec();

    let all_results: Vec<(&str, Vec<StratResult>)> = coins.par_iter()
        .enumerate()
        .filter(|_| !shutdown.load(Ordering::SeqCst))
        .map(|(idx, (coin, strat))| (*coin, process_coin(coin, idx)))
        .collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Shutdown before completion.");
        return;
    }

    let mut sorted = all_results;
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    // ── Results by strategy ────────────────────────────────────────────────────
    for strat_name in ["mean_reversion", "momentum", "volatility_breakout"] {
        println!("  Strategy: {}", strat_name);
        println!("  {:<6}  {:>6}  {:>7}  {:>6}  {:>8}  |  {:>6}  {:>7}  {:>6}  {:>8}  {:>8}",
            "Coin", "DEt", "DEWR%", "DEPF", "DEP&L%",
            "Deft", "DefWR%", "DefPF", "DefP&L%", "PFdelta");
        println!("  {}", "-".repeat(78));

        let mut pf_deltas: Vec<f64> = Vec::new();
        let mut de_better = 0usize;

        for (coin, strats) in &sorted {
            let s = strats.iter().find(|s| s.name == strat_name);
            if let Some(s) = s {
                let (dt, dwr, dpf, dpnl) = s.de;
                let (ft, fwr, fpf, fpnl) = s.def;
                let delta = dpf - fpf;
                pf_deltas.push(delta);
                if delta > 0.0 && dt >= 5 { de_better += 1; }
                println!("  {:<6}  {:>6}  {:>7.1}  {:>6.2}  {:>8.1}  |  {:>6}  {:>7.1}  {:>6.2}  {:>8.1}  {:>8.2}",
                    coin, dt, dwr, dpf, dpnl, ft, fwr, fpf, fpnl, delta);
            }
        }

        let nc = pf_deltas.len() as f64;
        let avg_delta = pf_deltas.iter().sum::<f64>() / nc;
        println!("  Avg PF delta: {:+.3}  DE better (PF+, ≥5t): {}/{}",
            avg_delta, de_better, sorted.len());
        println!();
    }

    // ── Portfolio-level WR analysis ────────────────────────────────────────────
    println!("  OOS WR% across all strategies (breakeven = 44%):");
    println!("  {:<6}  {:>14}  {:>14}  {:>18}",
        "Coin", "MeanRev WR%", "Momentum WR%", "VolBreakout WR%");
    println!("  {}", "-".repeat(60));
    for (coin, strats) in &sorted {
        let wr = |nm: &str| strats.iter().find(|s| s.name == nm)
            .map(|s| s.de.1).unwrap_or(0.0);
        println!("  {:<6}  {:>14.1}  {:>14.1}  {:>18.1}",
            coin, wr("mean_reversion"), wr("momentum"), wr("volatility_breakout"));
    }

    let total_above44 = sorted.iter().flat_map(|(_, strats)| strats.iter())
        .filter(|s| s.de.1 > 44.0 && s.de.0 >= 10).count();
    let total_cases = sorted.len() * 3;
    println!();
    println!("  WR > 44% with ≥10 trades: {}/{} ({:.1}%)",
        total_above44, total_cases, total_above44 as f64 / total_cases as f64 * 100.0);

    // ── Save ───────────────────────────────────────────────────────────────────
    let per_coin: Vec<Value> = sorted.iter().map(|(coin, strats)| {
        let strat_vals: Vec<Value> = strats.iter().map(|s| {
            let (dt, dwr, dpf, dpnl) = s.de;
            let (ft, fwr, fpf, fpnl) = s.def;
            json!({
                "strategy": s.name,
                "de_params": s.de_params.iter().map(|(k,v)| json!({"param": k, "value": v})).collect::<Vec<_>>(),
                "de_test": {"trades": dt, "win_rate": dwr, "profit_factor": dpf, "total_pnl": dpnl},
                "default_test": {"trades": ft, "win_rate": fwr, "profit_factor": fpf, "total_pnl": fpnl},
                "pf_delta": dpf - fpf,
                "wr_delta": dwr - fwr,
            })
        }).collect();
        json!({"coin": coin, "strategies": strat_vals})
    }).collect();

    let out = json!({ "per_coin": per_coin });
    let out_path = "archive/RUN23/run23_results.json";
    std::fs::create_dir_all("archive/RUN23").ok();
    std::fs::write(out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    println!();
    println!("  Results saved to {}", out_path);
}
