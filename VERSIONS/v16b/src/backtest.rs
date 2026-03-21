//! Bayesian vs Binary Strategy Backtest for CoinClaw
//! Uses cached CSV data from ProjectCoin/data_cache
//! Run with: cargo run --release --features backtest -- --backtest

use crate::indicators::Candle;
use std::collections::HashMap;
use std::path::Path;

// === Data structure for computed indicators ===

#[derive(Debug, Clone)]
struct Ind {
    z: f64,
    rsi: f64,
    vol: f64,
    vol_ma: f64,
    bb_lo: f64,
    bb_hi: f64,
    vwap: f64,
    adr_lo: f64,
    adr_hi: f64,
    valid: bool,
}

// === Bayesian Model ===

#[derive(Debug, Clone, Default)]
struct BetaBayes {
    alpha: f64,
    beta: f64,
}

impl BetaBayes {
    fn new() -> Self {
        Self { alpha: 1.0, beta: 1.0 }
    }

    fn observe(&mut self, success: bool) {
        if success {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
    }

    fn posterior_mean(&self) -> f64 {
        let total = self.alpha + self.beta;
        if total <= 2.0 {
            return 0.5;
        }
        self.alpha / total
    }
}

struct BayesianModel {
    z_bins: HashMap<i32, BetaBayes>,
    combined_bins: HashMap<String, BetaBayes>,
    strategy_bins: HashMap<String, BetaBayes>,
}

impl BayesianModel {
    fn new() -> Self {
        Self {
            z_bins: HashMap::new(),
            combined_bins: HashMap::new(),
            strategy_bins: HashMap::new(),
        }
    }

    fn bin_z(&self, z: f64) -> i32 {
        (z / 0.5).floor() as i32
    }

    fn bin_rsi(&self, rsi: f64) -> i32 {
        (rsi / 10.0).floor() as i32
    }

    fn learn(&mut self, ind: &Ind, next_return: f64, strat_name: &str) {
        let up = next_return > 0.0;

        let z_bin = self.bin_z(ind.z);
        self.z_bins
            .entry(z_bin)
            .or_insert_with(BetaBayes::new)
            .observe(up);

        let key = format!("z{}:rsi{}", z_bin, self.bin_rsi(ind.rsi));
        self.combined_bins
            .entry(key)
            .or_insert_with(BetaBayes::new)
            .observe(up);

        self.strategy_bins
            .entry(strat_name.to_string())
            .or_insert_with(BetaBayes::new)
            .observe(up);
    }

    fn prob_up(&self, ind: &Ind) -> f64 {
        let z_bin = self.bin_z(ind.z);
        let key = format!("z{}:rsi{}", z_bin, self.bin_rsi(ind.rsi));

        if let Some(b) = self.combined_bins.get(&key) {
            let pm = b.posterior_mean();
            if pm != 0.5 {
                return pm;
            }
        }

        self.z_bins
            .get(&z_bin)
            .map(|b| b.posterior_mean())
            .unwrap_or(0.5)
    }

    fn prob_up_strat(&self, strat_name: &str) -> f64 {
        self.strategy_bins
            .get(strat_name)
            .map(|b| b.posterior_mean())
            .unwrap_or(0.5)
    }
}

// === Indicator Calculation ===

fn calc_indicators(candles: &[Candle]) -> Vec<Ind> {
    let n = candles.len();
    let mut result = Vec::with_capacity(n);

    for i in 30..n {
        let window = &candles[i.saturating_sub(30)..i];
        let prices: Vec<f64> = window.iter().map(|c| c.c).collect();
        let volumes: Vec<f64> = window.iter().map(|c| c.v).collect();

        if prices.len() < 20 {
            result.push(Ind {
                z: f64::NAN,
                rsi: f64::NAN,
                vol: 0.0,
                vol_ma: 0.0,
                bb_lo: f64::NAN,
                bb_hi: f64::NAN,
                vwap: f64::NAN,
                adr_lo: f64::NAN,
                adr_hi: f64::NAN,
                valid: false,
            });
            continue;
        }

        let sma20: f64 = prices.iter().sum::<f64>() / prices.len() as f64;
        let variance = prices.iter().map(|p| (p - sma20).powi(2)).sum::<f64>() / prices.len() as f64;
        let std = variance.sqrt();

        let z = if std > 0.0 {
            (candles[i].c - sma20) / std
        } else {
            0.0
        };

        let rsi = calc_rsi(&prices, 14);
        let vol_ma: f64 = volumes.iter().sum::<f64>() / volumes.len() as f64;
        let vol = candles[i].v;
        let bb_lo = sma20 - 2.0 * std;
        let bb_hi = sma20 + 2.0 * std;
        let vwap = candles[i].c;
        let adr_lo = window.iter().map(|c| c.l).fold(f64::INFINITY, f64::min);
        let adr_hi = window.iter().map(|c| c.h).fold(f64::NEG_INFINITY, f64::max);

        result.push(Ind {
            z,
            rsi,
            vol,
            vol_ma,
            bb_lo,
            bb_hi,
            vwap,
            adr_lo,
            adr_hi,
            valid: true,
        });
    }

    result
}

fn calc_rsi(prices: &[f64], period: usize) -> f64 {
    if prices.len() < period + 1 {
        return 50.0;
    }

    let mut gains = 0.0;
    let mut losses = 0.0;

    for i in prices.len() - period..prices.len() {
        let diff = prices[i] - prices[i - 1];
        if diff > 0.0 {
            gains += diff;
        } else {
            losses -= diff;
        }
    }

    let avg_gain = gains / period as f64;
    let avg_loss = losses / period as f64;

    if avg_loss == 0.0 {
        return 100.0;
    }
    let rs = avg_gain / avg_loss;
    100.0 - (100.0 / (1.0 + rs))
}

// === Backtest Results ===

#[derive(Debug, Clone)]
struct BacktestResult {
    strategy: String,
    signals: usize,
    wins: usize,
    trades: usize,
    final_balance: f64,
    return_pct: f64,
}

impl BacktestResult {
    fn new(strategy: &str) -> Self {
        Self {
            strategy: strategy.to_string(),
            signals: 0,
            wins: 0,
            trades: 0,
            final_balance: 10000.0,
            return_pct: 0.0,
        }
    }

    fn win_rate(&self) -> f64 {
        if self.signals == 0 {
            0.0
        } else {
            self.wins as f64 / self.signals as f64 * 100.0
        }
    }
}

#[derive(Debug)]
struct CoinResult {
    coin: String,
    binary: BacktestResult,
    bayesian: BacktestResult,
    combined: BacktestResult,
}

// === Load Data from CSV ===

fn load_csv_data(coin: &str) -> Vec<Candle> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_1year.csv", coin);
    
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("  Failed to read {}: {}", path, e);
            return vec![];
        }
    };

    let mut candles = Vec::new();
    for line in data.lines().skip(1) { // skip header
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 6 {
            continue;
        }
        
        let o: f64 = parts[1].parse().unwrap_or(f64::NAN);
        let h: f64 = parts[2].parse().unwrap_or(f64::NAN);
        let l: f64 = parts[3].parse().unwrap_or(f64::NAN);
        let c: f64 = parts[4].parse().unwrap_or(f64::NAN);
        let v: f64 = parts[5].parse().unwrap_or(f64::NAN);
        
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() || v.is_nan() {
            continue;
        }
        
        candles.push(Candle { o, h, l, c, v });
    }
    
    candles
}

// === Main Backtest Runner ===

pub async fn run_backtest() {
    println!("\n=== Bayesian vs Binary Strategy Backtest (PER-COIN MODEL) ===\n");
    println!("Using cached data from: /home/scamarena/ProjectCoin/data_cache/\n");

    let coins = vec![
        "DASH", "UNI", "NEAR", "ADA", "LTC", "SHIB", "LINK", "ETH",
        "BTC", "BNB", "XRP", "SOL", "DOT", "MATIC", "AVAX", "ATOM", "DOGE", "FIL",
    ];

    // Load all coin data from cache
    println!("Loading cached data for {} coins...", coins.len());
    let mut all_candles: HashMap<String, Vec<Candle>> = HashMap::new();
    
    for coin in &coins {
        print!("  Loading {}... ", coin);
        let candles = load_csv_data(coin);
        println!("{} bars", candles.len());
        all_candles.insert(coin.to_string(), candles);
    }
    println!();

    // Train and test per coin
    println!("Training & testing PER-COIN Bayesian models...\n");

    let mut results: Vec<CoinResult> = Vec::new();
    let initial_balance = 10000.0;
    let leverage = 3.0;
    let trade_size = 1000.0;

    for coin in &coins {
        let candles = match all_candles.get(*coin) {
            Some(c) => c,
            None => continue,
        };
        if candles.len() < 500 {
            continue;
        }

        let indicators = calc_indicators(candles);
        let data_start = 30;
        let total_data = indicators.len();
        
        if total_data < 100 {
            continue;
        }

        let train_size = total_data / 2;
        let test_start = data_start + train_size;

        // Train PER-COIN Bayesian model
        let mut bayesian = BayesianModel::new();
        
        for i in data_start..test_start.saturating_sub(4) {
            let ind = match indicators.get(i - data_start) {
                Some(i) => i,
                None => continue,
            };
            if !ind.valid || ind.z.is_nan() || ind.rsi.is_nan() {
                continue;
            }

            let current_price = candles[i].c;
            let next_price = candles[i + 4].c;
            let ret = (next_price - current_price) / current_price;

            let strat = detect_coinclaw_strategy(ind);
            bayesian.learn(ind, ret, &strat);
        }

        // Test with per-coin model
        let mut binary_result = BacktestResult::new("Binary");
        let mut bayesian_result = BacktestResult::new("Bayesian");
        let mut combined_result = BacktestResult::new("Combined");

        for i in test_start..(total_data - 4 + data_start) {
            let ind = match indicators.get(i - data_start) {
                Some(i) => i,
                None => continue,
            };
            if !ind.valid || ind.z.is_nan() || ind.rsi.is_nan() {
                continue;
            }

            let current_price = candles[i].c;
            let next_price = candles[i + 4].c;
            let ret = (next_price - current_price) / current_price;
            let up = ret > 0.0;

            // Binary
            if ind.z < -1.5 && ind.rsi < 40.0 && ind.vol > ind.vol_ma * 1.2 {
                binary_result.signals += 1;
                if up { binary_result.wins += 1; }
                binary_result.trades += 1;
                binary_result.final_balance += trade_size * ret * leverage;
            }

            // Bayesian (per-coin model!)
            let prob_up = bayesian.prob_up(ind);
            if prob_up > 0.60 {
                bayesian_result.signals += 1;
                if up { bayesian_result.wins += 1; }
                bayesian_result.trades += 1;
                bayesian_result.final_balance += trade_size * ret * leverage;
            }

            // Combined
            let strat = detect_coinclaw_strategy(ind);
            let prob_strat = bayesian.prob_up_strat(&strat);
            let is_binary_signal = ind.z < -1.5 && ind.rsi < 40.0;

            if is_binary_signal && prob_strat > 0.55 {
                combined_result.signals += 1;
                if up { combined_result.wins += 1; }
                combined_result.trades += 1;
                combined_result.final_balance += trade_size * ret * leverage;
            }
        }

        binary_result.return_pct = (binary_result.final_balance / initial_balance - 1.0) * 100.0;
        bayesian_result.return_pct = (bayesian_result.final_balance / initial_balance - 1.0) * 100.0;
        combined_result.return_pct = (combined_result.final_balance / initial_balance - 1.0) * 100.0;

        // Print per-coin stats
        let b_wr = if binary_result.signals > 0 { binary_result.win_rate() } else { 0.0 };
        let bay_wr = if bayesian_result.signals > 0 { bayesian_result.win_rate() } else { 0.0 };
        let c_wr = if combined_result.signals > 0 { combined_result.win_rate() } else { 0.0 };
        
        println!("{}: Binary {:>5.0}% ({:>3}sig) | Bayesian {:>5.0}% ({:>3}sig) | Combined {:>5.0}% ({:>3}sig)",
            coin,
            b_wr, binary_result.signals,
            bay_wr, bayesian_result.signals,
            c_wr, combined_result.signals
        );

        results.push(CoinResult {
            coin: coin.to_string(),
            binary: binary_result,
            bayesian: bayesian_result,
            combined: combined_result,
        });
    }

    // Aggregate totals
    let mut total_binary = BacktestResult::new("Binary");
    let mut total_bayesian = BacktestResult::new("Bayesian");
    let mut total_combined = BacktestResult::new("Combined");

    for r in &results {
        total_binary.signals += r.binary.signals;
        total_binary.wins += r.binary.wins;
        total_binary.trades += r.binary.trades;
        total_binary.final_balance += r.binary.final_balance - initial_balance;

        total_bayesian.signals += r.bayesian.signals;
        total_bayesian.wins += r.bayesian.wins;
        total_bayesian.trades += r.bayesian.trades;
        total_bayesian.final_balance += r.bayesian.final_balance - initial_balance;

        total_combined.signals += r.combined.signals;
        total_combined.wins += r.combined.wins;
        total_combined.trades += r.combined.trades;
        total_combined.final_balance += r.combined.final_balance - initial_balance;
    }

    total_binary.return_pct = (total_binary.final_balance / (initial_balance * results.len() as f64)) * 100.0;
    total_bayesian.return_pct = (total_bayesian.final_balance / (initial_balance * results.len() as f64)) * 100.0;
    total_combined.return_pct = (total_combined.final_balance / (initial_balance * results.len() as f64)) * 100.0;

    // Summary table
    println!("\n╔═══════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                   SUMMARY TABLE                                               ║");
    println!("╠═══════════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║ {:^8} │ {:^15} │ {:^15} │ {:^15} │", "COIN", "BINARY", "BAYESIAN", "COMBINED");
    println!("║ {:^8} │ {:^15} │ {:^15} │ {:^15} │", "", "WR%  Ret%", "WR%  Ret%", "WR%  Ret%");
    println!("╟──────────┼───────────────────┼───────────────────┼───────────────────╢");

    for r in &results {
        let b_wr = if r.binary.signals > 0 { r.binary.win_rate() } else { 0.0 };
        let bay_wr = if r.bayesian.signals > 0 { r.bayesian.win_rate() } else { 0.0 };
        let c_wr = if r.combined.signals > 0 { r.combined.win_rate() } else { 0.0 };
        
        println!("║ {:^8} │ {:>5.0}% {:>+7.1}% │ {:>5.0}% {:>+7.1}% │ {:>5.0}% {:>+7.1}% ║",
            r.coin,
            b_wr, r.binary.return_pct,
            bay_wr, r.bayesian.return_pct,
            c_wr, r.combined.return_pct
        );
    }

    println!("╟──────────┼───────────────────┼───────────────────┼───────────────────╢");
    
    let tb_wr = if total_binary.signals > 0 { total_binary.win_rate() } else { 0.0 };
    let tbay_wr = if total_bayesian.signals > 0 { total_bayesian.win_rate() } else { 0.0 };
    let tc_wr = if total_combined.signals > 0 { total_combined.win_rate() } else { 0.0 };
    
    println!("║ {:^8} │ {:>5.0}% {:>+7.1}% │ {:>5.0}% {:>+7.1}% │ {:>5.0}% {:>+7.1}% ║",
        "TOTAL",
        tb_wr, total_binary.return_pct,
        tbay_wr, total_bayesian.return_pct,
        tc_wr, total_combined.return_pct
    );
    println!("╚═══════════════════════════════════════════════════════════════════════════════════════════════════╝");

    println!("\n=== FINAL SUMMARY ===\n");
    println!("Signals: Binary = {}, Bayesian = {}, Combined = {}", 
             total_binary.signals, total_bayesian.signals, total_combined.signals);
    println!("Win Rates: Binary = {:.1}%, Bayesian = {:.1}%, Combined = {:.1}%",
             tb_wr, tbay_wr, tc_wr);
    println!("Returns:   Binary = {:+.1}%, Bayesian = {:+.1}%, Combined = {:+.1}%",
             total_binary.return_pct, total_bayesian.return_pct, total_combined.return_pct);
    println!();

    // Count wins
    let mut bayesian_wins = 0;
    let mut binary_wins = 0;
    let mut combined_wins = 0;
    
    for r in &results {
        let b_wr = r.binary.signals > 0 && r.binary.win_rate() > 50.0;
        let bay_wr = r.bayesian.signals > 0 && r.bayesian.win_rate() > 50.0;
        let c_wr = r.combined.signals > 0 && r.combined.win_rate() > 50.0;
        
        if b_wr { binary_wins += 1; }
        if bay_wr { bayesian_wins += 1; }
        if c_wr { combined_wins += 1; }
    }
    
    println!("Coins with WR > 50%: Binary = {}/{}, Bayesian = {}/{}, Combined = {}/{}",
             binary_wins, results.len(),
             bayesian_wins, results.len(),
             combined_wins, results.len());
    println!();

    if tbay_wr > tb_wr {
        println!("✓ PER-COIN Bayesian improves win rate by {:.1} pts over Binary", tbay_wr - tb_wr);
    } else {
        println!("○ Binary wins by {:.1} pts", tb_wr - tbay_wr);
    }
}

fn detect_coinclaw_strategy(ind: &Ind) -> String {
    if !ind.valid || ind.z.is_nan() {
        return "none".to_string();
    }

    if ind.z < -1.5 && ind.rsi < 40.0 {
        return "mean_rev".to_string();
    }

    if ind.bb_lo.is_nan() {
        return "other".to_string();
    }
    if ind.vwap <= ind.bb_lo * 1.02 && ind.vol > ind.vol_ma * 1.3 {
        return "bb_bounce".to_string();
    }

    let range = ind.adr_hi - ind.adr_lo;
    if range > 0.0 && ind.vwap <= ind.adr_lo + range * 0.25 {
        return "adr_rev".to_string();
    }

    if ind.z < -1.5 && ind.vwap < ind.vwap && ind.vol > ind.vol_ma * 1.2 {
        return "vwap_rev".to_string();
    }

    "other".to_string()
}
