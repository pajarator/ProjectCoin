mod backtest;
mod indicators;
mod loader;
mod mc;
mod run15a;
mod run15b;
mod run15c;
mod run15d;
mod run15e;
mod run19;
mod run20;
mod run21;
mod run22;
mod run23;
mod run24;
mod run25;
mod run26;
mod run27;
mod run27_2;
mod run27_3;
mod run28;
mod run29;
mod run17_2;
mod run17_3;
mod strategies;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");

    // SIGINT handler — child modules check this flag
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let s = shutdown.clone();
    ctrlc::set_handler(move || {
        s.store(true, std::sync::atomic::Ordering::SeqCst);
        eprintln!("\nShutdown requested...");
    })
    .ok();

    match cmd {
        "run15a" => run15a::run(shutdown),
        "run15b" => run15b::run(shutdown),
        "run15c" => run15c::run(shutdown),
        "run15d" => run15d::run(shutdown),
        "run15e" => run15e::run(shutdown),
        "run19"  => run19::run(shutdown),
        "run20"  => run20::run(shutdown),
        "run21"  => run21::run(shutdown),
        "run22"  => run22::run(shutdown),
        "run23"  => run23::run(shutdown),
        "run24"  => run24::run(shutdown),
        "run25"  => run25::run(shutdown),
        "run26"  => run26::run(shutdown),
        "run27"  => run27::run(shutdown),
        "run27.2" => run27_2::run(shutdown),
        "run27.3" => run27_3::run(shutdown),
        "run28"   => run28::run(shutdown),
        "run29"   => run29::run(shutdown),
        "run17.2" => run17_2::run(shutdown),
        "run17.3" => run17_3::run(shutdown),
        _ => {
            eprintln!("ProjectCoin Research Tools — Rust + Rayon");
            eprintln!();
            eprintln!("Usage: tools <command>");
            eprintln!();
            eprintln!("  run15a    Bayesian entry gate vs COINCLAW primary strategies (train/test OOS)");
            eprintln!("  run15b    Scalp NN filter — proper TP/SL sim, fees, full 1-year 1m data");
            eprintln!("  run17.2   COINCLAW primary strategies — 100k MC, all 18 coins (parallel)");
            eprintln!("  run17.3   Portfolio-level MC simulation — block bootstrap equity curves");
        }
    }
}
