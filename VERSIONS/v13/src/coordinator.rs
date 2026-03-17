use crate::config;
use crate::state::SharedState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketMode {
    Long,
    IsoShort,
    Short,
}

impl std::fmt::Display for MarketMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Long => write!(f, "LONG"),
            Self::IsoShort => write!(f, "ISO_SHORT"),
            Self::Short => write!(f, "SHORT"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarketCtx {
    pub avg_z: f64,
    pub avg_rsi: f64,
    pub btc_z: f64,
    pub avg_z_valid: bool,
    pub avg_rsi_valid: bool,
    pub btc_z_valid: bool,
}

pub fn compute_breadth_and_context(state: &SharedState) -> (f64, usize, usize, MarketMode, MarketCtx) {
    let mut z_scores = Vec::new();
    let mut rsi_values = Vec::new();
    let mut btc_z = f64::NAN;

    for cs in &state.coins {
        if let Some(ref ind) = cs.ind_15m {
            if !ind.z.is_nan() {
                z_scores.push(ind.z);
                if cs.name == "BTC" {
                    btc_z = ind.z;
                }
            }
            if !ind.rsi.is_nan() {
                rsi_values.push(ind.rsi);
            }
        }
    }

    let total = z_scores.len();
    let bearish = z_scores.iter().filter(|&&z| z < -1.0).count();
    let breadth = if total > 0 { bearish as f64 / total as f64 } else { 0.0 };

    let avg_z = if !z_scores.is_empty() {
        z_scores.iter().sum::<f64>() / z_scores.len() as f64
    } else { 0.0 };

    let avg_rsi = if !rsi_values.is_empty() {
        rsi_values.iter().sum::<f64>() / rsi_values.len() as f64
    } else { 50.0 };

    let mode = if breadth <= config::BREADTH_MAX {
        MarketMode::Long
    } else if breadth >= config::SHORT_BREADTH_MIN {
        MarketMode::Short
    } else {
        MarketMode::IsoShort
    };

    let ctx = MarketCtx {
        avg_z,
        avg_rsi,
        btc_z: if btc_z.is_nan() { 0.0 } else { btc_z },
        avg_z_valid: !z_scores.is_empty(),
        avg_rsi_valid: !rsi_values.is_empty(),
        btc_z_valid: !btc_z.is_nan(),
    };

    (breadth, bearish, total, mode, ctx)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    Ranging,
    WeakTrend,
    StrongTrend,
    HighVol,
    Squeeze,
}

impl std::fmt::Display for Regime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Ranging => write!(f, "RANGE"),
            Self::WeakTrend => write!(f, "WTREND"),
            Self::StrongTrend => write!(f, "STREND"),
            Self::HighVol => write!(f, "HIVOL"),
            Self::Squeeze => write!(f, "SQUEEZE"),
        }
    }
}

pub fn detect_regime(adx: f64, bb_width: f64, bb_width_avg: f64) -> Regime {
    if !bb_width_avg.is_nan() && bb_width < bb_width_avg * 0.6 {
        Regime::Squeeze
    } else if !bb_width_avg.is_nan() && bb_width > bb_width_avg * 1.5 {
        Regime::HighVol
    } else if !adx.is_nan() && adx > 30.0 {
        Regime::StrongTrend
    } else if !adx.is_nan() && adx > 20.0 {
        Regime::WeakTrend
    } else {
        Regime::Ranging
    }
}
