use crate::indicators::Candle;
use reqwest::Client;

const BINANCE_BASE: &str = "https://api.binance.com";

pub async fn fetch_klines(
    client: &Client,
    symbol: &str,
    interval: &str,
    limit: u32,
) -> Result<Vec<Candle>, String> {
    let url = format!(
        "{}/api/v3/klines?symbol={}&interval={}&limit={}",
        BINANCE_BASE, symbol, interval, limit
    );

    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("fetch {}: {}", symbol, e))?;

    if !resp.status().is_success() {
        return Err(format!("fetch {} HTTP {}", symbol, resp.status()));
    }

    let data: Vec<Vec<serde_json::Value>> = resp
        .json()
        .await
        .map_err(|e| format!("parse {}: {}", symbol, e))?;

    let mut candles = Vec::with_capacity(data.len());
    for row in &data {
        if row.len() < 6 { continue; }
        let o = parse_val(&row[1]);
        let h = parse_val(&row[2]);
        let l = parse_val(&row[3]);
        let c = parse_val(&row[4]);
        let v = parse_val(&row[5]);
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() || v.is_nan() {
            continue;
        }
        candles.push(Candle { o, h, l, c, v });
    }

    Ok(candles)
}

fn parse_val(v: &serde_json::Value) -> f64 {
    match v {
        serde_json::Value::String(s) => s.parse().unwrap_or(f64::NAN),
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}
