use crate::strategies::{ComplementStrat, IsoShortStrat, LongStrat, ShortStrat};

// ── COINCLAW v14: Momentum Breakout Layer ────────────────────────────────────
// Independent signal layer (RUN27/28): fires on hard directional moves with
// volume confirmation + ADX rising. Uses own ATR stop + trailing exit.
// Enabled only for coins with confirmed persistence: NEAR, XLM, DASH.
// Disabled for anti-momentum coins: ATOM, SHIB, LTC, ADA, DOT, BNB.
#[derive(Debug, Clone)]
pub struct MomentumBreakout {
    pub move_thresh: f64,    // 16-bar compounded return threshold (e.g. 0.025 = 2.5%)
    pub vol_mult: f64,       // volume must be ≥ vol_ma × vol_mult
    pub adx_thresh: f64,     // ADX must be ≥ this AND rising (> ADX 3 bars ago)
    pub atr_mult: f64,       // hard stop = entry - ATR * atr_mult
    pub trail_atr: f64,      // trailing distance = ATR * trail_atr (fixed from entry ATR)
    pub trail_act: f64,      // activation: profit must reach trail_act before trailing fires
    pub rsi_exit: f64,       // RSI overbought exit threshold (longs)
}

pub const INITIAL_CAPITAL: f64 = 100.0;
pub const RISK: f64 = 0.10;
pub const SCALP_RISK: f64 = 0.05;
pub const LEVERAGE: f64 = 5.0;
pub const STOP_LOSS: f64 = 0.003;
pub const MIN_HOLD_CANDLES: u32 = 2;
pub const BREADTH_MAX: f64 = 0.20;
pub const SHORT_BREADTH_MIN: f64 = 0.50;
pub const ISO_SHORT_BREADTH_MAX: f64 = 0.20;
pub const LOG_LINES: usize = 50;

// Scalp overlay params (RUN10 optimal)
pub const SCALP_SL: f64 = 0.001;
pub const SCALP_TP: f64 = 0.008;
pub const SCALP_VOL_MULT: f64 = 3.5;
pub const SCALP_RSI_EXTREME: f64 = 20.0;
pub const SCALP_STOCH_EXTREME: f64 = 5.0;
pub const SCALP_COOLDOWN_SECS: u64 = 300;  // Fix #1: 5 min cooldown after scalp SL
pub const MAX_SCALP_OPENS_PER_CYCLE: usize = 3; // Fix #5: max simultaneous scalp opens per 1m cycle

// F6 filter thresholds (RUN10.1 discovery, validated OOS in RUN10.2)
pub const F6_DIR_ROC_3: f64 = -0.195;
pub const F6_AVG_BODY_3: f64 = 0.072;

// ISO short default params
pub const ISO_Z_THRESHOLD: f64 = 1.5;
pub const ISO_BB_MARGIN: f64 = 0.98;
pub const ISO_VOL_MULT: f64 = 1.2;
pub const ISO_ADR_PCT: f64 = 0.25;
pub const ISO_EXIT_Z: f64 = -0.5;
pub const ISO_Z_SPREAD: f64 = 1.5;
pub const ISO_RSI_THRESHOLD: f64 = 75.0;
pub const ISO_VOL_SPIKE_MULT: f64 = 2.0;
pub const ISO_SQUEEZE_FACTOR: f64 = 0.8;
pub const ISO_SL_ESCALATE_COOLDOWN: u32 = 60; // Fix #4: cooldown after consecutive ISO SL (~1 hour = 4 candles)

// Fetch intervals
// OU Mean Reversion params (RUN11c: DASH only)
pub const OU_WINDOW: usize = 100;
pub const OU_MIN_HALFLIFE: f64 = 10.0;
pub const OU_MAX_HALFLIFE: f64 = 100.0;
pub const OU_DEV_THRESHOLD: f64 = 2.0;

pub const FETCH_15M_LIMIT: u32 = 150;
pub const FETCH_15M_INTERVAL_SECS: u64 = 60;
pub const FETCH_1M_INTERVAL_SECS: u64 = 15;

pub const STATE_FILE: &str = concat!("/home/scamarena/ProjectCoin/trading_state_v", env!("CARGO_PKG_VERSION"), ".json");
pub const LOG_FILE: &str = concat!("/home/scamarena/ProjectCoin/trading_log_v", env!("CARGO_PKG_VERSION"), ".txt");

#[derive(Debug, Clone)]
pub struct CoinConfig {
    pub symbol: &'static str,
    pub binance_symbol: &'static str,
    pub name: &'static str,
    pub long_strat: LongStrat,
    pub short_strat: ShortStrat,
    pub iso_short_strat: IsoShortStrat,
    pub complement_strat: ComplementStrat,
    pub complement_z_filter: f64,
    // RUN27/28: None = disabled (anti-momentum or unconfirmed coins)
    pub momentum: Option<MomentumBreakout>,
}

// Momentum configs (RUN27/28 validated)
// NEAR: genuine 16-bar persistence (+2.4% edge), walk-forward avg 42.8%
const MOM_NEAR: MomentumBreakout = MomentumBreakout {
    move_thresh: 0.025, vol_mult: 2.0, adx_thresh: 20.0,
    atr_mult: 0.75, trail_atr: 1.0, trail_act: 0.008, rsi_exit: 78.0,
};
// XLM: genuine persistence at strict conditions (+5.3% edge), walk-forward avg 45.2%
const MOM_XLM: MomentumBreakout = MomentumBreakout {
    move_thresh: 0.020, vol_mult: 2.5, adx_thresh: 20.0,
    atr_mult: 0.75, trail_atr: 1.0, trail_act: 0.008, rsi_exit: 78.0,
};
// DASH: right-tail skew mechanism (avg_fwd_ret +0.822%), R:R=2.5, PnL +25.4%
const MOM_DASH: MomentumBreakout = MomentumBreakout {
    move_thresh: 0.025, vol_mult: 2.0, adx_thresh: 20.0,
    atr_mult: 1.0, trail_atr: 0.75, trail_act: 0.005, rsi_exit: 78.0,
};

pub const COINS: [CoinConfig; 18] = [
    CoinConfig { symbol: "DASH/USDT", binance_symbol: "DASHUSDT", name: "DASH",
        long_strat: LongStrat::OuMeanRev, short_strat: ShortStrat::ShortMeanRev,
        iso_short_strat: IsoShortStrat::IsoDivergence,
        complement_strat: ComplementStrat::KstCross, complement_z_filter: -0.5,
        momentum: Some(MOM_DASH) },
    CoinConfig { symbol: "UNI/USDT", binance_symbol: "UNIUSDT", name: "UNI",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRelativeZ,
        complement_strat: ComplementStrat::KalmanFilter, complement_z_filter: -1.5,
        momentum: None },
    CoinConfig { symbol: "NEAR/USDT", binance_symbol: "NEARUSDT", name: "NEAR",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme,
        complement_strat: ComplementStrat::LaguerreRsi { gamma_idx: 3 }, complement_z_filter: -0.5,
        momentum: Some(MOM_NEAR) },
    CoinConfig { symbol: "ADA/USDT", binance_symbol: "ADAUSDT", name: "ADA",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoDivergence,
        complement_strat: ComplementStrat::LaguerreRsi { gamma_idx: 0 }, complement_z_filter: -1.0,
        momentum: None }, // anti-momentum: -5.2% persistence edge
    CoinConfig { symbol: "LTC/USDT", binance_symbol: "LTCUSDT", name: "LTC",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortMeanRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme,
        complement_strat: ComplementStrat::KalmanFilter, complement_z_filter: -1.5,
        momentum: None }, // anti-momentum: -5.2% persistence edge
    CoinConfig { symbol: "SHIB/USDT", binance_symbol: "SHIBUSDT", name: "SHIB",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortVwapRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme,
        complement_strat: ComplementStrat::LaguerreRsi { gamma_idx: 3 }, complement_z_filter: -1.0,
        momentum: None }, // anti-momentum: -6.1% persistence edge
    CoinConfig { symbol: "LINK/USDT", binance_symbol: "LINKUSDT", name: "LINK",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoRelativeZ,
        complement_strat: ComplementStrat::KalmanFilter, complement_z_filter: -1.5,
        momentum: None },
    CoinConfig { symbol: "ETH/USDT", binance_symbol: "ETHUSDT", name: "ETH",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme,
        complement_strat: ComplementStrat::KalmanFilter, complement_z_filter: -1.5,
        momentum: None },
    CoinConfig { symbol: "DOT/USDT", binance_symbol: "DOTUSDT", name: "DOT",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortVwapRev,
        iso_short_strat: IsoShortStrat::IsoRelativeZ,
        complement_strat: ComplementStrat::LaguerreRsi { gamma_idx: 2 }, complement_z_filter: -1.5,
        momentum: None }, // anti-momentum: -5.1% persistence edge
    CoinConfig { symbol: "XRP/USDT", binance_symbol: "XRPUSDT", name: "XRP",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme,
        complement_strat: ComplementStrat::None, complement_z_filter: 0.0,
        momentum: None },
    CoinConfig { symbol: "ATOM/USDT", binance_symbol: "ATOMUSDT", name: "ATOM",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRelativeZ,
        complement_strat: ComplementStrat::LaguerreRsi { gamma_idx: 1 }, complement_z_filter: -1.5,
        momentum: None }, // anti-momentum: -12.4% persistence edge (strongest reverter)
    CoinConfig { symbol: "SOL/USDT", binance_symbol: "SOLUSDT", name: "SOL",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme,
        complement_strat: ComplementStrat::KalmanFilter, complement_z_filter: -1.5,
        momentum: None },
    CoinConfig { symbol: "DOGE/USDT", binance_symbol: "DOGEUSDT", name: "DOGE",
        long_strat: LongStrat::BbBounce, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoDivergence,
        complement_strat: ComplementStrat::None, complement_z_filter: 0.0,
        momentum: None },
    CoinConfig { symbol: "XLM/USDT", binance_symbol: "XLMUSDT", name: "XLM",
        long_strat: LongStrat::DualRsi, short_strat: ShortStrat::ShortMeanRev,
        iso_short_strat: IsoShortStrat::IsoRelativeZ,
        complement_strat: ComplementStrat::LaguerreRsi { gamma_idx: 1 }, complement_z_filter: -1.5,
        momentum: Some(MOM_XLM) },
    CoinConfig { symbol: "AVAX/USDT", binance_symbol: "AVAXUSDT", name: "AVAX",
        long_strat: LongStrat::AdrReversal, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoRelativeZ,
        complement_strat: ComplementStrat::KalmanFilter, complement_z_filter: -1.5,
        momentum: None },
    CoinConfig { symbol: "ALGO/USDT", binance_symbol: "ALGOUSDT", name: "ALGO",
        long_strat: LongStrat::AdrReversal, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme,
        complement_strat: ComplementStrat::LaguerreRsi { gamma_idx: 3 }, complement_z_filter: -1.5,
        momentum: None },
    CoinConfig { symbol: "BNB/USDT", binance_symbol: "BNBUSDT", name: "BNB",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortVwapRev,
        iso_short_strat: IsoShortStrat::IsoDivergence,
        complement_strat: ComplementStrat::KstCross, complement_z_filter: -1.0,
        momentum: None }, // anti-momentum: -3.7% persistence edge
    CoinConfig { symbol: "BTC/USDT", binance_symbol: "BTCUSDT", name: "BTC",
        long_strat: LongStrat::BbBounce, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme,
        complement_strat: ComplementStrat::KstCross, complement_z_filter: -0.5,
        momentum: None },
];

pub fn coin_index(name: &str) -> Option<usize> {
    COINS.iter().position(|c| c.name == name)
}
