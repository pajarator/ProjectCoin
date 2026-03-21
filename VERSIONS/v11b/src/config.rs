use crate::strategies::{IsoShortStrat, LongStrat, ShortStrat};

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

// RUN35: scalp exit enhancements
pub const SCALP_BE_ACTIVATION: f64 = 0.004; // 0.40% gain activates breakeven stop (SL moves to entry)
pub const SCALP_STOCH_EXIT: f64 = 50.0;     // close scalp when stoch_k crosses 50 while profitable

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
}

pub const COINS: [CoinConfig; 18] = [
    CoinConfig { symbol: "DASH/USDT", binance_symbol: "DASHUSDT", name: "DASH",
        long_strat: LongStrat::OuMeanRev, short_strat: ShortStrat::ShortMeanRev,
        iso_short_strat: IsoShortStrat::IsoDivergence },
    CoinConfig { symbol: "UNI/USDT", binance_symbol: "UNIUSDT", name: "UNI",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRelativeZ },
    CoinConfig { symbol: "NEAR/USDT", binance_symbol: "NEARUSDT", name: "NEAR",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme },
    CoinConfig { symbol: "ADA/USDT", binance_symbol: "ADAUSDT", name: "ADA",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoDivergence },
    CoinConfig { symbol: "LTC/USDT", binance_symbol: "LTCUSDT", name: "LTC",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortMeanRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme },
    CoinConfig { symbol: "SHIB/USDT", binance_symbol: "SHIBUSDT", name: "SHIB",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortVwapRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme },
    CoinConfig { symbol: "LINK/USDT", binance_symbol: "LINKUSDT", name: "LINK",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoRelativeZ },
    CoinConfig { symbol: "ETH/USDT", binance_symbol: "ETHUSDT", name: "ETH",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme },
    CoinConfig { symbol: "DOT/USDT", binance_symbol: "DOTUSDT", name: "DOT",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortVwapRev,
        iso_short_strat: IsoShortStrat::IsoRelativeZ },
    CoinConfig { symbol: "XRP/USDT", binance_symbol: "XRPUSDT", name: "XRP",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme },
    CoinConfig { symbol: "ATOM/USDT", binance_symbol: "ATOMUSDT", name: "ATOM",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRelativeZ },
    CoinConfig { symbol: "SOL/USDT", binance_symbol: "SOLUSDT", name: "SOL",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme },
    CoinConfig { symbol: "DOGE/USDT", binance_symbol: "DOGEUSDT", name: "DOGE",
        long_strat: LongStrat::BbBounce, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoDivergence },
    CoinConfig { symbol: "XLM/USDT", binance_symbol: "XLMUSDT", name: "XLM",
        long_strat: LongStrat::DualRsi, short_strat: ShortStrat::ShortMeanRev,
        iso_short_strat: IsoShortStrat::IsoRelativeZ },
    CoinConfig { symbol: "AVAX/USDT", binance_symbol: "AVAXUSDT", name: "AVAX",
        long_strat: LongStrat::AdrReversal, short_strat: ShortStrat::ShortBbBounce,
        iso_short_strat: IsoShortStrat::IsoRelativeZ },
    CoinConfig { symbol: "ALGO/USDT", binance_symbol: "ALGOUSDT", name: "ALGO",
        long_strat: LongStrat::AdrReversal, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme },
    CoinConfig { symbol: "BNB/USDT", binance_symbol: "BNBUSDT", name: "BNB",
        long_strat: LongStrat::VwapReversion, short_strat: ShortStrat::ShortVwapRev,
        iso_short_strat: IsoShortStrat::IsoDivergence },
    CoinConfig { symbol: "BTC/USDT", binance_symbol: "BTCUSDT", name: "BTC",
        long_strat: LongStrat::BbBounce, short_strat: ShortStrat::ShortAdrRev,
        iso_short_strat: IsoShortStrat::IsoRsiExtreme },
];

pub fn coin_index(name: &str) -> Option<usize> {
    COINS.iter().position(|c| c.name == name)
}
