Process all TEST/RUN* folders sequentially. For each folder:

1. Read TEST/RUN{N}/RUN{N}_suggestion.md to understand the hypothesis
2. Implement run{N}.rs in coinclaw/src/ following the RUN38 pattern (copy run38.rs as template and adapt for the new hypothesis)
3. Add run{N} feature to coinclaw/Cargo.toml and an entry point in coinclaw/src/main.rs
4. Build and run the grid search with: cargo run --release --features run{N} -- --run{N}
5. Interpret results — if POSITIVE (P&L delta > 0 and WR improvement) proceed to walk-forward validation (RUN38.2 style). If NEGATIVE, skip walk-forward.
6. Write archive/RUN{N}/recommendation.md documenting: hypothesis, method, grid params tested, results table, per-coin breakdown, and recommendation (APPLY or NEGATIVE)
7. Copy run{N}_1_results.json (and _2_results.json if walk-forward ran) to archive/RUN{N}/
8. Move all TEST/RUN{N}/ files to archive/RUN{N}/ including the original suggestion. Make sure folder is empty and removed.

After all folders are processed, output <promise>ALL RUNS COMPLETE</promise>

Key constraints:
- Use Rust+Rayon for all heavy computation (per memory feedback)
- Include SIGINT checkpoint/resume handling in each run script
- 18 coins: DASH, UNI, NEAR, ADA, LTC, SHIB, LINK, ETH, DOT, XRP, ATOM, SOL, DOGE, XLM, AVAX, ALGO, BNB, BTC
- Data files: data_cache/{COIN}_USDT_15m_5months.csv
- Always save results.json even on interrupt
