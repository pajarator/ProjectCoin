/// Fast backtester — matches Python Backtester logic.
/// Fee: stored but not subtracted from P&L (matching Python implementation).
/// Slippage: applied to entry and exit prices.
/// Stop loss: fixed percentage from entry price.

pub const SLIPPAGE: f64 = 0.0005; // 0.05%
pub const STOP_LOSS: f64 = 0.003; // 0.3%

#[derive(Debug, Clone, Default)]
pub struct BacktestStats {
    pub n_trades: usize,
    pub win_rate: f64,       // percent (0–100)
    pub profit_factor: f64,
    pub sharpe: f64,
    pub max_drawdown: f64,   // percent
    pub total_pnl: f64,      // percent
}

/// Run backtest. Returns (per-trade P&L vec, summary stats).
/// `entry` and `exit` are bar-aligned boolean slices.
/// Long direction only (COINCLAW primary strategies are all long).
pub fn backtest(
    close: &[f64],
    entry: &[bool],
    exit: &[bool],
    stop_loss: f64,
) -> (Vec<f64>, BacktestStats) {
    let n = close.len();
    assert_eq!(n, entry.len());
    assert_eq!(n, exit.len());

    let mut pnls: Vec<f64> = Vec::new();
    let mut in_pos = false;
    let mut entry_price = 0.0f64;

    for i in 0..n {
        if in_pos {
            let pnl_frac = (close[i] - entry_price) / entry_price;
            if pnl_frac <= -stop_loss {
                // Stop loss: exit at entry * (1 - sl * (1 + slippage))
                let ep = entry_price * (1.0 - stop_loss * (1.0 + SLIPPAGE));
                let trade_pnl = (ep - entry_price) / entry_price * 100.0;
                pnls.push(trade_pnl);
                in_pos = false;
            } else if exit[i] {
                let ep = close[i] * (1.0 - SLIPPAGE);
                let trade_pnl = (ep - entry_price) / entry_price * 100.0;
                pnls.push(trade_pnl);
                in_pos = false;
            }
        } else if entry[i] {
            entry_price = close[i] * (1.0 + SLIPPAGE);
            in_pos = true;
        }
    }

    // Close open position at end of data
    if in_pos {
        let ep = *close.last().unwrap();
        let trade_pnl = (ep - entry_price) / entry_price * 100.0;
        pnls.push(trade_pnl);
    }

    let stats = compute_stats(&pnls);
    (pnls, stats)
}

pub fn compute_stats(pnls: &[f64]) -> BacktestStats {
    if pnls.is_empty() {
        return BacktestStats::default();
    }

    let n = pnls.len();
    let wins: Vec<f64> = pnls.iter().cloned().filter(|&p| p > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().cloned().filter(|&p| p <= 0.0).collect();

    let win_rate = wins.len() as f64 / n as f64 * 100.0;
    let total_wins: f64 = wins.iter().sum();
    let total_losses: f64 = losses.iter().map(|&p| -p).sum::<f64>();
    let profit_factor = if total_losses > 0.0 { total_wins / total_losses } else { total_wins };

    // Equity curve and drawdown
    let mut equity = 10_000.0f64;
    let mut peak = equity;
    let mut max_dd = 0.0f64;
    let mut eq_curve = vec![equity];
    for &p in pnls {
        equity *= 1.0 + p / 100.0;
        eq_curve.push(equity);
        if equity > peak { peak = equity; }
        let dd = (peak - equity) / peak * 100.0;
        if dd > max_dd { max_dd = dd; }
    }

    let total_pnl = (equity - 10_000.0) / 10_000.0 * 100.0;

    // Sharpe (trade-level, annualised ×√252)
    let returns: Vec<f64> = eq_curve.windows(2).map(|w| (w[1] - w[0]) / w[0]).collect();
    let mean_r: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
    let var_r: f64 = returns.iter().map(|&r| (r - mean_r).powi(2)).sum::<f64>() / returns.len() as f64;
    let sharpe = if var_r > 0.0 { mean_r / var_r.sqrt() * 252.0_f64.sqrt() } else { 0.0 };

    BacktestStats { n_trades: n, win_rate, profit_factor, sharpe, max_drawdown: max_dd, total_pnl }
}
