/// RUN22 — Genetic Algorithm v2 for Strategy Discovery
///
/// Evolves entry/exit rule combinations using 8 technical indicators.
/// Key fix over Python stub: fitness evaluated on TRAIN half only;
/// best genome is then evaluated on held-out TEST half (true OOS).
///
/// Genome: 1–3 entry rules (AND) + 1–2 exit rules (OR) + fixed SL=0.3%
/// Indicators: RSI14 | z20 | BB-position | vol-ratio | ROC5 | StochK | ADX14 | MACD-hist
/// Operators:  < | >
///
/// GA params: pop=80, gen=40, tournament=5, elite=5, mutate=20%, crossover=70%
/// Trade sim: SL=0.3% fee=0.1%/side slip=0.05%/side no TP
/// Split: 67% train (8mo) / 33% test (4mo) — fitness on train, final eval on test

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::indicators::{atr, bollinger, ema, rolling_max, rolling_mean, rolling_min, rsi, z_score};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};

// ── Constants ─────────────────────────────────────────────────────────────────
const SL:   f64 = 0.003;   // 0.3% stop loss
const FEE:  f64 = 0.001;   // 0.1%/side
const SLIP: f64 = 0.0005;  // 0.05%/side

const POP:   usize = 80;
const GENS:  usize = 40;
const TOURN: usize = 5;
const ELITE: usize = 5;
const MIN_TRADES_TRAIN: usize = 15; // minimum trades on train to get positive score

// LCG random number generator — reproducible per seed
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Rng(seed) }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn next_f64(&mut self) -> f64 { (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 }
    fn next_range(&mut self, lo: f64, hi: f64) -> f64 { lo + self.next_f64() * (hi - lo) }
    fn next_usize(&mut self, n: usize) -> usize { (self.next_u64() % n as u64) as usize }
    fn next_bool(&mut self, p: f64) -> bool { self.next_f64() < p }
}

// ── Feature definitions ───────────────────────────────────────────────────────
// feat_id → (name, lo, hi) — lo/hi = initialization range for thresholds
const FEATURES: [(&str, f64, f64); 8] = [
    ("RSI14",    20.0, 80.0),
    ("z20",      -2.5,  2.5),
    ("bb_pos",   -0.3,  1.3),
    ("vol_rat",   0.3,  4.0),
    ("ROC5",    -10.0, 10.0),
    ("StochK",   20.0, 80.0),
    ("ADX14",     5.0, 60.0),
    ("macd_h",   -3.0,  3.0),  // normalized by ATR
];
const N_FEATS: usize = 8;

// ── Feature matrix ────────────────────────────────────────────────────────────
struct FeatMat {
    n: usize,
    data: Vec<Vec<f64>>,  // [feat_id][bar_index]
}

impl FeatMat {
    fn build(bars: &[crate::loader::Bar]) -> Self {
        let n = bars.len();
        let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
        let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
        let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();
        let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();

        // RSI(14)
        let rsi14 = rsi(&close, 14);

        // z-score(20)
        let z20 = z_score(&close, 20);

        // BB position: (close - lower) / (upper - lower), clipped -0.5..1.5
        let (upper, _, lower) = bollinger(&close, 20, 2.0);
        let bb_pos: Vec<f64> = (0..n).map(|i| {
            let bw = upper[i] - lower[i];
            if bw > 0.0 { ((close[i] - lower[i]) / bw).clamp(-0.5, 1.5) } else { f64::NAN }
        }).collect();

        // Volume ratio: vol / rolling_mean(vol, 20)
        let vol_ma = rolling_mean(&vol, 20);
        let vol_rat: Vec<f64> = (0..n).map(|i| {
            if vol_ma[i] > 0.0 { vol[i] / vol_ma[i] } else { f64::NAN }
        }).collect();

        // ROC(5): (close[i] / close[i-5] - 1) * 100
        let mut roc5 = vec![f64::NAN; n];
        for i in 5..n {
            if close[i - 5] > 0.0 { roc5[i] = (close[i] / close[i - 5] - 1.0) * 100.0; }
        }

        // Stochastic K(14)
        let hh = rolling_max(&high, 14);
        let ll = rolling_min(&low, 14);
        let stoch_k: Vec<f64> = (0..n).map(|i| {
            let rng = hh[i] - ll[i];
            if rng > 0.0 { (close[i] - ll[i]) / rng * 100.0 } else { f64::NAN }
        }).collect();

        // ADX(14) via Wilder's RMA
        let alpha14 = 1.0 / 14.0;
        let mut adx14 = vec![f64::NAN; n];
        let mut pdm_s = 0.0f64; let mut mdm_s = 0.0f64;
        let mut tr_s  = 0.0f64; let mut adx_s = 0.0f64;
        let mut adx_init = 0usize;
        for i in 1..n {
            let up   = high[i] - high[i-1];
            let down = low[i-1] - low[i];
            let pdm = if up > down && up > 0.0 { up } else { 0.0 };
            let mdm = if down > up && down > 0.0 { down } else { 0.0 };
            let tr  = (high[i] - low[i])
                .max((high[i] - close[i-1]).abs())
                .max((low[i]  - close[i-1]).abs());
            if i == 1 { pdm_s = pdm; mdm_s = mdm; tr_s = tr; }
            else {
                pdm_s = pdm_s + alpha14 * (pdm - pdm_s);
                mdm_s = mdm_s + alpha14 * (mdm - mdm_s);
                tr_s  = tr_s  + alpha14 * (tr  - tr_s);
            }
            if tr_s > 0.0 {
                let pdi = 100.0 * pdm_s / tr_s;
                let mdi = 100.0 * mdm_s / tr_s;
                let dx  = if pdi + mdi > 0.0 { 100.0 * (pdi - mdi).abs() / (pdi + mdi) } else { 0.0 };
                if adx_init == 0 { adx_s = dx; adx_init = 1; }
                else { adx_s = adx_s + alpha14 * (dx - adx_s); }
                if i >= 14 { adx14[i] = adx_s; }
            }
        }

        // MACD histogram normalized by ATR(14)
        let atr14 = atr(&high, &low, &close, 14);
        let ema12 = ema(&close, 12);
        let ema26 = ema(&close, 26);
        let macd_line: Vec<f64> = (0..n).map(|i| {
            if ema12[i].is_nan() || ema26[i].is_nan() { f64::NAN } else { ema12[i] - ema26[i] }
        }).collect();
        let signal = ema(&macd_line, 9);
        let macd_h: Vec<f64> = (0..n).map(|i| {
            if macd_line[i].is_nan() || signal[i].is_nan() || atr14[i].is_nan() || atr14[i] == 0.0 {
                f64::NAN
            } else {
                ((macd_line[i] - signal[i]) / atr14[i]).clamp(-5.0, 5.0)
            }
        }).collect();

        FeatMat {
            n,
            data: vec![rsi14, z20, bb_pos, vol_rat, roc5, stoch_k, adx14, macd_h],
        }
    }

    fn get(&self, feat: usize, bar: usize) -> f64 { self.data[feat][bar] }
}

// ── Genome ────────────────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
struct Rule {
    feat: u8,   // 0..N_FEATS-1
    op:   u8,   // 0 = <, 1 = >
    thr:  f64,
}

#[derive(Clone, Debug)]
struct Genome {
    entry: Vec<Rule>,   // 1–3 rules, AND
    exit:  Vec<Rule>,   // 1–2 rules, OR
}

impl Genome {
    fn random(rng: &mut Rng) -> Self {
        let n_entry = 1 + rng.next_usize(3);
        let n_exit  = 1 + rng.next_usize(2);
        Genome {
            entry: (0..n_entry).map(|_| Rule::random(rng)).collect(),
            exit:  (0..n_exit).map(|_| Rule::random(rng)).collect(),
        }
    }
}

impl Rule {
    fn random(rng: &mut Rng) -> Self {
        let feat = rng.next_usize(N_FEATS) as u8;
        let op   = rng.next_usize(2) as u8;
        let (lo, hi) = (FEATURES[feat as usize].1, FEATURES[feat as usize].2);
        let thr  = rng.next_range(lo, hi);
        Rule { feat, op, thr }
    }

    fn matches(&self, fm: &FeatMat, i: usize) -> bool {
        let v = fm.get(self.feat as usize, i);
        if v.is_nan() { return false; }
        if self.op == 0 { v < self.thr } else { v > self.thr }
    }
}

// ── Signal generation ─────────────────────────────────────────────────────────
fn gen_signals(genome: &Genome, fm: &FeatMat, start: usize, end: usize) -> (Vec<bool>, Vec<bool>) {
    let len = end - start;
    let mut entry = vec![false; len];
    let mut exit  = vec![false; len];
    for (j, i) in (start..end).enumerate() {
        entry[j] = genome.entry.iter().all(|r| r.matches(fm, i));
        exit[j]  = genome.exit.iter().any(|r| r.matches(fm, i));
    }
    (entry, exit)
}

// ── Trade simulation ──────────────────────────────────────────────────────────
fn sim_trades(close: &[f64], entry: &[bool], exit: &[bool]) -> (usize, f64, f64) {
    // Returns: (n_trades, sharpe, total_pnl_pct)
    let n = close.len();
    let mut pnls: Vec<f64> = Vec::new();
    let mut in_pos = false;
    let mut ep = 0.0f64;

    for i in 0..n {
        if in_pos {
            let pnl_frac = (close[i] - ep) / ep;
            if pnl_frac <= -SL {
                let ex = ep * (1.0 - SL * (1.0 + SLIP));
                pnls.push((ex - ep) / ep * 100.0 - FEE * 2.0 * 100.0);
                in_pos = false;
            } else if exit[i] {
                let ex = close[i] * (1.0 - SLIP);
                pnls.push((ex - ep) / ep * 100.0 - FEE * 2.0 * 100.0);
                in_pos = false;
            }
        } else if entry[i] {
            ep = close[i] * (1.0 + SLIP);
            in_pos = true;
        }
    }
    if in_pos {
        let ex = *close.last().unwrap();
        pnls.push((ex - ep) / ep * 100.0 - FEE * 2.0 * 100.0);
    }

    let nt = pnls.len();
    if nt == 0 { return (0, -999.0, 0.0); }

    // Sharpe on equity returns
    let mut eq = 10_000.0f64;
    let mut rets = Vec::with_capacity(nt);
    for &p in &pnls {
        let prev = eq;
        eq *= 1.0 + p / 100.0;
        rets.push((eq - prev) / prev);
    }
    let mean_r = rets.iter().sum::<f64>() / nt as f64;
    let var_r  = rets.iter().map(|&r| (r - mean_r).powi(2)).sum::<f64>() / nt as f64;
    let sharpe = if var_r > 0.0 { mean_r / var_r.sqrt() * (nt as f64).sqrt() } else { 0.0 };

    let total_pnl = (eq - 10_000.0) / 10_000.0 * 100.0;
    (nt, sharpe, total_pnl)
}

fn fitness(genome: &Genome, fm: &FeatMat, close: &[f64], start: usize, end: usize) -> f64 {
    let (entry, exit) = gen_signals(genome, fm, start, end);
    let close_slice = &close[start..end];
    let (nt, sharpe, _) = sim_trades(close_slice, &entry, &exit);
    if nt < MIN_TRADES_TRAIN { return -999.0; }
    sharpe
}

// ── Mutation / Crossover ──────────────────────────────────────────────────────
fn mutate(g: &Genome, rng: &mut Rng) -> Genome {
    let mut g = g.clone();
    let choice = rng.next_usize(5);
    match choice {
        0 => {
            // Perturb a threshold
            let rules = if rng.next_bool(0.5) { &mut g.entry } else { &mut g.exit };
            if !rules.is_empty() {
                let idx = rng.next_usize(rules.len());
                let (lo, hi) = (FEATURES[rules[idx].feat as usize].1, FEATURES[rules[idx].feat as usize].2);
                let delta = rng.next_range(-(hi - lo) * 0.15, (hi - lo) * 0.15);
                rules[idx].thr = (rules[idx].thr + delta).clamp(lo, hi);
            }
        }
        1 => {
            // Replace a random rule
            let use_entry = rng.next_bool(0.6);
            let rules = if use_entry { &mut g.entry } else { &mut g.exit };
            if !rules.is_empty() {
                let idx = rng.next_usize(rules.len());
                rules[idx] = Rule::random(rng);
            }
        }
        2 => {
            // Add a rule (if room)
            if rng.next_bool(0.5) {
                if g.entry.len() < 3 { g.entry.push(Rule::random(rng)); }
            } else {
                if g.exit.len() < 2 { g.exit.push(Rule::random(rng)); }
            }
        }
        3 => {
            // Remove a rule (keep min 1)
            if rng.next_bool(0.5) {
                if g.entry.len() > 1 { let i = rng.next_usize(g.entry.len()); g.entry.remove(i); }
            } else {
                if g.exit.len() > 1 { let i = rng.next_usize(g.exit.len()); g.exit.remove(i); }
            }
        }
        _ => {
            // Flip operator on a rule
            let rules = if rng.next_bool(0.5) { &mut g.entry } else { &mut g.exit };
            if !rules.is_empty() {
                let idx = rng.next_usize(rules.len());
                rules[idx].op = 1 - rules[idx].op;
            }
        }
    }
    g
}

fn crossover(p1: &Genome, p2: &Genome, rng: &mut Rng) -> Genome {
    Genome {
        entry: if rng.next_bool(0.5) { p1.entry.clone() } else { p2.entry.clone() },
        exit:  if rng.next_bool(0.5) { p1.exit.clone()  } else { p2.exit.clone()  },
    }
}

fn tournament(pop: &[Genome], fits: &[f64], rng: &mut Rng) -> Genome {
    let mut best_i = rng.next_usize(pop.len());
    for _ in 1..TOURN {
        let i = rng.next_usize(pop.len());
        if fits[i] > fits[best_i] { best_i = i; }
    }
    pop[best_i].clone()
}

// ── Per-coin evolution ────────────────────────────────────────────────────────
#[derive(Debug)]
struct CoinResult {
    coin:          String,
    strategy:      String,
    train_fitness: f64,
    test_trades:   usize,
    test_wr:       f64,
    test_pf:       f64,
    test_pnl:      f64,
    test_dd:       f64,
    best_entry:    Vec<(String, String, f64)>,  // (feat, op, thr)
    best_exit:     Vec<(String, String, f64)>,
    fitness_hist:  Vec<f64>,  // best fitness per generation (every 10)
}

fn evolve_coin(coin: &str, strategy: &str, coin_idx: usize) -> CoinResult {
    let bars = load_ohlcv(coin);
    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let n = close.len();
    let fm = FeatMat::build(&bars);

    // 67/33 split: train 8mo, test 4mo
    let split = n * 2 / 3;
    let train_start = 0;
    let train_end   = split;
    let test_start  = split;
    let test_end    = n;

    let mut rng = Rng::new(coin_idx as u64 * 0xdeadbeef + 0xcafebabe);

    // Initialize population
    let mut pop: Vec<Genome> = (0..POP).map(|_| Genome::random(&mut rng)).collect();
    let mut fits: Vec<f64>   = pop.iter().map(|g| fitness(g, &fm, &close, train_start, train_end)).collect();

    let mut best_genome = pop[0].clone();
    let mut best_fitness = fits[0];
    let mut hist: Vec<f64> = Vec::new();

    for gen in 0..GENS {
        // Track best
        for i in 0..POP {
            if fits[i] > best_fitness {
                best_fitness = fits[i];
                best_genome = pop[i].clone();
            }
        }
        if gen % 10 == 0 { hist.push(best_fitness); }

        // Build next generation
        let mut next: Vec<Genome> = Vec::with_capacity(POP);

        // Elitism: carry top ELITE
        let mut ranked: Vec<usize> = (0..POP).collect();
        ranked.sort_by(|&a, &b| fits[b].partial_cmp(&fits[a]).unwrap_or(std::cmp::Ordering::Equal));
        for &i in ranked.iter().take(ELITE) {
            next.push(pop[i].clone());
        }

        // Fill rest
        while next.len() < POP {
            let child = if rng.next_bool(0.7) {
                let p1 = tournament(&pop, &fits, &mut rng);
                let p2 = tournament(&pop, &fits, &mut rng);
                crossover(&p1, &p2, &mut rng)
            } else {
                tournament(&pop, &fits, &mut rng)
            };
            let child = if rng.next_bool(0.20) { mutate(&child, &mut rng) } else { child };
            next.push(child);
        }

        fits = next.iter().map(|g| fitness(g, &fm, &close, train_start, train_end)).collect();
        pop = next;
    }

    // Final OOS evaluation on TEST half
    let (t_entry, t_exit) = gen_signals(&best_genome, &fm, test_start, test_end);
    let close_test = &close[test_start..test_end];
    let (test_trades, _, _) = sim_trades(close_test, &t_entry, &t_exit);

    // Detailed test stats
    let mut pnls: Vec<f64> = Vec::new();
    let mut in_pos = false;
    let mut ep = 0.0f64;
    for i in 0..close_test.len() {
        if in_pos {
            let pnl_frac = (close_test[i] - ep) / ep;
            if pnl_frac <= -SL {
                let ex = ep * (1.0 - SL * (1.0 + SLIP));
                pnls.push((ex - ep) / ep * 100.0 - FEE * 2.0 * 100.0);
                in_pos = false;
            } else if t_exit[i] {
                let ex = close_test[i] * (1.0 - SLIP);
                pnls.push((ex - ep) / ep * 100.0 - FEE * 2.0 * 100.0);
                in_pos = false;
            }
        } else if t_entry[i] {
            ep = close_test[i] * (1.0 + SLIP);
            in_pos = true;
        }
    }
    if in_pos {
        pnls.push((*close_test.last().unwrap() - ep) / ep * 100.0 - FEE * 2.0 * 100.0);
    }

    let (test_wr, test_pf, test_pnl, test_dd) = if pnls.is_empty() {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        let wins: Vec<f64>   = pnls.iter().cloned().filter(|&p| p > 0.0).collect();
        let losses: Vec<f64> = pnls.iter().cloned().filter(|&p| p <= 0.0).map(|p| -p).collect();
        let wr = wins.len() as f64 / pnls.len() as f64 * 100.0;
        let gw: f64 = wins.iter().sum();
        let gl: f64 = losses.iter().sum();
        let pf = if gl > 0.0 { gw / gl } else { gw };
        let mut eq = 10_000.0f64;
        let mut peak = eq; let mut dd = 0.0f64;
        for &p in &pnls {
            eq *= 1.0 + p / 100.0;
            if eq > peak { peak = eq; }
            let d = (peak - eq) / peak * 100.0;
            if d > dd { dd = d; }
        }
        let pnl = (eq - 10_000.0) / 10_000.0 * 100.0;
        (wr, pf, pnl, dd)
    };

    let op_str = |op: u8| if op == 0 { "<" } else { ">" };
    let best_entry: Vec<_> = best_genome.entry.iter().map(|r|
        (FEATURES[r.feat as usize].0.to_string(), op_str(r.op).to_string(), r.thr)).collect();
    let best_exit: Vec<_> = best_genome.exit.iter().map(|r|
        (FEATURES[r.feat as usize].0.to_string(), op_str(r.op).to_string(), r.thr)).collect();

    CoinResult {
        coin: coin.to_string(), strategy: strategy.to_string(),
        train_fitness: best_fitness,
        test_trades, test_wr, test_pf, test_pnl, test_dd,
        best_entry, best_exit, fitness_hist: hist,
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN22 — Genetic Algorithm v2: Strategy Discovery");
    println!("================================================================");
    println!("Fix: fitness evaluated on TRAIN only; best genome scored on OOS TEST.");
    println!("Pop={} Gens={} Tournament={} Elite={} Mutate=20% Crossover=70%", POP, GENS, TOURN, ELITE);
    println!("Indicators: RSI14 | z20 | BB-pos | vol-ratio | ROC5 | StochK | ADX14 | MACD-h");
    println!("Genome: 1-3 entry rules (AND) + 1-2 exit rules (OR) | SL=0.3%");
    println!("Trade: fee=0.1%/side slip=0.05%/side | Breakeven WR ≈ 44%");
    println!("Split: 67% train (8mo) / 33% test (4mo)");
    println!();

    let coins: Vec<(&str, &str)> = COIN_STRATEGIES.to_vec();

    let results: Vec<CoinResult> = coins.par_iter()
        .enumerate()
        .filter(|_| !shutdown.load(Ordering::SeqCst))
        .map(|(idx, (coin, strat))| evolve_coin(coin, strat, idx))
        .collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Shutdown before completion.");
        return;
    }

    let mut sorted = results;
    sorted.sort_by(|a, b| a.coin.cmp(&b.coin));

    // ── Results table ──────────────────────────────────────────────────────────
    println!("  OOS Test Results (33% hold-out, 4mo):");
    println!("  {:<6}  {:>6}  {:>7}  {:>6}  {:>8}  {:>8}",
        "Coin", "Trades", "WR%", "PF", "P&L%", "MaxDD%");
    println!("  {}", "-".repeat(52));
    for r in &sorted {
        println!("  {:<6}  {:>6}  {:>7.1}  {:>6.2}  {:>8.1}  {:>8.1}",
            r.coin, r.test_trades, r.test_wr, r.test_pf, r.test_pnl, r.test_dd);
    }

    // ── Portfolio stats ────────────────────────────────────────────────────────
    let nc = sorted.len() as f64;
    let avg_wr  = sorted.iter().map(|r| r.test_wr).sum::<f64>() / nc;
    let avg_pf  = sorted.iter().filter(|r| r.test_trades > 0)
        .map(|r| r.test_pf).sum::<f64>() / nc;
    let avg_pnl = sorted.iter().map(|r| r.test_pnl).sum::<f64>() / nc;
    let n_above44 = sorted.iter().filter(|r| r.test_wr > 44.0 && r.test_trades >= 10).count();
    let n_pf_gt1  = sorted.iter().filter(|r| r.test_pf > 1.0 && r.test_trades >= 10).count();

    println!();
    println!("  Portfolio avg (OOS test half):");
    println!("  Avg WR: {:.2}%  Avg PF: {:.3}  Avg P&L: {:.2}%", avg_wr, avg_pf, avg_pnl);
    println!("  WR > 44%: {}/{}  PF > 1.0: {}/{}", n_above44, sorted.len(), n_pf_gt1, sorted.len());

    // ── Best entry rule patterns ───────────────────────────────────────────────
    println!();
    println!("  Best genome entry rules:");
    let mut feat_usage: [usize; N_FEATS] = [0; N_FEATS];
    for r in &sorted {
        for (feat, op, thr) in &r.best_entry {
            println!("    {} — {} {} {:.2}", r.coin, feat, op, thr);
            if let Some(fid) = FEATURES.iter().position(|(nm, _, _)| nm == feat) {
                feat_usage[fid] += 1;
            }
        }
    }
    println!();
    println!("  Indicator usage in evolved entry rules:");
    let mut usage_pairs: Vec<(&str, usize)> = FEATURES.iter().zip(feat_usage.iter())
        .map(|((nm, _, _), &cnt)| (*nm, cnt))
        .collect();
    usage_pairs.sort_by(|a, b| b.1.cmp(&a.1));
    for (nm, cnt) in &usage_pairs {
        if *cnt > 0 { println!("    {:<10}: {}", nm, cnt); }
    }

    // ── Save ───────────────────────────────────────────────────────────────────
    let per_coin: Vec<Value> = sorted.iter().map(|r| json!({
        "coin": r.coin,
        "coinclaw_strategy": r.strategy,
        "train_fitness": r.train_fitness,
        "test": {
            "n_trades": r.test_trades,
            "win_rate": r.test_wr,
            "profit_factor": r.test_pf,
            "total_pnl_pct": r.test_pnl,
            "max_drawdown": r.test_dd,
        },
        "best_entry_rules": r.best_entry.iter().map(|(f, op, thr)|
            json!({"feature": f, "op": op, "threshold": thr})).collect::<Vec<_>>(),
        "best_exit_rules": r.best_exit.iter().map(|(f, op, thr)|
            json!({"feature": f, "op": op, "threshold": thr})).collect::<Vec<_>>(),
        "fitness_history": r.fitness_hist,
    })).collect();

    let summary = json!({
        "avg_test_wr": avg_wr,
        "avg_test_pf": avg_pf,
        "avg_test_pnl": avg_pnl,
        "n_coins_wr_above44": n_above44,
        "n_coins_pf_above1": n_pf_gt1,
        "indicator_usage": usage_pairs.iter().map(|(nm, cnt)|
            json!({"indicator": nm, "count": cnt})).collect::<Vec<_>>(),
        "ga_params": {
            "pop": POP, "gens": GENS, "tournament": TOURN, "elite": ELITE,
            "mutation_rate": 0.20, "crossover_rate": 0.70,
            "min_trades_train": MIN_TRADES_TRAIN,
        },
    });

    let out = json!({ "per_coin": per_coin, "summary": summary });
    let out_path = "archive/RUN22/run22_results.json";
    std::fs::create_dir_all("archive/RUN22").ok();
    std::fs::write(out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    println!();
    println!("  Results saved to {}", out_path);
}
