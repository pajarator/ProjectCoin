/// RUN39.2 — Walk-Forward Validation for Asymmetric Cooldown
///
/// 3-window walk-forward (train 2mo, test 1mo) across 18 coins.
/// For each window: train finds best cooldown params, test evaluates OOS.
///
/// Run: cargo run --release --features run39 -- --run39-2

use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ── Constants (shared with run39) ───────────────────────────────────────────
const INITIAL_BAL: f64 = 1000.0;
const SL: f64 = 0.003;
const MIN_HOLD_BARS: u32 = 2;

const N_COINS: usize = 18;
const COIN_NAMES: [&str; N_COINS] = [
    "DASH","UNI","NEAR","ADA","LTC","SHIB","LINK","ETH",
    "DOT","XRP","ATOM","SOL","DOGE","XLM","AVAX","ALGO","BNB","BTC",
];
const COIN_STRATS: [&str; N_COINS] = [
    "OuMeanRev", "BB_Bounce", "VWAPRev", "ADRRev", "BollingerBounce",
    "RSI_Rev", "BB_Bounce", "VWAPRev", "ADRRev", "VWAPRev",
    "RSI_Rev", "BB_Bounce", "BollingerBounce", "VWAPRev", "BB_Bounce",
    "RSI_Rev", "BollingerBounce", "VWAPRev",
];

#[derive(Clone, Copy, PartialEq, Debug)]
struct CooldownGridEntry {
    win_cd: u32,
    loss_cd: u32,
    consec_2_cd: u32,
    consec_3_cd: u32,
}
impl CooldownGridEntry {
    fn label(&self) -> String {
        format!("W{}_L{}_C2_{}_C3_{}", self.win_cd, self.loss_cd, self.consec_2_cd, self.consec_3_cd)
    }
}

fn build_grid() -> Vec<CooldownGridEntry> {
    let mut grid = Vec::new();
    grid.push(CooldownGridEntry { win_cd: 2, loss_cd: 2, consec_2_cd: 60, consec_3_cd: 60 });
    for win_cd in [1u32, 2, 3] {
        for loss_cd in [2u32, 3, 4, 5] {
            for c2 in [4u32, 6, 8, 12] {
                for c3 in [8u32, 12, 16, 20] {
                    grid.push(CooldownGridEntry { win_cd, loss_cd, consec_2_cd: c2, consec_3_cd: c3 });
                }
            }
        }
    }
    grid
}

// ── Data structures (abbreviated — same as run39) ──────────────────────────
struct CoinData {
    close: Vec<f64>, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>,
    volume: Vec<f64>, rsi: Vec<f64>, sma20: Vec<f64>,
    bb_upper: Vec<f64>, bb_lower: Vec<f64>, atr: Vec<f64>,
    adx: Vec<f64>, vwap: Vec<f64>, zscore: Vec<f64>, regime: Vec<i8>,
}

struct RegimePos { dir: i8, entry: f64, entry_bar: usize, notional: f64 }
struct CooldownState { cooldown: usize, last_was_win: Option<bool>, consec_loss_streak: u32 }

#[derive(Serialize)]
struct CoinWindowResult {
    coin: String,
    train_pnl: f64,
    train_wr: f64,
    test_pnl: f64,
    test_wr: f64,
    test_delta_pnl: f64,
    best_train_cfg: String,
    is_positive: bool,
}

#[derive(Serialize)]
struct WindowResult {
    window_idx: usize,
    train_start: String,
    train_end: String,
    test_start: String,
    test_end: String,
    n_train: usize,
    n_test: usize,
    coins: Vec<CoinWindowResult>,
    portfolio_train_pnl: f64,
    portfolio_test_pnl: f64,
    portfolio_baseline_test_pnl: f64,
    portfolio_delta: f64,
    pct_positive: f64,
}

#[derive(Serialize)]
struct Output { notes: String, windows: Vec<WindowResult> }

// ── Helpers (duplicated for self-contained module) ───────────────────────────
fn rmean(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    let mut sum = 0.0;
    for i in 0..n { sum += data[i]; if i >= w { sum -= data[i-w]; } if i+1 >= w { out[i] = sum/w as f64; } }
    out
}
fn rstd(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { let s = &data[i+1-w..=i]; let m = s.iter().sum::<f64>()/w as f64; let v = s.iter().map(|x|(x-m).powi(2)).sum::<f64>()/w as f64; out[i]=v.sqrt(); }
    out
}
fn rmin(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { out[i] = data[i+1-w..=i].iter().cloned().fold(f64::INFINITY, f64::min); }
    out
}
fn rmax(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    for i in (w-1)..n { out[i] = data[i+1-w..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max); }
    out
}
fn rsi_calc(c: &[f64], period: usize) -> Vec<f64> {
    let n = c.len();
    let mut out = vec![f64::NAN; n];
    if n < period+1 { return out; }
    let mut gains = vec![0.0; n]; let mut losses = vec![0.0; n];
    for i in 1..n { let d = c[i]-c[i-1]; if d>0.0 { gains[i]=d; } else { losses[i]=-d; } }
    let ag = rmean(&gains, period); let al = rmean(&losses, period);
    for i in 0..n { if !ag[i].is_nan() && !al[i].is_nan() { out[i] = if al[i]==0.0 {100.0} else {100.0-100.0/(1.0+ag[i]/al[i])}; } }
    out
}
fn rmean_nan(data: &[f64], w: usize) -> Vec<f64> {
    let n = data.len(); let mut out = vec![f64::NAN; n]; let mut sum=0.0; let mut count=0usize;
    for i in 0..n { if !data[i].is_nan() { sum+=data[i]; count+=1; } if i>=w && !data[i-w].is_nan() { sum-=data[i-w]; count-=1; } if i+1>=w && count>0 { out[i]=sum/count as f64; } }
    out
}
fn compute_atr(h: &[f64], l: &[f64], c: &[f64]) -> Vec<f64> {
    let n = h.len(); let mut tr = vec![f64::NAN; n];
    for i in 1..n { let hl=(h[i]-l[i]).abs(); let hc=(h[i]-c[i-1]).abs(); let lc=(l[i]-c[i-1]).abs(); tr[i]=hl.max(hc).max(lc); }
    rmean_nan(&tr, 14)
}
fn compute_adx(h: &[f64], l: &[f64], c: &[f64]) -> Vec<f64> {
    let n = h.len(); let mut pdm=vec![0.0; n]; let mut mdm=vec![0.0; n];
    for i in 1..n { let hh=h[i]-h[i-1]; let ll=l[i-1]-l[i]; if hh>ll&&hh>0.0{pdm[i]=hh;} if ll>hh&&ll>0.0{mdm[i]=ll;} }
    let atr=compute_atr(h,l,c); let pa=rmean_nan(&pdm,14); let ma=rmean_nan(&mdm,14); let mut dx=vec![f64::NAN;n];
    for i in 14..n { if atr[i]>0.0&&!pa[i].is_nan()&&!ma[i].is_nan(){ let pdi=100.0*pa[i]/atr[i]; let mdi=100.0*ma[i]/atr[i]; let ds=pdi+mdi; if ds>0.0{dx[i]=100.0*(pdi-mdi).abs()/ds;} } }
    rmean_nan(&dx,14)
}
fn compute_vwap(h: &[f64], l: &[f64], c: &[f64], v: &[f64]) -> Vec<f64> {
    let n=c.len(); let mut out=vec![f64::NAN;n]; let mut ct=0.0; let mut cv=0.0;
    for i in 0..n { let tpv=(h[i]+l[i]+c[i])/3.0; ct+=tpv*v[i]; cv+=v[i]; if cv>0.0{out[i]=ct/cv;} }
    out
}
fn regime_dir(adx: f64, close: f64, sma20: f64) -> i8 {
    if adx.is_nan() {0} else {if adx>=25.0{if close>sma20{1}else{-1}}else{0}}
}

fn load_15m(coin: &str) -> Option<(Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>)> {
    let path = format!("/home/scamarena/ProjectCoin/data_cache/{}_USDT_15m_5months.csv", coin);
    let data = match std::fs::read_to_string(&path) { Ok(d)=>d, Err(_)=>return None, };
    let mut o=Vec::new();let mut h=Vec::new();let mut l=Vec::new();let mut c=Vec::new();let mut v=Vec::new();
    for line in data.lines().skip(1) { let mut it=line.splitn(7,','); let _ts=it.next(); let oo: f64=it.next()?.parse().ok()?; let hh: f64=it.next()?.parse().ok()?; let ll: f64=it.next()?.parse().ok()?; let cc: f64=it.next()?.parse().ok()?; let vv: f64=it.next()?.parse().ok()?; if oo.is_nan()||hh.is_nan()||ll.is_nan()||cc.is_nan()||vv.is_nan(){continue;} o.push(oo);h.push(hh);l.push(ll);c.push(cc);v.push(vv); }
    if c.len()<100 {return None;} Some((o,h,l,c,v))
}

fn compute_data(o: Vec<f64>, h: Vec<f64>, l: Vec<f64>, c: Vec<f64>, v: Vec<f64>) -> CoinData {
    let n=c.len(); let rsi=rsi_calc(&c,14); let sma20=rmean(&c,20);
    let bb_std=rstd(&c,20); let mut bb_u=vec![f64::NAN;n]; let mut bb_l=vec![f64::NAN;n];
    for i in 0..n { if !sma20[i].is_nan()&&!bb_std[i].is_nan(){bb_u[i]=sma20[i]+2.0*bb_std[i];bb_l[i]=sma20[i]-2.0*bb_std[i];} }
    let atr=compute_atr(&h,&l,&c); let adx=compute_adx(&h,&l,&c); let vwap=compute_vwap(&h,&l,&c,&v);
    let cm=rmean(&c,20); let cs=rstd(&c,20); let mut zscore=vec![f64::NAN;n];
    for i in 0..n { if !cm[i].is_nan()&&cs[i]>0.0{zscore[i]=(c[i]-cm[i])/cs[i];} }
    let mut regime=vec![0i8;n];
    for i in 20..n { if !sma20[i].is_nan(){regime[i]=regime_dir(adx[i],c[i],sma20[i]);} }
    CoinData{close:c,open:o,high:h,low:l,volume:v,rsi,sma20,bb_upper:bb_u,bb_lower:bb_l,atr,adx,vwap,zscore,regime}
}

fn long_entry(d: &CoinData, strat: &str, i: usize) -> i8 {
    if i<25 {return 0;} let c=d.close[i]; let rsi=d.rsi[i]; let bbu=d.bb_upper[i]; let bbl=d.bb_lower[i]; let z=d.zscore[i]; let vwap=d.vwap[i];
    match strat {
        "VWAPRev"=>{if !vwap.is_nan()&&!rsi.is_nan(){let d=(c-vwap)/vwap*100.0; if d< -0.5&&rsi<40.0{return 1;}if d>0.5&&rsi>60.0{return -1;}}}
        "BB_Bounce"|"BollingerBounce"=>{if !bbl.is_nan()&&!bbu.is_nan()&&!rsi.is_nan(){if c<=bbl&&rsi<30.0{return 1;}if c>=bbu&&rsi>70.0{return -1;}}}
        "ADRRev"=>{let hh=rmax(&d.close,14);let ll=rmin(&d.close,14);if !hh[i].is_nan()&&!ll[i].is_nan()&&(hh[i]-ll[i])>0.0{let wr=-100.0*(hh[i]-c)/(hh[i]-ll[i]);if !rsi.is_nan(){if rsi<25.0&&wr< -80.0{return 1;}if rsi>75.0&&wr> -20.0{return -1;}}}}
        "RSI_Rev"=>{if !rsi.is_nan(){if rsi<30.0{return 1;}if rsi>70.0{return -1;}}}
        "OuMeanRev"=>{if !z.is_nan()&&!d.adx[i].is_nan(){let reg=d.regime[i];if z< -2.0&&reg==1{return 1;}if z>2.0&&reg==-1{return -1;}}}
        _=>{}
    }
    0
}

fn simulate_range(d: &CoinData, strat: &str, cfg: &CooldownGridEntry, start: usize, end: usize) -> (f64, usize, usize, f64) {
    // Returns (pnl, wins, losses, wr)
    let mut bal=INITIAL_BAL; let mut peak=INITIAL_BAL;
    let mut pos: Option<RegimePos>=None;
    let mut cd=CooldownState{cooldown:0,last_was_win:None,consec_loss_streak:0};
    let mut wins=0usize; let mut losses=0usize;
    for i in (start+1)..end.min(d.close.len()) {
        if bal>peak{peak=bal;}
        if let Some(ref p)=pos {
            let pct=if p.dir==1{(d.close[i]-p.entry)/p.entry}else{(p.entry-d.close[i])/p.entry};
            let bars_held=i-p.entry_bar;
            let hit_sl=pct<=-SL; let min_done=bars_held as u32>=MIN_HOLD_BARS;
            if hit_sl||min_done {
                let net=p.notional*pct; bal+=net;
                if net>1e-8{wins+=1;}else if net< -1e-8{losses+=1;}
                if net<=-SL{cd.consec_loss_streak+=1;}else{cd.consec_loss_streak=0;}
                cd.last_was_win=Some(net>1e-8);
                cd.cooldown=match cd.consec_loss_streak{0=>if net<=-SL{cfg.loss_cd as usize}else{cfg.win_cd as usize},1=>cfg.loss_cd as usize,2=>cfg.consec_2_cd as usize,_=>cfg.consec_3_cd as usize};
                pos=None;
            }
        } else if cd.cooldown>0 { cd.cooldown-=1; }
        else {
            let dir=long_entry(d,strat,i);
            if dir!=0 {
                let ok=if dir==1{d.regime[i]==1||d.regime[i]==0}else{d.regime[i]==-1||d.regime[i]==0};
                if ok&&i+1<end.min(d.close.len()) {
                    pos=Some(RegimePos{dir,entry:d.open[i+1],entry_bar:i+1,notional:bal*0.02});
                }
            }
        }
    }
    if let Some(ref p)=pos { let pct=if p.dir==1{(d.close[end.min(d.close.len())-1]-p.entry)/p.entry}else{(p.entry-d.close[end.min(d.close.len())-1])/p.entry}; bal+=p.notional*pct; }
    let trades=wins+losses; let wr=if trades>0{wins as f64/trades as f64*100.0}else{0.0};
    (bal-INITIAL_BAL, wins, losses, wr)
}

// ── Entry point ─────────────────────────────────────────────────────────────
pub fn run(shutdown: Arc<AtomicBool>) {
    eprintln!("RUN39.2 — Walk-Forward Validation: 3 windows (train 2mo, test 1mo)");
    eprintln!();

    // Load data
    eprintln!("Loading data...");
    let mut raw: Vec<Option<_>> = (0..N_COINS).map(|ci| {
        load_15m(COIN_NAMES[ci])
    }).collect();
    let mut all_ok = raw.iter().all(|r| r.is_some());
    if !all_ok { eprintln!("Missing data!"); return; }

    let coin_data: Vec<_> = raw.into_iter().map(|r| {
        let (o,h,l,c,v)=r.unwrap(); compute_data(o,h,l,c,v)
    }).collect();

    let bars_per_month = 30 * 24 * 4; // 15m bars: 4/hour * 24h * 30d
    let train_bars = 2 * bars_per_month; // 2 months train
    let test_bars = 1 * bars_per_month;  // 1 month test
    let total = coin_data[0].close.len();
    let n_windows = 3;

    let grid = build_grid();
    let baseline_cfg = CooldownGridEntry { win_cd:2, loss_cd:2, consec_2_cd:60, consec_3_cd:60 };

    let mut window_results = Vec::new();
    for wi in 0..n_windows {
        if shutdown.load(Ordering::SeqCst) { return; }
        let test_start = wi * test_bars;
        let test_end = (test_start + test_bars).min(total);
        let train_end = test_start;
        let train_start = (train_end as isize - train_bars as isize).max(0) as usize;

        eprintln!("\n=== Window {}: train [{}-{}] test [{}-{}] ===",
            wi, train_start, train_end, test_start, test_end);

        let coin_results: Vec<CoinWindowResult> = coin_data.iter().zip(COIN_STRATS.iter())
            .map(|(cd, strat)| {
                // Train: find best config
                let mut best_cfg = &baseline_cfg;
                let mut best_pnl = f64::NEG_INFINITY;
                for cfg in &grid {
                    let (pnl,_,_,_) = simulate_range(cd, strat, cfg, train_start, train_end);
                    if pnl > best_pnl { best_pnl = pnl; best_cfg = cfg; }
                }
                // Baseline on train
                let (base_train_pnl, base_train_wins, base_train_losses, base_train_wr) =
                    simulate_range(cd, strat, &baseline_cfg, train_start, train_end);
                let base_train_trades = base_train_wins + base_train_losses;
                let base_wr = if base_train_trades>0 {base_train_wins as f64/base_train_trades as f64*100.0}else{0.0};

                // Test: best config
                let (test_pnl, test_wins, test_losses, test_wr) =
                    simulate_range(cd, strat, best_cfg, test_start, test_end);
                // Test: baseline
                let (base_test_pnl, _, _, _) =
                    simulate_range(cd, strat, &baseline_cfg, test_start, test_end);

                CoinWindowResult {
                    coin: COIN_NAMES[wi].to_string(),
                    train_pnl: base_train_pnl,
                    train_wr: base_wr,
                    test_pnl,
                    test_wr,
                    test_delta_pnl: test_pnl - base_test_pnl,
                    best_train_cfg: best_cfg.label(),
                    is_positive: test_pnl > base_test_pnl,
                }
            })
            .collect();

        let portfolio_test_pnl: f64 = coin_results.iter().map(|c| c.test_pnl).sum();
        let portfolio_baseline: f64 = {
            coin_data.iter().zip(COIN_STRATS.iter())
                .map(|(cd, strat)| {
                    let (pnl,_,_,_) = simulate_range(cd, strat, &baseline_cfg, test_start, test_end);
                    pnl
                }).sum()
        };
        let portfolio_delta = portfolio_test_pnl - portfolio_baseline;
        let n_positive = coin_results.iter().filter(|c| c.is_positive).count();
        let pct_positive = n_positive as f64 / N_COINS as f64 * 100.0;

        eprintln!("  Portfolio test PnL: {:+.1} (baseline: {:+.1}, Δ={:+.1})", portfolio_test_pnl, portfolio_baseline, portfolio_delta);
        eprintln!("  Positive coins: {}/{} ({:.0}%)", n_positive, N_COINS, pct_positive);

        window_results.push(WindowResult {
            window_idx: wi,
            train_start: format!("bar_{}", train_start),
            train_end: format!("bar_{}", train_end),
            test_start: format!("bar_{}", test_start),
            test_end: format!("bar_{}", test_end),
            n_train: train_end - train_start,
            n_test: test_end - test_start,
            coins: coin_results,
            portfolio_train_pnl: 0.0, // simplified
            portfolio_test_pnl,
            portfolio_baseline_test_pnl: portfolio_baseline,
            portfolio_delta,
            pct_positive,
        });
    }

    // Overall summary
    let avg_delta: f64 = window_results.iter().map(|w| w.portfolio_delta).sum::<f64>() / n_windows as f64;
    let avg_positive_pct: f64 = window_results.iter().map(|w| w.pct_positive).sum::<f64>() / n_windows as f64;
    let all_positive = window_results.iter().all(|w| w.portfolio_delta > 0.0);

    println!("\n=== RUN39.2 Walk-Forward Summary ===");
    for w in &window_results {
        println!("  Window {}: ΔPnL={:+.1}, positive={:.0}%", w.window_idx, w.portfolio_delta, w.pct_positive);
    }
    println!("\nAvg ΔPnL: {:+.1}", avg_delta);
    println!("Avg positive coins: {:.0}%", avg_positive_pct);
    println!("VERDICT: {}",
        if all_positive && avg_positive_pct >= 55.0 {
            "POSITIVE — walk-forward confirms grid search"
        } else if avg_delta > 0.0 && avg_positive_pct >= 50.0 {
            "CONDITIONALLY POSITIVE — mixed OOS results"
        } else {
            "NEGATIVE — OOS results do not confirm grid search"
        });

    let notes = format!("RUN39.2 walk-forward: 3 windows train=2mo test=1mo. Avg ΔPnL={:.1}, Avg positive={:.0}%, All positive={}", avg_delta, avg_positive_pct, all_positive);
    let output = Output { notes, windows: window_results };
    let json = serde_json::to_string_pretty(&output).unwrap();
    std::fs::write("/home/scamarena/ProjectCoin/run39_2_results.json", &json).ok();
    eprintln!("\nSaved → run39_2_results.json");
}
