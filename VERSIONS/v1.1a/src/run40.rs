/// RUN40 — BTC Dominance Scalp Filter: Cross-Coin Relative Strength Gate
///
/// Hypothesis: When BTC significantly outperforms alts (btc_z >> avg_z), scalp LONG
/// entries on alts face a headwind. When BTC underperforms (btc_z << avg_z),
/// scalp LONGs on alts have higher success rate.
///
/// Grid:
///   BTC_DOM_SCALE_LONG:  [0.5, 1.0, 1.5, 2.0]  σ threshold to block LONG scalps
///   BTC_DOM_SCALE_SHORT: [0.5, 1.0, 1.5, 2.0]  σ threshold to block SHORT scalps
///
/// Uses 1m data for scalp simulation, 15m for BTC dominance signal.
/// Simplification: apply 15m BTC dominance gate to all 1m scalp entries in that bar.
///
/// Run: cargo run --release --features run40 -- --run40

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// ── Constants ────────────────────────────────────────────────────────────────
const SCALP_RISK: f64 = 0.05;
const LEVERAGE: f64 = 5.0;
const INITIAL_BAL: f64 = 100.0;
const SCALP_TP: f64 = 0.008;
const SCALP_SL: f64 = 0.001;
const SCALP_MAX_HOLD: u32 = 480;
const SCALP_VOL_MULT: f64 = 3.5;
const SCALP_RSI_EXTREME: f64 = 20.0;
const SCALP_STOCH_EXTREME: f64 = 5.0;
const SCALP_BB_SQUEEZE: f64 = 0.4;
const F6_DIR_ROC_3: f64 = -0.195;
const F6_AVG_BODY_3: f64 = 0.072;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];

// ── Grid ────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
struct BtcDomCfg {
    long_thresh: f64,  // block LONG when btc_z > avg_z + this
    short_thresh: f64, // block SHORT when btc_z < avg_z - this
}

impl BtcDomCfg {
    fn label(&self) -> String {
        format!("L{:.1}_S{:.1}", self.long_thresh, self.short_thresh)
    }
}

fn build_grid() -> Vec<BtcDomCfg> {
    let mut grid = Vec::new();
    // Baseline: no filter
    grid.push(BtcDomCfg { long_thresh: 999.0, short_thresh: 999.0 });
    for lt in [0.5, 1.0, 1.5, 2.0] {
        for st in [0.5, 1.0, 1.5, 2.0] {
            grid.push(BtcDomCfg { long_thresh: lt, short_thresh: st });
        }
    }
    grid
}

// ── Data structures ─────────────────────────────────────────────────────────
struct CoinData1m {
    name: &'static str,
    close: Vec<f64>,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    vol: Vec<f64>,
    rsi: Vec<f64>,
    vol_ma: Vec<f64>,
    stoch_k: Vec<f64>,
    stoch_d: Vec<f64>,
    bb_upper: Vec<f64>,
    bb_lower: Vec<f64>,
    bb_width: Vec<f64>,
    bb_width_avg: Vec<f64>,
    roc_3: Vec<f64>,
    avg_body_3: Vec<f64>,
}

struct ScalpPos {
    dir: i8, entry: f64, notional: f64, bars_held: u32,
}

#[derive(Serialize)]
struct CoinResult {
    coin: String,
    trades: usize,
    wins: usize,
    losses: usize,
    flats: usize,
    wr: f64,
    pnl: f64,
    pf: f64,
    avg_win: f64,
    avg_loss: f64,
    trades_blocked: usize,
}

#[derive(Serialize)]
struct ConfigResult {
    label: String,
    total_pnl: f64,
    portfolio_wr: f64,
    total_trades: usize,
    total_blocked: usize,
    pf: f64,
    coins: Vec<CoinResult>,
    is_baseline: bool,
}

#[derive(Serialize)]
struct Output { notes: String, configs: Vec<ConfigResult> }

// ── Rolling helpers ───────────────────────────────────────────────────────────
fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n]; let mut sum = 0.0;
    for i in 0..n { sum += data[i]; if i>=w { sum -= data[i-w]; } if i+1>=w { out[i]=sum/w as f64; } }
    out
}
fn rstd(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { let s=&data[i+1-w..=i]; let m=s.iter().sum::<f64>()/w as f64; let v=s.iter().map(|x|(x-m).powi(2)).sum::<f64>()/w as f64; out[i]=v.sqrt(); }
    out
}
fn rmin(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { out[i]=data[i+1-w..=i].iter().cloned().fold(f64::INFINITY, f64::min); }
    out
}
fn rmax(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { out[i]=data[i+1-w..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max); }
    out
}
fn rsi_calc(c: &[f64], period: usize) -> Vec<f64> {
    let n = c.len(); let mut out = vec![f64::NAN; n];
    if n < period+1 { return out; }
    let mut gains = vec![0.0; n]; let mut losses = vec![0.0; n];
    for i in 1..n { let d=c[i]-c[i-1]; if d>0.0{gains[i]=d;}else{losses[i]=-d;} }
    let ag=rmean(&gains,period); let al=rmean(&losses,period);
    for i in 0..n { if !ag[i].is_nan()&&!al[i].is_nan(){out[i]=if al[i]==0.0{100.0}else{100.0-100.0/(1.0+ag[i]/al[i])}; } }
    out
}

// ── CSV loaders ────────────────────────────────────────────────────────────────
fn load_1m(coin: &str) -> Option<(Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>)> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_1m_5months.csv", coin);
    let data = match std::fs::read_to_string(&path) { Ok(d)=>d, Err(_)=>return None };
    let mut o=Vec::new();let mut h=Vec::new();let mut l=Vec::new();let mut c=Vec::new();let mut v=Vec::new();
    for line in data.lines().skip(1) { let mut it=line.splitn(7,','); let _ts=it.next(); let oo: f64=it.next()?.parse().ok()?; let hh: f64=it.next()?.parse().ok()?; let ll: f64=it.next()?.parse().ok()?; let cc: f64=it.next()?.parse().ok()?; let vv: f64=it.next()?.parse().ok()?; if oo.is_nan()||hh.is_nan()||ll.is_nan()||cc.is_nan()||vv.is_nan(){continue;} o.push(oo);h.push(hh);l.push(ll);c.push(cc);v.push(vv); }
    if c.len()<100 {return None;} Some((o,h,l,c,v))
}

fn load_15m(coin: &str) -> Option<Vec<f64>> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = match std::fs::read_to_string(&path) { Ok(d)=>d, Err(_)=>return None };
    let mut c = Vec::new();
    for line in data.lines().skip(1) { let mut it=line.splitn(7,','); let _ts=it.next(); let _o: f64=it.next()?.parse().ok()?; let _h: f64=it.next()?.parse().ok()?; let _l: f64=it.next()?.parse().ok()?; let cc: f64=it.next()?.parse().ok()?; let _v: f64=it.next()?.parse().ok()?; if cc.is_nan(){continue;} c.push(cc); }
    if c.len()<100 {return None;} Some(c)
}

// ── Indicators ────────────────────────────────────────────────────────────────
fn compute_1m_data(name: &'static str, o: Vec<f64>, h: Vec<f64>, l: Vec<f64>, c: Vec<f64>, v: Vec<f64>) -> CoinData1m {
    let n = c.len();
    let rsi = rsi_calc(&c, 14);
    let vol_ma = rmean(&v, 20);
    let ll = rmin(&l, 14); let hh = rmax(&h, 14);
    let mut stoch_k = vec![f64::NAN; n];
    for i in 0..n { if !ll[i].is_nan()&&hh[i]>ll[i]{stoch_k[i]=100.0*(c[i]-ll[i])/(hh[i]-ll[i]);} }
    let stoch_d = rmean(&stoch_k, 3);
    let bb_sma = rmean(&c, 20); let bb_std = rstd(&c, 20);
    let mut bb_upper = vec![f64::NAN; n]; let mut bb_lower = vec![f64::NAN; n]; let mut bb_width_raw = vec![f64::NAN; n];
    for i in 0..n { if !bb_sma[i].is_nan()&&!bb_std[i].is_nan(){bb_upper[i]=bb_sma[i]+2.0*bb_std[i];bb_lower[i]=bb_sma[i]-2.0*bb_std[i];bb_width_raw[i]=bb_upper[i]-bb_lower[i];} }
    let bb_width_avg = rmean(&bb_width_raw, 20);
    let mut roc_3 = vec![f64::NAN; n];
    for i in 3..n { if c[i-3]>0.0{roc_3[i]=(c[i]-c[i-3])/c[i-3]*100.0;} }
    let mut avg_body_3 = vec![f64::NAN; n];
    for i in 2..n { let b0=(c[i]-o[i]).abs()/c[i]*100.0; let b1=(c[i-1]-o[i-1]).abs()/c[i-1]*100.0; let b2=(c[i-2]-o[i-2]).abs()/c[i-2]*100.0; avg_body_3[i]=(b0+b1+b2)/3.0; }
    CoinData1m{name,close:c,open:o,high:h,low:l,vol:v,rsi,vol_ma,stoch_k,stoch_d,bb_upper,bb_lower,bb_width:bb_width_raw,bb_width_avg,roc_3,avg_body_3}
}

// ── BTC dominance signal ──────────────────────────────────────────────────────
// Returns btc_z and avg_z at each 15m bar index (sampled from 1m data)
fn compute_btc_dominance(btc_closes_15m: &[f64], all_closes_15m: &[Vec<f64>]) -> (Vec<f64>, Vec<f64>) {
    let n = btc_closes_15m.len();
    let mut btc_z = vec![f64::NAN; n];
    let mut avg_z = vec![f64::NAN; n];

    for i in 20..n {
        let btc_c = btc_closes_15m[i];
        let mut zscores = Vec::new();
        for coin_c in all_closes_15m {
            if i < coin_c.len() {
                let window = &coin_c[i+1-20..=i];
                let mean = window.iter().sum::<f64>()/20.0;
                let std = (window.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/20.0).sqrt();
                if std > 0.0 { zscores.push((btc_c - mean)/std); }
            }
        }
        if !zscores.is_empty() {
            btc_z[i] = zscores[0]; // BTC's z-score (first coin = BTC)
            avg_z[i] = zscores.iter().sum::<f64>()/zscores.len() as f64;
        }
    }
    (btc_z, avg_z)
}

// ── Scalp signal ─────────────────────────────────────────────────────────────
fn f6_pass(d: &CoinData1m, i: usize, dir: i8) -> bool {
    if d.roc_3[i].is_nan()||d.avg_body_3[i].is_nan() { return false; }
    let sign = if dir==1 {1.0}else{-1.0};
    d.roc_3[i]*sign < F6_DIR_ROC_3 && d.avg_body_3[i] > F6_AVG_BODY_3
}

fn scalp_signal(d: &CoinData1m, i: usize) -> Option<i8> {
    if i<40 { return None; }
    if d.vol_ma[i].is_nan()||d.vol_ma[i]<=0.0 { return None; }
    if d.rsi[i].is_nan() { return None; }
    let vol_r = d.vol[i]/d.vol_ma[i];
    let rsi_lo = SCALP_RSI_EXTREME;
    let rsi_hi = 100.0-SCALP_RSI_EXTREME;
    // vol_spike_rev
    if vol_r > SCALP_VOL_MULT {
        if d.rsi[i] < rsi_lo && f6_pass(d, i, 1) { return Some(1); }
        if d.rsi[i] > rsi_hi && f6_pass(d, i, -1) { return Some(-1); }
    }
    // stoch_cross
    if i>=1 {
        let sk=d.stoch_k[i]; let sd=d.stoch_d[i]; let skp=d.stoch_k[i-1]; let sdp=d.stoch_d[i-1];
        if !sk.is_nan()&&!sd.is_nan()&&!skp.is_nan()&&!sdp.is_nan() {
            let lo=SCALP_STOCH_EXTREME; let hi=100.0-SCALP_STOCH_EXTREME;
            if skp<=sdp && sk>sd && sk<lo && sd<lo && f6_pass(d,i,1) { return Some(1); }
            if skp>=sdp && sk<sd && sk>hi && sd>hi && f6_pass(d,i,-1) { return Some(-1); }
        }
    }
    // bb_squeeze_break
    if !d.bb_width_avg[i].is_nan()&&d.bb_width_avg[i]>0.0&&!d.bb_upper[i].is_nan() {
        let squeeze = d.bb_width[i] < d.bb_width_avg[i]*SCALP_BB_SQUEEZE;
        if squeeze && vol_r>2.0 {
            if d.close[i]>d.bb_upper[i] { return Some(1); }
            if d.close[i]<d.bb_lower[i] { return Some(-1); }
        }
    }
    None
}

// ── Simulation ────────────────────────────────────────────────────────────────
struct SimResult { trades: usize, wins: usize, losses: usize, flats: usize, pnl: f64, win_pnls: Vec<f64>, loss_pnls: Vec<f64>, blocked: usize }

fn simulate_coin(d: &CoinData1m, cfg: &BtcDomCfg, btc_z_arr: &[f64], avg_z_arr: &[f64]) -> SimResult {
    let n = d.close.len();
    let mut bal = INITIAL_BAL;
    let mut pos: Option<ScalpPos> = None;
    let mut cooldown = 0usize;
    let mut win_pnls = Vec::new(); let mut loss_pnls = Vec::new();
    let mut flats = 0usize; let mut blocked = 0usize;
    let bars_per_15m = 15usize;

    for i in 1..n {
        if let Some(ref mut p) = pos {
            let pct = if p.dir==1{(d.close[i]-p.entry)/p.entry}else{(p.entry-d.close[i])/p.entry};
            let mut closed = false;
            let mut exit_pct = 0.0;
            if pct <= -SCALP_SL { exit_pct = -SCALP_SL; closed=true; }
            else if pct >= SCALP_TP { exit_pct = SCALP_TP; closed=true; }
            p.bars_held += 1;
            if !closed && p.bars_held >= SCALP_MAX_HOLD { exit_pct = pct; closed=true; }
            if closed {
                let net = p.notional * exit_pct;
                bal += net;
                if net>1e-10{win_pnls.push(net);}else if net< -1e-10{loss_pnls.push(net);}else{flats+=1;}
                pos = None; cooldown = 2;
            }
        } else if cooldown>0 { cooldown -= 1; }
        else {
            // BTC dominance filter — look up 15m window for this 1m bar
            let dominated = if cfg.long_thresh < 999.0 || cfg.short_thresh < 999.0 {
                let m15_idx = i / bars_per_15m;
                if m15_idx < btc_z_arr.len() {
                    let bz = btc_z_arr[m15_idx];
                    let az = avg_z_arr[m15_idx];
                    if !bz.is_nan() && !az.is_nan() {
                        if cfg.long_thresh < 999.0 && bz > az + cfg.long_thresh { true }
                        else if cfg.short_thresh < 999.0 && bz < az - cfg.short_thresh { true }
                        else { false }
                    } else { false }
                } else { false }
            } else { false };

            if dominated {
                blocked += 1;
            } else {
                if let Some(dir) = scalp_signal(d, i) {
                    if i+1 < n {
                        let entry_price = d.open[i+1];
                        if entry_price>0.0 {
                            pos = Some(ScalpPos{dir, entry: entry_price, notional: bal*SCALP_RISK*LEVERAGE, bars_held:0});
                        }
                    }
                }
            }
        }
    }
    if let Some(ref p)=pos { let pct=if p.dir==1{(d.close[n-1]-p.entry)/p.entry}else{(p.entry-d.close[n-1])/p.entry}; let net=p.notional*pct; bal+=net; if net>1e-10{win_pnls.push(net);}else if net< -1e-10{loss_pnls.push(net);}else{flats+=1;} }
    let total = win_pnls.len()+loss_pnls.len()+flats;
    SimResult{trades:total,wins:win_pnls.len(),losses:loss_pnls.len(),flats,pnl:bal-INITIAL_BAL,win_pnls,loss_pnls,blocked}
}

// ── Entry point ──────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN40 — BTC Dominance Scalp Filter Grid Search");
    eprintln!("BTC_DOM_SCALE_LONG × BTC_DOM_SCALE_SHORT");
    eprintln!();

    // Load 1m data for all coins
    eprintln!("Loading 1m data for {} coins...", N_COINS);
    let mut raw1m: Vec<Option<(Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>)>> = Vec::new();
    for &name in &COIN_NAMES {
        let loaded = load_1m(name);
        if let Some(ref d) = loaded { eprintln!("  {} — {} bars", name, d.3.len()); }
        raw1m.push(loaded);
    }
    let all_ok = raw1m.iter().all(|r| r.is_some());
    if !all_ok { eprintln!("Missing data!"); return; }
    if shutdown.load(Ordering::SeqCst) { return; }

    // Load 15m BTC close for dominance signal
    eprintln!("Loading 15m BTC data for dominance signal...");
    let btc_15m = load_15m("BTC");
    if btc_15m.is_none() { eprintln!("BTC 15m data missing!"); return; }
    let btc_15m = btc_15m.unwrap();

    // Build 15m close samples for all coins (for z-score computation)
    let mut all_15m_closes: Vec<Vec<f64>> = Vec::new();
    let _min_len = btc_15m.len();
    for name in &COIN_NAMES {
        if let Some(c) = load_15m(name) {
            all_15m_closes.push(c);
        } else {
            all_15m_closes.push(vec![]);
        }
    }

    // Compute BTC dominance signal
    let (btc_z_arr, avg_z_arr) = compute_btc_dominance(&btc_15m, &all_15m_closes);
    eprintln!("BTC dominance signal computed: {} bars", btc_15m.len());

    // Compute indicators
    eprintln!("\nComputing 1m indicators...");
    let start = std::time::Instant::now();
    let coin_data: Vec<CoinData1m> = raw1m.into_par_iter().enumerate()
        .map(|(ci, r)| {
            let (o,h,l,c,v) = r.unwrap();
            compute_1m_data(COIN_NAMES[ci], o, h, l, c, v)
        })
        .collect();
    eprintln!("Indicators computed in {:.1}s", start.elapsed().as_secs_f64());

    if shutdown.load(Ordering::SeqCst) { return; }

    // Build grid
    let grid = build_grid();
    eprintln!("\nSimulating {} configs × {} coins...", grid.len(), N_COINS);

    let done = AtomicUsize::new(0);
    let total_cfgs = grid.len();

    // Clone arrays for the parallel closure (each parallel task needs owned copy)
    let btc_z_owned = btc_z_arr.to_vec();
    let avg_z_owned = avg_z_arr.to_vec();

    let results: Vec<ConfigResult> = grid.par_iter().map(|cfg| {
        if shutdown.load(Ordering::SeqCst) {
            return ConfigResult {
                label: cfg.label(), total_pnl: 0.0, portfolio_wr: 0.0,
                total_trades: 0, total_blocked: 0, pf: 0.0, coins: vec![],
                is_baseline: cfg.long_thresh > 900.0,
            };
        }

        let coin_results: Vec<CoinResult> = coin_data.iter()
            .map(|cd| {
                let r = simulate_coin(cd, cfg, &btc_z_owned, &avg_z_owned);
                let wr = if r.trades>0 { r.wins as f64/r.trades as f64*100.0 } else { 0.0 };
                let avg_win = if r.wins>0 { r.win_pnls.iter().sum::<f64>()/r.wins as f64 } else { 0.0 };
                let avg_loss = if r.losses>0 { r.loss_pnls.iter().sum::<f64>()/r.losses as f64 } else { 0.0 };
                let pf = if avg_loss.abs()>1e-8 { avg_win/avg_loss.abs() } else { 0.0 };
                CoinResult {
                    coin: cd.name.to_string(),
                    trades: r.trades, wins: r.wins, losses: r.losses, flats: r.flats,
                    wr, pnl: r.pnl, pf, avg_win, avg_loss,
                    trades_blocked: r.blocked,
                }
            })
            .collect();

        let total_trades: usize = coin_results.iter().map(|c| c.trades).sum();
        let wins_sum: usize = coin_results.iter().map(|c| c.wins).sum();
        let total_pnl: f64 = coin_results.iter().map(|c| c.pnl).sum();
        let total_blocked: usize = coin_results.iter().map(|c| c.trades_blocked).sum();
        let portfolio_wr = if total_trades>0 { wins_sum as f64/total_trades as f64*100.0 } else { 0.0 };
        let wins_total: usize = coin_results.iter().filter(|c|c.wins>0).map(|c|c.wins).sum();
        let losses_total: usize = coin_results.iter().filter(|c|c.losses>0).map(|c|c.losses).sum();
        let avg_win_all = if wins_total>0 { coin_results.iter().filter(|c|c.wins>0).map(|c|c.avg_win*c.wins as f64).sum::<f64>()/wins_total as f64 } else { 0.0 };
        let avg_loss_all = if losses_total>0 { coin_results.iter().filter(|c|c.losses>0).map(|c|c.avg_loss*c.losses as f64).sum::<f64>()/losses_total as f64 } else { 0.0 };
        let pf = if avg_loss_all.abs()>1e-8 { avg_win_all/avg_loss_all.abs() } else { 0.0 };
        let is_baseline = cfg.long_thresh > 900.0;
        let d = done.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("  [{:>2}/{}] {:<18} trades={:>5}  WR={:>5.1}%  PnL={:>+8.2}  PF={:.3}",
            d, total_cfgs, cfg.label(), total_trades, portfolio_wr, total_pnl, pf);

        ConfigResult {
            label: cfg.label(), total_pnl, portfolio_wr, total_trades,
            total_blocked, pf, coins: coin_results, is_baseline,
        }
    }).collect();

    if shutdown.load(Ordering::SeqCst) {
        eprintln!("Interrupted — saving partial...");
        let output = Output { notes: "RUN40 interrupted".to_string(), configs: results };
        std::fs::write("/home/scamarena/ProjectCoin/run40_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
        return;
    }

    eprintln!();
    let baseline = results.iter().find(|r| r.is_baseline).unwrap();

    let mut sorted: Vec<&ConfigResult> = results.iter().collect();
    sorted.sort_by(|a,b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n=== RUN40 BTC Dominance Scalp Filter Results ===");
    println!("Baseline (no filter): PnL={:+.2}  WR={:.1}%  PF={:.3}", baseline.total_pnl, baseline.portfolio_wr, baseline.pf);
    println!("\n{:>3}  {:<18} {:>8} {:>7} {:>8} {:>7}",
        "#", "Config", "PnL", "ΔPnL", "WR%", "PF");
    println!("{}", "-".repeat(60));
    for (i,r) in sorted.iter().enumerate() {
        let delta = r.total_pnl - baseline.total_pnl;
        println!("{:>3}  {:<18} {:>+8.2} {:>+7.2} {:>6.1}% {:>7.3}",
            i+1, r.label, r.total_pnl, delta, r.portfolio_wr, r.pf);
        if i>=19 { println!("  ... ({} more)", sorted.len()-20); break; }
    }
    println!("{}", "=".repeat(60));

    let best = sorted.first().unwrap();
    let is_positive = best.total_pnl > baseline.total_pnl;
    println!("\nVERDICT: {} (best ΔPnL={:+.2})",
        if is_positive { "POSITIVE" } else { "NEGATIVE — no config beats baseline" },
        best.total_pnl - baseline.total_pnl);

    let notes = format!("RUN40 BTC dominance scalp filter. {} configs. Baseline PnL={:.2}. Best: {} (PnL={:.2}, Δ={:.2})",
        results.len(), baseline.total_pnl, best.label, best.total_pnl, best.total_pnl-baseline.total_pnl);
    let output = Output { notes, configs: results };
    std::fs::write("/home/scamarena/ProjectCoin/run40_1_results.json", &serde_json::to_string_pretty(&output).unwrap()).ok();
    eprintln!("\nSaved → run40_1_results.json");
}
