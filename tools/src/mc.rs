use rand::prelude::*;
use rand::rngs::SmallRng;
use rayon::prelude::*;

use crate::backtest::compute_stats;

#[derive(Debug, Clone, serde::Serialize)]
pub struct McResult {
    pub n_sims: usize,
    // Profit Factor percentiles
    pub p5_pf:  f64,
    pub p50_pf: f64,
    pub p95_pf: f64,
    // Win rate percentiles (%)
    pub p5_wr:  f64,
    pub p50_wr: f64,
    pub p95_wr: f64,
    // Max drawdown percentiles (%)
    pub p5_dd:  f64,
    pub p50_dd: f64,
    pub p95_dd: f64,
    // Probability of PF > 1 across simulations
    pub prob_profit: f64,
}

/// Monte Carlo simulation: shuffle trade P&L order N times.
/// Each shuffle = a different luck ordering. Reports distribution of outcomes.
/// Uses Rayon to parallelise across `n_sims` chunks.
pub fn monte_carlo(pnls: &[f64], n_sims: usize) -> McResult {
    if pnls.is_empty() {
        return McResult {
            n_sims,
            p5_pf: 0.0, p50_pf: 0.0, p95_pf: 0.0,
            p5_wr: 0.0, p50_wr: 0.0, p95_wr: 0.0,
            p5_dd: 0.0, p50_dd: 0.0, p95_dd: 0.0,
            prob_profit: 0.0,
        };
    }

    // Split into 20 parallel chunks
    let chunk = (n_sims / 20).max(1);
    let results: Vec<(Vec<f64>, Vec<f64>, Vec<f64>)> = (0..20usize)
        .into_par_iter()
        .map(|chunk_id| {
            let mut rng = SmallRng::seed_from_u64(chunk_id as u64 * 7919 + 42);
            let mut sim_pnls = pnls.to_vec();
            let mut pfs = Vec::with_capacity(chunk);
            let mut wrs = Vec::with_capacity(chunk);
            let mut dds = Vec::with_capacity(chunk);

            for _ in 0..chunk {
                sim_pnls.shuffle(&mut rng);
                let s = compute_stats(&sim_pnls);
                pfs.push(s.profit_factor);
                wrs.push(s.win_rate);
                dds.push(s.max_drawdown);
            }
            (pfs, wrs, dds)
        })
        .collect();

    let mut all_pfs: Vec<f64> = results.iter().flat_map(|(p, _, _)| p.iter().cloned()).collect();
    let mut all_wrs: Vec<f64> = results.iter().flat_map(|(_, w, _)| w.iter().cloned()).collect();
    let mut all_dds: Vec<f64> = results.iter().flat_map(|(_, _, d)| d.iter().cloned()).collect();

    all_pfs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    all_wrs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    all_dds.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let total = all_pfs.len();
    let prob_profit = all_pfs.iter().filter(|&&p| p > 1.0).count() as f64 / total as f64;

    McResult {
        n_sims: total,
        p5_pf:  all_pfs[total / 20],
        p50_pf: all_pfs[total / 2],
        p95_pf: all_pfs[total * 19 / 20],
        p5_wr:  all_wrs[total / 20],
        p50_wr: all_wrs[total / 2],
        p95_wr: all_wrs[total * 19 / 20],
        p5_dd:  all_dds[total / 20],
        p50_dd: all_dds[total / 2],
        p95_dd: all_dds[total * 19 / 20],
        prob_profit,
    }
}

/// Block bootstrap for time-series: resamples blocks of `block_size` bars
/// to preserve autocorrelation within trades. Used for portfolio equity curve.
/// Returns distribution of final portfolio returns.
pub fn block_bootstrap(daily_returns: &[f64], n_sims: usize, block_size: usize) -> Vec<f64> {
    let n = daily_returns.len();
    if n == 0 { return vec![0.0; n_sims]; }
    let n_blocks = (n / block_size) + 1;

    (0..n_sims)
        .into_par_iter()
        .map(|seed| {
            let mut rng = SmallRng::seed_from_u64(seed as u64 + 99991);
            let mut portfolio = vec![0.0f64; n];
            let mut pos = 0;
            while pos < n {
                let block_start = rng.gen_range(0..n.saturating_sub(block_size).max(1));
                let block_end = (block_start + block_size).min(n);
                let take = (n - pos).min(block_end - block_start);
                portfolio[pos..pos + take].copy_from_slice(&daily_returns[block_start..block_start + take]);
                pos += take;
            }
            // Compound return
            portfolio.iter().map(|&r| 1.0 + r).product::<f64>() - 1.0
        })
        .collect()
}

/// Summarise a distribution into percentiles + mean
pub fn percentiles(mut vals: Vec<f64>) -> (f64, f64, f64, f64) {
    if vals.is_empty() { return (0.0, 0.0, 0.0, 0.0); }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = vals.len();
    let mean = vals.iter().sum::<f64>() / n as f64;
    (vals[n / 20], vals[n / 2], vals[n * 19 / 20], mean)
}
