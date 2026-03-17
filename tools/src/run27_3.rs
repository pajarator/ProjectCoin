/// RUN27.3 — Comparison: COINCLAW v13 Primary Long vs Momentum Breakout
///
/// Side-by-side evaluation on the same OOS test period (67%→100%):
///   A. COINCLAW v13 per-coin primary long strategy (fixed 0.3% SL, signal exit)
///   B. Momentum breakout best config (grid on train 0%→67%, eval on OOS 67%→100%)
///   C. Combined: COINCLAW OR momentum fires → take the trade
///
/// Metrics: n_trades, WR%, PF, total P&L%
/// Fee: 0.1%/side, slip: 0.05%/side (same as prior RUNs).

use rayon::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::indicators::{atr, rsi, rolling_mean, sma};
use crate::loader::{load_ohlcv, COIN_STRATEGIES};
use crate::loader::Bar;
use crate::strategies::signals;
use crate::run27::{
    adx14, bar16_return, signals_long, sim, Params,
    MOVE_THRESHOLDS, VOL_MULTS, ADX_THRESHOLDS, ATR_MULTS, TRAIL_CONFIGS,
};

const FEE:  f64 = 0.001;
const SLIP: f64 = 0.0005;
const FIXED_SL: f64 = 0.003;
const SPLIT: f64 = 0.67;

// ── COINCLAW fixed-SL simulation ──────────────────────────────────────────────
fn sim_coinclaw(close: &[f64], entry: &[bool], exit: &[bool])
    -> (usize, f64, f64, f64, f64, f64)
{
    let n = close.len();
    let mut wins = 0usize;
    let mut gross_win = 0.0f64;
    let mut gross_loss = 0.0f64;
    let mut total_pnl = 0.0f64;
    let mut n_trades = 0usize;
    let mut in_pos = false;
    let mut ep = 0.0f64;

    for i in 0..n {
        if in_pos {
            let ret = (close[i] - ep) / ep;
            let closed = if ret <= -FIXED_SL {
                Some(-FIXED_SL * (1.0 + SLIP) * 100.0 - FEE * 200.0)
            } else if exit[i] {
                Some((close[i] * (1.0 - SLIP) - ep) / ep * 100.0 - FEE * 200.0)
            } else { None };
            if let Some(pnl) = closed {
                if pnl > 0.0 { wins += 1; gross_win += pnl; }
                else { gross_loss += -pnl; }
                total_pnl += pnl;
                n_trades += 1;
                in_pos = false;
            }
        } else if entry[i] {
            ep = close[i] * (1.0 + SLIP);
            in_pos = true;
        }
    }
    if in_pos {
        let pnl = (close[n-1] * (1.0-SLIP) - ep) / ep * 100.0 - FEE * 200.0;
        if pnl > 0.0 { wins += 1; gross_win += pnl; }
        else { gross_loss += -pnl; }
        total_pnl += pnl;
        n_trades += 1;
    }
    if n_trades == 0 { return (0, 0.0, 0.0, 0.0, 0.0, 0.0); }
    let wr = wins as f64 / n_trades as f64 * 100.0;
    let pf = if gross_loss > 0.0 { gross_win / gross_loss } else { gross_win + 1.0 };
    let avg_win  = if wins > 0 { gross_win / wins as f64 } else { 0.0 };
    let avg_loss = if n_trades > wins { gross_loss / (n_trades - wins) as f64 } else { 0.0 };
    (n_trades, wr, pf, total_pnl, avg_win, avg_loss)
}

// ── Per-coin comparison ────────────────────────────────────────────────────────
fn process_coin(coin: &str, strat_name: &str) -> Value {
    let bars = load_ohlcv(coin);
    let n = bars.len();
    let split = (n as f64 * SPLIT) as usize;

    let close: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let high:  Vec<f64> = bars.iter().map(|b| b.h).collect();
    let low:   Vec<f64> = bars.iter().map(|b| b.l).collect();
    let vol:   Vec<f64> = bars.iter().map(|b| b.v).collect();

    // Precompute full-series indicators (no look-ahead: each bar only uses past)
    let ret16  = bar16_return(&close);
    let vol_ma = rolling_mean(&vol, 20);
    let adx    = adx14(&high, &low, &close);
    let rsi14  = rsi(&close, 14);
    let sma20  = sma(&close, 20);
    let sma50  = sma(&close, 50);
    let atr14  = atr(&high, &low, &close, 14);

    // COINCLAW signals on FULL series (uses all history for indicator warmup)
    let (ce_full, cx_full) = signals(&bars, strat_name);

    // OOS slices
    let oc   = &close[split..];
    let oh   = &high[split..];
    let ol   = &low[split..];
    let ov   = &vol[split..];
    let or16 = &ret16[split..];
    let ovm  = &vol_ma[split..];
    let oadx = &adx[split..];
    let orsi = &rsi14[split..];
    let os20 = &sma20[split..];
    let os50 = &sma50[split..];
    let oatr = &atr14[split..];

    // ── A: COINCLAW OOS ──
    let ce_oos = &ce_full[split..];
    let cx_oos = &cx_full[split..];
    let (cnt, cwr, cpf, cpnl, caw, cal) = sim_coinclaw(oc, ce_oos, cx_oos);

    // ── B: Momentum breakout — grid on train → best on OOS ──
    let tc   = &close[..split];
    let th   = &high[..split];
    let tl   = &low[..split];
    let tv   = &vol[..split];
    let tr16 = &ret16[..split];
    let tvm  = &vol_ma[..split];
    let tadx = &adx[..split];
    let trsi = &rsi14[..split];
    let ts20 = &sma20[..split];
    let ts50 = &sma50[..split];
    let tatr = &atr14[..split];

    // Grid search (long only) on train
    let mut best_p = Params { move_thresh: 0.02, vol_mult: 2.0, adx_thresh: 20.0,
                               atr_mult: 0.75, trail_atr: 1.0, trail_act: 0.008 };
    let mut best_score = f64::NEG_INFINITY;

    for &move_thresh in &MOVE_THRESHOLDS {
        for &vol_mult in &VOL_MULTS {
            for &adx_thresh in &ADX_THRESHOLDS {
                for &atr_mult in &ATR_MULTS {
                    for &(trail_atr, trail_act) in &TRAIL_CONFIGS {
                        let p = Params { move_thresh, vol_mult, adx_thresh,
                                         atr_mult, trail_atr, trail_act };
                        let (e, x) = signals_long(tc,th,tl,tv,tr16,tvm,tadx,trsi,ts20,ts50,&p);
                        let (nt,_,_,_,_,_,sh) = sim(tc,tatr,&e,&x,atr_mult,trail_atr,trail_act,false);
                        if nt < 5 { continue; }
                        let score = sh * (nt as f64).sqrt();
                        if score > best_score { best_score = score; best_p = p; }
                    }
                }
            }
        }
    }

    let (me, mx) = signals_long(oc,oh,ol,ov,or16,ovm,oadx,orsi,os20,os50,&best_p);
    let (mnt, mwr, mpf, mpnl, maw, mal, _) =
        sim(oc, oatr, &me, &mx, best_p.atr_mult, best_p.trail_atr, best_p.trail_act, false);

    // ── C: Combined — fire if EITHER coinclaw OR momentum signals ──
    let m_oos = oc.len();
    let comb_entry: Vec<bool> = (0..m_oos).map(|i| ce_oos[i] || me[i]).collect();
    let comb_exit:  Vec<bool> = (0..m_oos).map(|i| cx_oos[i] || mx[i]).collect();
    // For combined, use COINCLAW fixed SL (conservative)
    let (xnt, xwr, xpf, xpnl, xaw, xal) = sim_coinclaw(oc, &comb_entry, &comb_exit);

    json!({
        "coin": coin,
        "strat": strat_name,
        "coinclaw": { "t": cnt, "wr": cwr, "pf": cpf, "pnl": cpnl,
                      "avg_win": caw, "avg_loss": cal },
        "momentum": { "t": mnt, "wr": mwr, "pf": mpf, "pnl": mpnl,
                      "avg_win": maw, "avg_loss": mal,
                      "config": { "move": best_p.move_thresh*100.0,
                                  "vol": best_p.vol_mult, "adx": best_p.adx_thresh,
                                  "atr": best_p.atr_mult, "trail": best_p.trail_atr,
                                  "act": best_p.trail_act*100.0 } },
        "combined": { "t": xnt, "wr": xwr, "pf": xpf, "pnl": xpnl,
                      "avg_win": xaw, "avg_loss": xal },
    })
}

// ── Entry point ────────────────────────────────────────────────────────────────
pub fn run(_shutdown: Arc<AtomicBool>) {
    println!("================================================================");
    println!("RUN27.3 — Comparison: COINCLAW v13 vs Momentum Breakout (LONG)");
    println!("================================================================");
    println!("OOS test period: bars[67%..100%]  Fee=0.1%/side  Slip=0.05%/side");
    println!("A: COINCLAW v13 primary long strategy (fixed 0.3% SL)");
    println!("B: Momentum breakout — best config from grid on train[0..67%]");
    println!("C: Combined — COINCLAW OR momentum entry (fixed 0.3% SL, COINCLAW exit)");
    println!();

    let coins: Vec<(&str, &str)> = COIN_STRATEGIES.iter()
        .map(|(c, s)| (*c, *s)).collect();

    let results: Vec<Value> = coins.par_iter()
        .map(|&(c, s)| process_coin(c, s))
        .collect();

    // ── Print table ───────────────────────────────────────────────────────────
    println!("  {:6}  | {:>5} {:>6} {:>6} {:>7} | {:>5} {:>6} {:>6} {:>7} | {:>5} {:>6} {:>6} {:>7}",
        "Coin",
        "C.t", "C.WR%", "C.PF", "C.PnL%",
        "M.t", "M.WR%", "M.PF", "M.PnL%",
        "X.t", "X.WR%", "X.PF", "X.PnL%");
    println!("  {}+{}+{}", "-".repeat(34), "-".repeat(34), "-".repeat(34));

    let focus = ["NEAR", "DASH", "XLM"];

    let mut tot_c_pnl = 0.0f64; let mut tot_m_pnl = 0.0f64; let mut tot_x_pnl = 0.0f64;
    let mut c_wr_sum = 0.0f64;  let mut m_wr_sum = 0.0f64;  let mut x_wr_sum = 0.0f64;
    let nc = results.len() as f64;

    for r in &results {
        let coin = r["coin"].as_str().unwrap_or("?");
        let c = &r["coinclaw"];
        let m = &r["momentum"];
        let x = &r["combined"];

        let cnt  = c["t"].as_u64().unwrap_or(0);
        let cwr  = c["wr"].as_f64().unwrap_or(0.0);
        let cpf  = c["pf"].as_f64().unwrap_or(0.0);
        let cpnl = c["pnl"].as_f64().unwrap_or(0.0);

        let mnt  = m["t"].as_u64().unwrap_or(0);
        let mwr  = m["wr"].as_f64().unwrap_or(0.0);
        let mpf  = m["pf"].as_f64().unwrap_or(0.0);
        let mpnl = m["pnl"].as_f64().unwrap_or(0.0);

        let xnt  = x["t"].as_u64().unwrap_or(0);
        let xwr  = x["wr"].as_f64().unwrap_or(0.0);
        let xpf  = x["pf"].as_f64().unwrap_or(0.0);
        let xpnl = x["pnl"].as_f64().unwrap_or(0.0);

        let mark = if focus.contains(&coin) { "*" } else { " " };

        println!("  {:6}{}| {:>5} {:>6.1} {:>6.2} {:>+7.1} | {:>5} {:>6.1} {:>6.2} {:>+7.1} | {:>5} {:>6.1} {:>6.2} {:>+7.1}",
            coin, mark,
            cnt, cwr, cpf, cpnl,
            mnt, mwr, mpf, mpnl,
            xnt, xwr, xpf, xpnl);

        tot_c_pnl += cpnl; c_wr_sum += cwr;
        tot_m_pnl += mpnl; m_wr_sum += mwr;
        tot_x_pnl += xpnl; x_wr_sum += xwr;
    }

    println!("  {}+{}+{}", "-".repeat(34), "-".repeat(34), "-".repeat(34));
    println!("  {:7}| {:>5} {:>6.1} {:>6} {:>+7.1} | {:>5} {:>6.1} {:>6} {:>+7.1} | {:>5} {:>6.1} {:>6} {:>+7.1}",
        "AVG",
        "", c_wr_sum / nc, "", tot_c_pnl / nc,
        "", m_wr_sum / nc, "", tot_m_pnl / nc,
        "", x_wr_sum / nc, "", tot_x_pnl / nc);

    println!();
    println!("  Portfolio totals (sum across 18 coins):");
    println!("    COINCLAW total P&L:  {:+.1}%", tot_c_pnl);
    println!("    Momentum total P&L:  {:+.1}%", tot_m_pnl);
    println!("    Combined total P&L:  {:+.1}%", tot_x_pnl);

    println!();
    println!("  Focus coins (NEAR / DASH / XLM):");
    for r in &results {
        let coin = r["coin"].as_str().unwrap_or("?");
        if !focus.contains(&coin) { continue; }
        let c = &r["coinclaw"];
        let m = &r["momentum"];
        let x = &r["combined"];
        println!("  {} COINCLAW: t={} WR={:.1}% PF={:.2} PnL={:+.1}%",
            coin, c["t"], c["wr"].as_f64().unwrap_or(0.0),
            c["pf"].as_f64().unwrap_or(0.0), c["pnl"].as_f64().unwrap_or(0.0));
        println!("  {} MOMENTUM: t={} WR={:.1}% PF={:.2} PnL={:+.1}%",
            coin, m["t"], m["wr"].as_f64().unwrap_or(0.0),
            m["pf"].as_f64().unwrap_or(0.0), m["pnl"].as_f64().unwrap_or(0.0));
        println!("  {} COMBINED: t={} WR={:.1}% PF={:.2} PnL={:+.1}%",
            coin, x["t"], x["wr"].as_f64().unwrap_or(0.0),
            x["pf"].as_f64().unwrap_or(0.0), x["pnl"].as_f64().unwrap_or(0.0));
        println!("  {}   Best momentum config: move={:.1}% vol={:.1}x ADX≥{:.0} ATR×{:.2} trail={:.2}x@{:.1}%",
            coin,
            m["config"]["move"].as_f64().unwrap_or(0.0),
            m["config"]["vol"].as_f64().unwrap_or(0.0),
            m["config"]["adx"].as_f64().unwrap_or(0.0),
            m["config"]["atr"].as_f64().unwrap_or(0.0),
            m["config"]["trail"].as_f64().unwrap_or(0.0),
            m["config"]["act"].as_f64().unwrap_or(0.0));
        println!();
    }

    // Save
    let out_path = "archive/RUN27/run27_3_results.json";
    let output = serde_json::json!({ "run": "RUN27.3", "coins": results });
    std::fs::write(out_path, serde_json::to_string_pretty(&output).unwrap()).unwrap();
    println!("  Results saved to {}", out_path);
}
