use crate::config::{self, COINS, INITIAL_CAPITAL};
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
    pub scalp_cooldown_until: Option<std::time::Instant>, // Fix #1: time-based scalp cooldown
    pub consecutive_sl: u32, // Fix #4: consecutive SL counter for escalating cooldown
    pub regime: Regime,
    pub active_strat: Option<String>,
    pub ind_15m: Option<Ind15m>,
    pub ind_1m: Option<Ind1m>,
    pub candles_15m: Vec<Candle>,
    pub candles_1m: Vec<Candle>,
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
                scalp_cooldown_until: None,
                consecutive_sl: 0,
                regime: Regime::Ranging,
                active_strat: None,
                ind_15m: None,
                ind_1m: None,
                candles_15m: Vec::new(),
                candles_1m: Vec::new(),
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
