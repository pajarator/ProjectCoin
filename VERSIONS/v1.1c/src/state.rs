use crate::config::{self, COINS, INITIAL_CAPITAL, SHARPE_WINDOW};
use crate::coordinator::{MarketMode, Regime};
use crate::indicators::{Candle, Ind15m, Ind1m};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeType {
    Regime,
    Scalp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub e: f64,
    pub s: f64,
    pub high: f64,
    pub low: f64,
    pub margin: f64,
    pub dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_type: Option<TradeType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub pnl: f64,
    pub reason: String,
    pub dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_type: Option<TradeType>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoinPersist {
    pub bal: f64,
    pub pos: Option<Position>,
    pub trades: Vec<TradeRecord>,
    pub candles_held: u32,
    pub cooldown: u32,
}

/// Per-coin runtime state (not serialized directly)
pub struct CoinState {
    pub name: &'static str,
    pub binance_symbol: &'static str,
    pub config_idx: usize,
    pub bal: f64,
    pub pos: Option<Position>,
    pub trades: Vec<TradeRecord>,
    pub candles_held: u32,
    pub cooldown: u32,
    pub regime: Regime,
    pub active_strat: Option<String>,
    pub ind_15m: Option<Ind15m>,
    pub ind_1m: Option<Ind1m>,
    pub candles_15m: Vec<Candle>,
    pub candles_1m: Vec<Candle>,
    // RUN75: Sharpe-weighted capital allocation
    pub trade_pnls_pct: VecDeque<f64>,  // trailing closed-trade PnL percentages
    pub last_rebalance_bar: u32,        // last bar index when rebalance ran
}

impl CoinState {
    pub fn effective_bal(&self) -> f64 {
        if let (Some(ref pos), Some(ref ind)) = (&self.pos, &self.ind_15m) {
            let price = ind.p;
            let cost = pos.s * pos.e;
            let current_val = pos.s * price;
            let unrealized = if pos.dir == "long" {
                current_val - cost
            } else {
                cost - current_val
            };
            self.bal + unrealized
        } else {
            self.bal
        }
    }

    pub fn win_count(&self) -> usize {
        self.trades.iter().filter(|t| t.pnl > 0.0).count()
    }

    /// Compute trailing Sharpe ratio from the last SHARPE_WINDOW closed-trade PnL percentages.
    /// Returns 0.0 if fewer than 2 trades available.
    pub fn trailing_sharpe(&self) -> f64 {
        let n = self.trade_pnls_pct.len();
        if n < 2 { return 0.0; }
        let window = SHARPE_WINDOW;
        let start = if n > window { n - window } else { 0 };
        let trades_slice: Vec<f64> = self.trade_pnls_pct.iter().skip(start).cloned().collect();
        if trades_slice.len() < 2 { return 0.0; }
        let mean: f64 = trades_slice.iter().sum::<f64>() / trades_slice.len() as f64;
        let variance: f64 = trades_slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / trades_slice.len() as f64;
        let std = variance.sqrt();
        if std == 0.0 { return 0.0; }
        // Annualize for 15m bars: 35040 bars/year
        let annualized_mean = mean * 35040.0;
        let annualized_std = std * (35040.0_f64.sqrt());
        annualized_mean / annualized_std
    }

    pub fn to_persist(&self) -> CoinPersist {
        CoinPersist {
            bal: self.bal,
            pos: self.pos.clone(),
            trades: self.trades.clone(),
            candles_held: self.candles_held,
            cooldown: self.cooldown,
        }
    }
}

pub struct SharedState {
    pub coins: Vec<CoinState>,
    pub log_entries: VecDeque<String>,
    pub breadth: f64,
    pub bearish_count: usize,
    pub total_count: usize,
    pub market_mode: MarketMode,
    pub running: bool,
    pub bars_since_rebal: u32,  // RUN75: bar counter since last Sharpe rebalance
}

impl SharedState {
    pub fn new() -> Self {
        let mut coins = Vec::with_capacity(COINS.len());
        for (idx, cfg) in COINS.iter().enumerate() {
            coins.push(CoinState {
                name: cfg.name,
                binance_symbol: cfg.binance_symbol,
                config_idx: idx,
                bal: INITIAL_CAPITAL,
                pos: None,
                trades: Vec::new(),
                candles_held: 0,
                cooldown: 0,
                regime: Regime::Ranging,
                active_strat: None,
                ind_15m: None,
                ind_1m: None,
                candles_15m: Vec::new(),
                candles_1m: Vec::new(),
                trade_pnls_pct: VecDeque::with_capacity(SHARPE_WINDOW * 2),
                last_rebalance_bar: 0,
            });
        }

        // Load existing log
        let mut log_entries = VecDeque::with_capacity(config::LOG_LINES);
        if let Ok(contents) = fs::read_to_string(config::LOG_FILE) {
            for line in contents.lines() {
                log_entries.push_back(line.to_string());
                if log_entries.len() > config::LOG_LINES {
                    log_entries.pop_front();
                }
            }
        }

        let mut state = SharedState {
            coins,
            log_entries,
            breadth: 0.0,
            bearish_count: 0,
            total_count: 0,
            market_mode: MarketMode::Long,
            running: true,
            bars_since_rebal: 0,
        };
        state.load_state();
        state
    }

    pub fn load_state(&mut self) {
        let data = match fs::read_to_string(config::STATE_FILE) {
            Ok(d) => d,
            Err(_) => return,
        };
        let map: HashMap<String, CoinPersist> = match serde_json::from_str(&data) {
            Ok(m) => m,
            Err(_) => return,
        };
        for cs in &mut self.coins {
            if let Some(persisted) = map.get(cs.name) {
                cs.bal = persisted.bal;
                cs.pos = persisted.pos.clone();
                cs.trades = persisted.trades.clone();
                cs.candles_held = persisted.candles_held;
                cs.cooldown = persisted.cooldown;
            }
        }
    }

    pub fn save_state(&self) {
        let mut map: HashMap<String, CoinPersist> = HashMap::new();
        // Load existing state to preserve coins we don't track
        if let Ok(data) = fs::read_to_string(config::STATE_FILE) {
            if let Ok(existing) = serde_json::from_str::<HashMap<String, CoinPersist>>(&data) {
                map = existing;
            }
        }
        for cs in &self.coins {
            map.insert(cs.name.to_string(), cs.to_persist());
        }
        if let Ok(json) = serde_json::to_string(&map) {
            let _ = fs::write(config::STATE_FILE, json);
        }
    }

    pub fn log(&mut self, msg: String) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let entry = format!("[{}] {}", timestamp, msg);
        self.log_entries.push_back(entry.clone());
        if self.log_entries.len() > config::LOG_LINES {
            self.log_entries.pop_front();
        }
        // Append to file
        use std::io::Write;
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(config::LOG_FILE)
        {
            let _ = writeln!(f, "{}", entry);
        }
    }

    pub fn total_balance(&self) -> f64 {
        self.coins.iter().map(|c| c.effective_bal()).sum()
    }

    pub fn total_trades(&self) -> usize {
        self.coins.iter().map(|c| c.trades.len()).sum()
    }

    pub fn total_wins(&self) -> usize {
        self.coins.iter().map(|c| c.win_count()).sum()
    }

    pub fn pnl_by_type(&self) -> (f64, f64) {
        let (mut regime, mut scalp) = (0.0, 0.0);
        for cs in &self.coins {
            for t in &cs.trades {
                let is_regime = match t.trade_type {
                    Some(TradeType::Regime) => true,
                    Some(_) => false,
                    None => t.pnl.abs() >= 0.10 || t.reason == "SMA",
                };
                if is_regime { regime += t.pnl; } else { scalp += t.pnl; }
            }
        }
        (regime, scalp)
    }

    /// RUN75: Rebalance per-coin capital every SHARPE_REBAL_FREQ bars based on
    /// trailing Sharpe ratio of closed regime trades.
    /// Called from the 15m fetch loop in main.rs.
    pub fn rebalance_by_sharpe(&mut self) {
        use crate::config::{SHARPE_REBAL_FREQ, SHARPE_MIN_CAP, SHARPE_MAX_CAP, INITIAL_CAPITAL, RISK};

        self.bars_since_rebal += 1;
        if self.bars_since_rebal < SHARPE_REBAL_FREQ { return; }
        self.bars_since_rebal = 0;

        // Compute trailing Sharpe for each coin
        let sharpes: Vec<f64> = self.coins.iter().map(|c| c.trailing_sharpe()).collect();

        let sum_sharpe: f64 = sharpes.iter().sum();
        if !sum_sharpe.is_finite() || sum_sharpe <= 0.0 { return; }

        let total_portfolio: f64 = self.coins.iter().map(|c| c.bal).sum();
        let base_cap = INITIAL_CAPITAL * RISK; // $10 per coin at 10% risk

        // Target capital: weight by Sharpe, clamped to [min_cap, max_cap]
        let mut targets: Vec<f64> = Vec::with_capacity(self.coins.len());
        for &sh in &sharpes {
            let weight = sh / sum_sharpe;
            let target = (total_portfolio * weight).max(base_cap * SHARPE_MIN_CAP).min(base_cap * SHARPE_MAX_CAP);
            targets.push(target);
        }

        // Apply capital redistribution (coins with surplus give to those with deficit)
        for ci in 0..self.coins.len() {
            self.coins[ci].last_rebalance_bar = self.bars_since_rebal;
            let diff = targets[ci] - self.coins[ci].bal;
            self.coins[ci].bal += diff;
        }

        self.log(format!("[RUN75] Sharpe rebalance: portfolio=${:.2}", total_portfolio));
    }
}

pub fn fmt_price(p: f64) -> String {
    if p < 0.01 {
        format!("${:.6}", p)
    } else if p < 1.0 {
        format!("${:.4}", p)
    } else {
        format!("${:.2}", p)
    }
}
