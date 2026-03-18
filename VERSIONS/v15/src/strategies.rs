use crate::config;
use crate::coordinator::MarketCtx;
use crate::indicators::{Ind15m, Ind1m};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LongStrat {
    VwapReversion,
    BbBounce,
    AdrReversal,
    DualRsi,
    MeanReversion,
    OuMeanRev,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortStrat {
    ShortMeanRev,
    ShortAdrRev,
    ShortBbBounce,
    ShortVwapRev,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsoShortStrat {
    IsoRelativeZ,
    IsoRsiExtreme,
    IsoDivergence,
    IsoMeanRev,
    IsoVwapRev,
    IsoBbBounce,
    IsoAdrRev,
    IsoVolSpike,
    IsoBbSqueeze,
}

/// RUN13 complementary strategies — fire when primary long_entry doesn't
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplementStrat {
    LaguerreRsi { gamma_idx: u8 },  // 0=0.5, 1=0.6, 2=0.7, 3=0.8
    KalmanFilter,
    KstCross,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Long,
    Short,
}

impl std::fmt::Display for LongStrat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::VwapReversion => write!(f, "vwap_rev"),
            Self::BbBounce => write!(f, "bb_bounce"),
            Self::AdrReversal => write!(f, "adr_rev"),
            Self::DualRsi => write!(f, "dual_rsi"),
            Self::MeanReversion => write!(f, "mean_rev"),
            Self::OuMeanRev => write!(f, "ou_mean_rev"),
        }
    }
}

impl std::fmt::Display for ShortStrat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ShortMeanRev => write!(f, "short_mean_rev"),
            Self::ShortAdrRev => write!(f, "short_adr_rev"),
            Self::ShortBbBounce => write!(f, "short_bb_bounce"),
            Self::ShortVwapRev => write!(f, "short_vwap_rev"),
        }
    }
}

impl std::fmt::Display for IsoShortStrat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::IsoRelativeZ => write!(f, "iso_relative_z"),
            Self::IsoRsiExtreme => write!(f, "iso_rsi_extreme"),
            Self::IsoDivergence => write!(f, "iso_divergence"),
            Self::IsoMeanRev => write!(f, "iso_mean_rev"),
            Self::IsoVwapRev => write!(f, "iso_vwap_rev"),
            Self::IsoBbBounce => write!(f, "iso_bb_bounce"),
            Self::IsoAdrRev => write!(f, "iso_adr_rev"),
            Self::IsoVolSpike => write!(f, "iso_vol_spike"),
            Self::IsoBbSqueeze => write!(f, "iso_bb_squeeze"),
        }
    }
}

impl std::fmt::Display for ComplementStrat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::LaguerreRsi { gamma_idx } => {
                let g = match gamma_idx { 0 => "0.5", 1 => "0.6", 2 => "0.7", _ => "0.8" };
                write!(f, "laguerre_rsi_{}", g)
            }
            Self::KalmanFilter => write!(f, "kalman_filter"),
            Self::KstCross => write!(f, "kst_cross"),
            Self::None => write!(f, "none"),
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Long => write!(f, "long"),
            Self::Short => write!(f, "short"),
        }
    }
}

pub fn long_entry(ind: &Ind15m, strat: LongStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p > ind.sma20 || ind.z > 0.5 { return false; }
    match strat {
        LongStrat::VwapReversion => {
            ind.z < -1.5 && ind.p < ind.vwap && ind.vol > ind.vol_ma * 1.2
        }
        LongStrat::BbBounce => {
            ind.p <= ind.bb_lo * 1.02 && ind.vol > ind.vol_ma * 1.3
        }
        LongStrat::DualRsi => {
            ind.rsi < 40.0 && ind.rsi7 < 30.0 && ind.sma9 > ind.sma20
        }
        LongStrat::AdrReversal => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_lo.is_nan() && range > 0.0
                && ind.p <= ind.adr_lo + range * 0.25
                && ind.vol > ind.vol_ma * 1.1
        }
        LongStrat::MeanReversion => ind.z < -1.5,
        LongStrat::OuMeanRev => {
            // RUN11c: Ornstein-Uhlenbeck confirmed mean-reverting regime + extended deviation
            // Halflife in [10, 100] = reasonable mean-reversion speed
            // Deviation below mean by 2.0 standard deviations
            !ind.ou_halflife.is_nan() && !ind.ou_deviation.is_nan()
                && ind.std20 > 0.0
                && ind.ou_halflife >= config::OU_MIN_HALFLIFE
                && ind.ou_halflife <= config::OU_MAX_HALFLIFE
                && (ind.ou_deviation / ind.std20) < -config::OU_DEV_THRESHOLD
        }
    }
}

pub fn short_entry(ind: &Ind15m, strat: ShortStrat) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p < ind.sma20 || ind.z < -0.5 { return false; }
    match strat {
        ShortStrat::ShortVwapRev => {
            ind.z > 1.5 && ind.p > ind.vwap && ind.vol > ind.vol_ma * 1.2
        }
        ShortStrat::ShortBbBounce => {
            ind.p >= ind.bb_hi * 0.98 && ind.vol > ind.vol_ma * 1.3
        }
        ShortStrat::ShortMeanRev => ind.z > 1.5,
        ShortStrat::ShortAdrRev => {
            let range = ind.adr_hi - ind.adr_lo;
            !ind.adr_hi.is_nan() && range > 0.0
                && ind.p >= ind.adr_hi - range * 0.25
                && ind.vol > ind.vol_ma * 1.1
        }
    }
}

pub fn iso_short_entry(ind: &Ind15m, strat: IsoShortStrat, ctx: &MarketCtx) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if ind.p < ind.sma20 || ind.z < -0.5 { return false; }
    match strat {
        IsoShortStrat::IsoMeanRev => ind.z > config::ISO_Z_THRESHOLD,
        IsoShortStrat::IsoVwapRev => {
            ind.z > config::ISO_Z_THRESHOLD
                && ind.p > ind.vwap
                && ind.vol > ind.vol_ma * config::ISO_VOL_MULT
        }
        IsoShortStrat::IsoBbBounce => {
            ind.p >= ind.bb_hi * config::ISO_BB_MARGIN
                && ind.vol > ind.vol_ma * (config::ISO_VOL_MULT + 0.1)
        }
        IsoShortStrat::IsoAdrRev => {
            let range = ind.adr_hi - ind.adr_lo;
            range > 0.0
                && ind.p >= ind.adr_hi - range * config::ISO_ADR_PCT
                && ind.vol > ind.vol_ma * config::ISO_VOL_MULT
        }
        IsoShortStrat::IsoRelativeZ => {
            ctx.avg_z_valid && ind.z > ctx.avg_z + config::ISO_Z_SPREAD
        }
        IsoShortStrat::IsoRsiExtreme => {
            ctx.avg_rsi_valid && !ind.rsi.is_nan()
                && ind.rsi > config::ISO_RSI_THRESHOLD
                && ctx.avg_rsi < 55.0
        }
        IsoShortStrat::IsoDivergence => {
            ctx.btc_z_valid && ind.z > config::ISO_Z_THRESHOLD && ctx.btc_z < 0.0
        }
        IsoShortStrat::IsoVolSpike => {
            ind.z > 1.0 && ind.vol > ind.vol_ma * config::ISO_VOL_SPIKE_MULT
        }
        IsoShortStrat::IsoBbSqueeze => {
            !ind.bb_width_avg.is_nan() && ind.bb_width_avg > 0.0
                && ind.p >= ind.bb_hi * 0.98
                && ind.bb_width < ind.bb_width_avg * config::ISO_SQUEEZE_FACTOR
        }
    }
}

pub fn scalp_entry(ind: &Ind1m) -> Option<(Direction, &'static str)> {
    if !ind.valid || ind.vol_ma == 0.0 { return None; }

    let vol_r = ind.vol / ind.vol_ma;
    let rsi_low = config::SCALP_RSI_EXTREME;
    let rsi_high = 100.0 - config::SCALP_RSI_EXTREME;

    // 1. scalp_vol_spike_rev (re-enabled v10 with F6 filter + wide TP)
    if vol_r > config::SCALP_VOL_MULT {
        if ind.rsi < rsi_low && passes_f6(ind, Direction::Long) {
            return Some((Direction::Long, "scalp_vol_spike_rev"));
        }
        if ind.rsi > rsi_high && passes_f6(ind, Direction::Short) {
            return Some((Direction::Short, "scalp_vol_spike_rev"));
        }
    }

    // 2. scalp_stoch_cross
    if !ind.stoch_k.is_nan() && !ind.stoch_d.is_nan()
        && !ind.stoch_k_prev.is_nan() && !ind.stoch_d_prev.is_nan()
    {
        let stoch_lo = config::SCALP_STOCH_EXTREME;
        let stoch_hi = 100.0 - config::SCALP_STOCH_EXTREME;
        if ind.stoch_k_prev <= ind.stoch_d_prev && ind.stoch_k > ind.stoch_d
            && ind.stoch_k < stoch_lo && ind.stoch_d < stoch_lo
            && passes_f6(ind, Direction::Long)
        {
            return Some((Direction::Long, "scalp_stoch_cross"));
        }
        if ind.stoch_k_prev >= ind.stoch_d_prev && ind.stoch_k < ind.stoch_d
            && ind.stoch_k > stoch_hi && ind.stoch_d > stoch_hi
            && passes_f6(ind, Direction::Short)
        {
            return Some((Direction::Short, "scalp_stoch_cross"));
        }
    }

    None
}

/// Fix #3: Scalp entry using only stoch_cross (vol_spike_rev removed — 5.9% live WR)
pub fn scalp_entry_stoch_only(ind: &Ind1m) -> Option<(Direction, &'static str)> {
    if !ind.valid || ind.vol_ma == 0.0 { return None; }

    if !ind.stoch_k.is_nan() && !ind.stoch_d.is_nan()
        && !ind.stoch_k_prev.is_nan() && !ind.stoch_d_prev.is_nan()
    {
        let stoch_lo = config::SCALP_STOCH_EXTREME;
        let stoch_hi = 100.0 - config::SCALP_STOCH_EXTREME;
        if ind.stoch_k_prev <= ind.stoch_d_prev && ind.stoch_k > ind.stoch_d
            && ind.stoch_k < stoch_lo && ind.stoch_d < stoch_lo
            && passes_f6(ind, Direction::Long)
        {
            return Some((Direction::Long, "scalp_stoch_cross"));
        }
        if ind.stoch_k_prev >= ind.stoch_d_prev && ind.stoch_k < ind.stoch_d
            && ind.stoch_k > stoch_hi && ind.stoch_d > stoch_hi
            && passes_f6(ind, Direction::Short)
        {
            return Some((Direction::Short, "scalp_stoch_cross"));
        }
    }

    None
}

/// F6 pre-entry filter: counter-momentum + active candles (RUN10.1/10.2 validated OOS)
/// dir_roc_3 < -0.195 means price moved against our trade direction (counter-momentum entry)
/// avg_body_3 > 0.072 means candles are active, not dojis
fn passes_f6(ind: &Ind1m, dir: Direction) -> bool {
    if ind.roc_3.is_nan() || ind.avg_body_3.is_nan() { return false; }
    let sign = match dir { Direction::Long => 1.0, Direction::Short => -1.0 };
    let dir_roc_3 = ind.roc_3 * sign;
    dir_roc_3 < config::F6_DIR_ROC_3 && ind.avg_body_3 > config::F6_AVG_BODY_3
}

/// RUN13 complement entry — fires when primary long_entry didn't fire.
/// These capture oversold conditions the primary strategy misses.
pub fn complement_entry(ind: &Ind15m, strat: ComplementStrat, z_filter: f64) -> bool {
    if !ind.valid || ind.z.is_nan() { return false; }
    if matches!(strat, ComplementStrat::None) { return false; }
    // z-score filter: price must be depressed enough
    if ind.z > z_filter { return false; }
    // Must be below SMA20 (same guard as primary longs)
    if ind.p > ind.sma20 { return false; }

    match strat {
        ComplementStrat::LaguerreRsi { gamma_idx } => {
            let (cur, prev) = match gamma_idx {
                0 => (ind.laguerre_rsi_05, ind.laguerre_rsi_05_prev),
                1 => (ind.laguerre_rsi_06, ind.laguerre_rsi_06_prev),
                2 => (ind.laguerre_rsi_07, ind.laguerre_rsi_07_prev),
                _ => (ind.laguerre_rsi_08, ind.laguerre_rsi_08_prev),
            };
            if cur.is_nan() || prev.is_nan() { return false; }
            // Entry: Laguerre RSI < 20 and rising
            cur < 20.0 && cur > prev
        }
        ComplementStrat::KalmanFilter => {
            if ind.kalman_est.is_nan() || ind.kalman_var.is_nan() { return false; }
            let std_dev = ind.kalman_var.sqrt();
            // Entry: price below Kalman estimate by 2 std devs
            ind.p < ind.kalman_est - 2.0 * std_dev && ind.p < ind.sma20
        }
        ComplementStrat::KstCross => {
            if ind.kst.is_nan() || ind.kst_signal.is_nan()
                || ind.kst_prev.is_nan() || ind.kst_signal_prev.is_nan()
            {
                return false;
            }
            // Entry: KST crosses above signal from below
            ind.kst_prev <= ind.kst_signal_prev && ind.kst > ind.kst_signal
        }
        ComplementStrat::None => false,
    }
}

/// v15: Full scalp entry with 3 strategies (v9 behavior restored)
/// Strategies: vol_spike_rev + stoch_cross + bb_squeeze_break
/// F6 filter gate on reversal strategies; bb_squeeze_break is a breakout (no F6)
pub fn scalp_entry_with_price(ind: &Ind1m, price: f64) -> Option<(Direction, &'static str)> {
    if !ind.valid || ind.vol_ma == 0.0 { return None; }

    let vol_r = ind.vol / ind.vol_ma;
    let rsi_low = config::SCALP_RSI_EXTREME;
    let rsi_high = 100.0 - config::SCALP_RSI_EXTREME;

    // 1. scalp_vol_spike_rev (F6 filtered — counter-momentum reversal)
    if vol_r > config::SCALP_VOL_MULT {
        if ind.rsi < rsi_low && passes_f6(ind, Direction::Long) {
            return Some((Direction::Long, "scalp_vol_spike_rev"));
        }
        if ind.rsi > rsi_high && passes_f6(ind, Direction::Short) {
            return Some((Direction::Short, "scalp_vol_spike_rev"));
        }
    }

    // 2. scalp_stoch_cross (F6 filtered)
    if !ind.stoch_k.is_nan() && !ind.stoch_d.is_nan()
        && !ind.stoch_k_prev.is_nan() && !ind.stoch_d_prev.is_nan()
    {
        let stoch_lo = config::SCALP_STOCH_EXTREME;
        let stoch_hi = 100.0 - config::SCALP_STOCH_EXTREME;
        if ind.stoch_k_prev <= ind.stoch_d_prev && ind.stoch_k > ind.stoch_d
            && ind.stoch_k < stoch_lo && ind.stoch_d < stoch_lo
            && passes_f6(ind, Direction::Long)
        {
            return Some((Direction::Long, "scalp_stoch_cross"));
        }
        if ind.stoch_k_prev >= ind.stoch_d_prev && ind.stoch_k < ind.stoch_d
            && ind.stoch_k > stoch_hi && ind.stoch_d > stoch_hi
            && passes_f6(ind, Direction::Short)
        {
            return Some((Direction::Short, "scalp_stoch_cross"));
        }
    }

    // 3. scalp_bb_squeeze_break (v9 breakout signal — no F6, direction aligns with breakout)
    if !ind.bb_width_avg.is_nan() && ind.bb_width_avg > 0.0 && !ind.bb_upper.is_nan() {
        let squeeze = ind.bb_width < ind.bb_width_avg * config::SCALP_BB_SQUEEZE;
        if squeeze && vol_r > 2.0 {
            if price > ind.bb_upper {
                return Some((Direction::Long, "scalp_bb_squeeze_break"));
            }
            if price < ind.bb_lower {
                return Some((Direction::Short, "scalp_bb_squeeze_break"));
            }
        }
    }

    None
}
