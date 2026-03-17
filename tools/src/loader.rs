#[derive(Debug, Clone, Default)]
pub struct Bar {
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
}

// COINCLAW v13: 18 active coins and their primary long strategies
pub const COIN_STRATEGIES: [(&str, &str); 18] = [
    ("DASH", "mean_rev"),    // OuMeanRev → approximated as MeanRev
    ("UNI",  "vwap_rev"),
    ("NEAR", "vwap_rev"),
    ("ADA",  "vwap_rev"),
    ("LTC",  "vwap_rev"),
    ("SHIB", "vwap_rev"),
    ("LINK", "vwap_rev"),
    ("ETH",  "vwap_rev"),
    ("DOT",  "vwap_rev"),
    ("XRP",  "vwap_rev"),
    ("ATOM", "vwap_rev"),
    ("SOL",  "vwap_rev"),
    ("DOGE", "bb_bounce"),
    ("XLM",  "dual_rsi"),
    ("AVAX", "adr_rev"),
    ("ALGO", "adr_rev"),
    ("BNB",  "vwap_rev"),
    ("BTC",  "bb_bounce"),
];

pub fn load_ohlcv(coin: &str) -> Vec<Bar> {
    let path = format!("data_cache/{}_USDT_15m_1year.csv", coin);
    let mut rdr = csv::Reader::from_path(&path)
        .unwrap_or_else(|_| panic!("Cannot open {}", path));
    let mut bars = Vec::with_capacity(35_100);
    for result in rdr.records() {
        let r = result.expect("CSV parse error");
        bars.push(Bar {
            o: r[1].parse().unwrap_or(0.0),
            h: r[2].parse().unwrap_or(0.0),
            l: r[3].parse().unwrap_or(0.0),
            c: r[4].parse().unwrap_or(0.0),
            v: r[5].parse().unwrap_or(0.0),
        });
    }
    bars
}
