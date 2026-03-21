# COINCLAW Strategies Tested — Complete Archive

Generated from archive/RUN*_suggestion.md across all 140 RUNs.

## Quick Summary

| Category | RUNs | Count |
|---|---|---|
| Exit Strategy / Stop Loss | 7, 8, 26, 46, 61, 62, 73, 77, 88, 90, 96, 98, 103, 104 | 14 |
| Entry Filter / Confirmation | 50, 55, 59, 60, 61, 63, 65, 70, 71, 85, 89, 95, 105, 107, 109, 110, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140 | 43 |
| Position Sizing / Risk | 19, 42, 51, 52, 67, 68, 74, 75, 79, 93, 100, 101 | 12 |
| Cooldown / Re-Entry | 38, 39, 45, 48, 59, 72, 80, 83, 94, 106 | 10 |
| Portfolio / Multi-Coin | 49, 63, 64, 72, 78, 81, 86, 87, 100 | 9 |
| Scalp Strategy | 10, 12, 29, 35, 40, 45, 66, 67, 72, 95, 97, 106, 108 | 13 |
| Strategy Discovery | 1, 2, 3, 4, 5, 6, 11, 13, 14, 22, 23 | 11 |
| Indicator Confirmation | 50, 58, 60, 65, 71, 80, 85, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140 | 35 |
| ML / GA | 14, 15, 16, 22, 23, 24, 25 | 7 |
| Regime / Breadth Filter | 12, 20, 21, 43, 54, 55, 63, 82, 87, 89 | 10 |
| Take Profit / Partial Exit | 8, 46, 53, 69, 84, 90, 101, 102 | 8 |
| Time / Session Filters | 41, 57, 84, 91, 106 | 5 |
| Momentum / Breakout | 27, 28, 40, 85, 108, 122, 127, 128, 139 | 9 |
| Correlation / BTC Filters | 40, 49, 55, 63, 68, 86, 100 | 7 |
| Volume-Based | 50, 58, 64, 71, 80, 97, 104, 109, 110, 125 | 10 |
| Indicator Library / Bugs | 14, 36, 37 | 3 |
| Validation Only | 17 | 1 |
| Uncategorized | 30-34 (not found) | 0 |

---

## Full RUN List

### RUN1 — Strategy discovery (best strategies per coin)
POSITIVE — Mean Reversion achieves 72-87% WR across coins on 15m timeframe

### RUN2 — Parameter tuning
### RUN3 — Mean reversion deep-dive
### RUN4 — Long directional trading optimization
### RUN5 — Short (market-dump) strategies
### RUN6 — ISO short (coin-specific overbought) strategies

### RUN7 — Stop loss optimization
POSITIVE — SL 0.5% to 0.3% improves PF; applied to COINCLAW

### RUN8 — Take profit optimization
NEGATIVE — TP does not help regime trades; no TP added

### RUN10 — Scalp indicator discovery, F6 filter validation, fee-aware TP/SL
POSITIVE — Scalp overlay (TP=0.80%, SL=0.10%) added

### RUN11 — STRAT1000 strategy discovery
POSITIVE — DASH switched to OuMeanRev strategy

### RUN12 — Scalp market mode filter
POSITIVE — Scalp direction must match regime direction

### RUN13 — STRAT1000 final sweep: complementary signals
POSITIVE — Laguerre RSI, Kalman Filter, KST Cross as secondary entries (+14.2% P&L)

### RUN14 — Indicator library expansion (+18 indicators)
Bug fixes applied (doji NaN, CMF propagation, flat RSI)

### RUN15 — Feature engineering pipeline (66-feature ML matrix)
Bug fixes applied; ML infrastructure created

### RUN16 — ML feature importance + walk-forward
NEGATIVE — 54.6% IS accuracy doesn't survive OOS; no COINCLAW changes

### RUN17 — Monte Carlo validation of COINCLAW v13
POSITIVE — 18/18 coins robust; p50 return +15,045%; no changes needed (validation only)

### RUN19 — Position sizing comparison
NEGATIVE — Kelly amplifies losses 15-17x; Fixed 2% remains default

### RUN20 — Momentum crowding filter
NEGATIVE — Filters are counterproductive; no COINCLAW changes

### RUN21 — Sentiment regime filter (BTC RSI as Fear/Greed)
NEGATIVE — Fear regime reduces WR to 30.4%; no regime clears breakeven

### RUN22 — Genetic algorithm v2 strategy discovery
NEGATIVE — 0/18 coins achieve >44% WR OOS; GA overfits train regime

### RUN23 — Differential evolution parameter optimization
NEGATIVE — 0/54 pairs achieve >44% WR OOS; DE cannot fix regime mismatch

### RUN24 — Ensemble strategy framework (voting/intersection)
NEGATIVE — 1/90 combinations clears 44% WR; strategies too correlated

### RUN25 — ML regime detection (BTC 4-regime framework)
NEGATIVE — 1/72 combinations clears 44% WR; circular ML bug dropped

### RUN26 — ATR-based dynamic stops + trailing stops
NEGATIVE — WR rises +3pp but PF drops; trailing stops not suited to mean-reversion

### RUN27 — Breakout momentum rider
CONDITIONALLY POSITIVE — NEAR/XLM/XRP confirmed as persistence coins; must run as independent layer

### RUN28 — Momentum persistence classifier
— — NEAR (+2.4%) and XLM (+5.3%) confirmed; ATOM/SHIB/LTC/ADA/DOT/BNB hard exclusions

### RUN29 — MAX_SCALP_OPENS_PER_CYCLE optimization
NEGATIVE — Cap irrelevant; scalp WR=10.9% vs breakeven 27.8%; live outperformance was artifact

### RUN34 — ISO short drawdown mitigation
NEGATIVE — ISO shorts are primary profit engine (+$118 of $144); mitigations hurt P&L

### RUN35 — Scalp Exit Strategy Grid Search
POSITIVE — stoch_50 exit improves PnL +$12.54 vs baseline; applied to COINCLAW

### RUN36 — ADX zero-value bug fix
Bug corrected; ADX was zero-valued when high=low

### RUN37 — Regime filter logic bug
Bug corrected in regime filter logic

### RUN38 — Volume-Volatility Event Proxy: Targeted Mean Reversion in High-Energy Windows
UNEXECUTED

### RUN39 — Asymmetric Win/Loss Cooldown: Consecutive-Loss Escalation Beyond ISO
UNEXECUTED

### RUN40 — BTC Dominance Scalp Filter: Cross-Coin Relative Strength Gate
UNEXECUTED

### RUN41 — Session-Based Trade Filter: Asia/Europe/US Session Conditional Engagement
UNEXECUTED

### RUN42 — Dynamic Leverage by Volatility Regime: Risk-Adjusted Position Sizing
UNEXECUTED

### RUN43 — Breadth Momentum (Velocity) Filter: Anticipating Regime Transitions
UNEXECUTED

### RUN44 — Multi-Timeframe ISO Short Confirmation: 15m/1h/4h Overbought Alignment
UNEXECUTED

### RUN45 — Complement-Scalp Mutual Exclusion + Exhaustion Timer
UNEXECUTED

### RUN46 — Partial Reversion Signal Exit: Z-Score Deviation-Adaptive Exit
UNEXECUTED

### RUN47 — Per-Strategy Optimal MIN_HOLD: Strategy-Specific Hold Times
UNEXECUTED

### RUN48 — Z-Score Recovery Suppression: Anti-Chase Re-Entry Gate
UNEXECUTED

### RUN49 — Cross-Coin Correlation Filter: Avoiding Clustered Over-Concentration
UNEXECUTED

### RUN50 — Candle Composition Filter: Volume Profile Imbalance as Entry Quality Gate
UNEXECUTED

### RUN51 — Drawdown-Contingent Stop Loss Widening: Dynamic SL Based on Cumulative P&L
UNEXECUTED

### RUN52 — Z-Score Deviation Position Sizing: Signal Confidence-Weighted Entries
UNEXECUTED

### RUN53 — Partial Exit / Scale-Out on Regime Trades: Progressive Profit-Taking
UNEXECUTED

### RUN54 — Volatility Regime Entry Filter: Trade Only When Volatility Is Below Median
UNEXECUTED

### RUN55 — BTC-Altcoin Breadth Divergence Filter for ISO Shorts
UNEXECUTED

### RUN56 — SMA Cross-Back Depth Filter: Exit Quality Gate for Signal Exits
UNEXECUTED

### RUN57 — Day-of-Week Trade Filter: Suppressing Low-Edge Days
UNEXECUTED

### RUN58 — Post-Event Gap Fill Strategy: Exploiting CME Gap Dynamics in Crypto
UNEXECUTED

### RUN59 — Same-Direction Consecutive Signal Suppression: Anti-Doubling Down Filter
UNEXECUTED

### RUN60 — Z-Score Momentum Filter: Confirming Direction of Indicator Deterioration
UNEXECUTED

### RUN61 — RSI Divergence Confirmation for LONG Entries
UNEXECUTED

### RUN62 — Regime Breakeven Stop: Locking In Gains Without Exiting
UNEXECUTED

### RUN63 — BTC Trend Confirmation for Regime LONG Entries
UNEXECUTED

### RUN64 — Portfolio Position Density Filter: Risk Managing Market crowdedness
UNEXECUTED

### RUN65 — BB Squeeze Duration Filter: Entry Quality Based on Compression Time
UNEXECUTED

### RUN66 — Exit Reason Priority Reordering: Optimizing Which Signal Fires First
UNEXECUTED

### RUN67 — Scalp Entry Z-Score Threshold Tightening: More Selective Scalp Entries
UNEXECUTED

### RUN68 — Cross-Asset Correlation at Entry: Position Sizing Based on BTC Correlation
UNEXECUTED

### RUN69 — Winning Streak Profit-Taking: Exit Early After Extended Winning Runs
UNEXECUTED

### RUN70 — Z-Score Convergence Filter: Market-Wide Entry Confirmation
UNEXECUTED

### RUN71 — BB Width Percentile Rank Filter: Historically Compressed Bands Only
UNEXECUTED

### RUN72 — Scalp Mode: Disabling Scalp During Sustained Low-Volatility Markets
UNEXECUTED

### RUN73 — Dynamic Max Hold Based on Entry Z-Score
UNEXECUTED

### RUN74 — Daily Equity Compounding with Per-Coin Reset
UNEXECUTED

### RUN75 — Sharpe-Weighted Capital Allocation
UNEXECUTED

### RUN76 — Volatility-Adaptive Stop Loss
UNEXECUTED

### RUN77 — Z-Score Recovery Rate Exit
UNEXECUTED

### RUN78 — Cross-Coin Z-Score Confirmation
UNEXECUTED

### RUN79 — Breadth-Adaptive Position Sizing
UNEXECUTED

### RUN80 — Volume Imbalance Confirmation: On-Balance Volume Direction as Entry Filter
UNEXECUTED

### RUN81 — Equity Curve Circuit Breaker: Halt New Entries During Sustained Drawdown Periods
UNEXECUTED

### RUN82 — Regime Decay Detection: Early Exit When Ranging Transitions to Trending
UNEXECUTED

### RUN83 — Cooldown by Market Mode: Adaptive Cooldown Periods Based on Current Regime
UNEXECUTED

### RUN84 — Session-Based Partial Exit Scaling: Time-of-Day Dependent Take-Profit Levels
UNEXECUTED

### RUN85 — Momentum Pulse Filter: Short-Term Momentum Alignment for Regime Entries
UNEXECUTED

### RUN86 — Coin Correlation Clustering: Suppress Correlated Coin Entries to Reduce Concentration Risk
UNEXECUTED

### RUN87 — Drawdown Recovery Mode: Shift Market Mode Bias During Portfolio Drawdown
UNEXECUTED

### RUN88 — Trailing Z-Score Exit: Exit When Mean Reversion Has Recovered a Target Fraction
UNEXECUTED

### RUN89 — Market-Wide ADX Confirmation: Suppress Regime Entries During High-Trend Environments
UNEXECUTED

### RUN90 — Symmetry Exit: Risk-Reward Ratio Scaled by Entry Z-Score Magnitude
UNEXECUTED

### RUN91 — Hourly Z-Threshold Scaling: Time-of-Day Adaptive Entry Thresholds
UNEXECUTED

### RUN92 — Exit Reason Weighted Learning: Dynamic Signal Weighting Based on Historical Exit Performance
UNEXECUTED

### RUN93 — Consecutive Wins Streak Boost: Increase Position Size After Winning Streaks
UNEXECUTED

### RUN94 — Partial Reentry After Cooldown: Re-Enter at Improved Price After Early Signal Reappearance
UNEXECUTED

### RUN95 — Scalp Momentum Alignment: Require 1m Scalp Entries to Align With 15m Regime Direction
UNEXECUTED

### RUN96 — Z-Confluence Exit: Exit When Multiple Coins' Z-Scores Simultaneously Converge Toward Mean
UNEXECUTED

### RUN97 — BB Width Scalp Gate: Suppress Scalp Entries When Bollinger Band Width Indicates Low Volatility
UNEXECUTED

### RUN98 — Intraday Max Drawdown Clip: Exit Positions When Unrealized Loss Exceeds Daily Threshold
UNEXECUTED

### RUN99 — Z-Score Momentum Divergence: Exit When Z-Score and Price Momentum Diverge
UNEXECUTED

### RUN100 — Portfolio Correlation Risk Limit: Reduce Deployed Capital When Cross-Coin Correlation Spikes
UNEXECUTED

### RUN101 — Partial Position Split: Split Each Position Into Core and Satellite Halves With Different Exits
UNEXECUTED

### RUN102 — TWAP Entry Execution: Accumulate Positions Over Time to Reduce Entry Timing Risk
UNEXECUTED

### RUN103 — Stochastic Extreme Exit: Exit When Stochastic Reaches Extreme Levels During Profitable Trades
UNEXECUTED

### RUN104 — Volume Dry-Up Exit: Exit When Volume Collapses During Profitable Trades
UNEXECUTED

### RUN105 — Z-Score Persistence: Require Sustained Extreme Z-Score for N Consecutive Bars Before Entry
UNEXECUTED

### RUN106 — Hourly Scalp Cooldown: Scalp Cooldown Periods Scaled by UTC Session
UNEXECUTED

### RUN107 — Percentile Rank Z-Filter: Require Z-Score to Be at Historical Percentile Extreme
UNEXECUTED

### RUN108 — Momentum Hour Filter: Suppress Regime Entries When 1m Momentum Strongly Opposes Trade Direction
UNEXECUTED

### RUN109 — Minimum Volume Surge Confirmation: Require Volume Spike at Entry, Not Just Above-Average
UNEXECUTED

### RUN110 — BB Width Compression Entry: Enter When Bollinger Band Width Has Been Compressed
UNEXECUTED

### RUN111 — MACD Histogram Slope Exit: Exit When MACD Histogram Confirms Mean Reversion Is Complete
UNEXECUTED

### RUN112 — Money Flow Index Confirmation: Require MFI Extreme at Entry for Institutional Conviction
UNEXECUTED

### RUN113 — CCI Regime Confirmation: Require Commodity Channel Index Extreme at Entry
UNEXECUTED

### RUN114 — Aroon Regime Confirmation: Use Aroon Indicator to Confirm Trending vs Range-Bound States
UNEXECUTED

### RUN115 — Supertrend Confirmation: Use Supertrend as Trailing Stop and Entry Direction Filter
UNEXECUTED

### RUN116 — KST Confirmation Filter: Use Know Sure Thing Momentum Oscillator for Entry Timing
UNEXECUTED

### RUN117 — Rate of Change Percentile Filter: Require ROC to Be at Historical Extreme
UNEXECUTED

### RUN118 — Hull Suite Trend Filter: Use Alan Hull's Hull MA for Low-Lag Trend Confirmation
UNEXECUTED

### RUN119 — Vortex Indicator Confirmation: Use VI+ / VI- Crossover for Trend Direction Confirmation
UNEXECUTED

### RUN120 — Mass Index Reversal Filter: Use Mass Index Trend Reversal Detection for Entry Confirmation
UNEXECUTED

### RUN121 — TD Sequential Entry Filter: Use Thomas DeMark's TD Sequential for Countdown Entry Confirmation
UNEXECUTED

### RUN122 — Elder Ray Bull Power Confirmation: Use Dr. Alexander Elder's Bull Power for Entry Direction
UNEXECUTED

### RUN123 — Accumulation/Distribution Line Divergence: Use A/D Line for Institutional Money Flow Confirmation
UNEXECUTED

### RUN124 — Choppiness Index Confirmation: Use CI to Confirm Market Is Choppy/Ranging Before Mean-Reversion Entries
UNEXECUTED

### RUN125 — Ease of Movement Confirmation: Use EMV to Confirm Price Moves "Easily" Before Entry
UNEXECUTED

### RUN126 — Ichimoku Cloud Confirmation: Use Ichimoku Cloud Components for Multi-Dimensional Entry Filtering
UNEXECUTED

### RUN127 — Force Index Confirmation: Use Alexander Elder's Force Index for Momentum Confirmation
UNEXECUTED

### RUN128 — Schaff Trend Cycle Confirmation: Use Schaff Trend Cycle for Fast Trend/Momentum Confirmation
UNEXECUTED

### RUN129 — VWAP Deviation Percentile Filter: Require Price to Be at Historical Extreme Distance from VWAP
UNEXECUTED

### RUN130 — Negative Day Revenue Filter: Use Rolling N-Day Loss Frequency as Market Stress Signal
UNEXECUTED

### RUN131 — Volume-Weighted RSI Confirmation: Require RSI Extreme to Be Volume-Confirmed
UNEXECUTED

### RUN132 — RSI Divergence Confirmation: Require Hidden or Classical RSI Divergence at Entry
UNEXECUTED

### RUN133 — Ultimate Oscillator Confirmation: Use Multi-Timeframe Oscillator for Smoother Entry Confirmation
UNEXECUTED

### RUN134 — Connors RSI Confirmation: Use Short-Term RSI Streak for Faster Entry Confirmation
UNEXECUTED

### RUN135 — Stress Accumulation Meter: Track Consecutive Directional Bars as Market Stress/Exhaustion Signal
UNEXECUTED

### RUN136 — Trade Zone Index Confirmation: Require Price to Be In or Breaking Out of a Consolidation Zone
UNEXECUTED

### RUN137 — Bollinger Band Width Percentile Filter: Require BB Width to Be at Historical Extreme for Entries
UNEXECUTED

### RUN138 — Opening Range Breakout Filter: Suppress Regime Entries When Price Has Broken Out of the Opening Range
UNEXECUTED

### RUN139 — Darvas Box Entry Filter: Use Nicolas Darvas's Box Theory for Support/Resistance and Entry Timing
UNEXECUTED

### RUN140 — Keltner Channel Breakout Filter: Use ATR-Based Channels for Trend/Range Detection
UNEXECUTED

---

## Duplicate Prevention — What NOT to Propose

### Already Tested (Negative Results)
- **Take Profit on regime trades** — RUN8: TP does not help
- **Kelly position sizing** — RUN19: amplifies losses 15-17x
- **Momentum crowding filters** — RUN20: counterproductive
- **BTC RSI sentiment regime** — RUN21: reduces WR to 30.4%
- **Genetic algorithm discovery** — RUN22: overfits, 0/18 OOS
- **Differential evolution** — RUN23: can't fix regime mismatch
- **Ensemble voting/AND** — RUN24: too correlated
- **ML regime detection** — RUN25: circular ML bug
- **ATR trailing stops** — RUN26: WR rises but PF drops
- **MAX_SCALP_OPENS cap** — RUN29: cap irrelevant, scalp WR=10.9%
- **ISO short mitigations** — RUN34: ISO shorts are profit engine, mitigations hurt
- **Z-score momentum filter** — RUN60: proposed but unexecuted
- **RSI divergence confirmation** — RUN61/RUN132: proposed but unexecuted
- **Breakeven stop** — RUN62: proposed but unexecuted
- **BTC trend confirmation** — RUN63: proposed but unexecuted
- **ADX confirmation** — RUN89: proposed but unexecuted

### Already Implemented (Applied to COINCLAW)
- **SL 0.3%** — RUN7
- **Scalp TP=0.8%, SL=0.10%** — RUN10
- **DASH OuMeanRev** — RUN11
- **Scalp regime direction filter** — RUN12
- **Complement signals (Laguerre RSI, Kalman, KST)** — RUN13
- **F6 filter** — RUN10
- **Stochastic 50 exit for scalps** — RUN35

### Never Executed (Unexplored — potential for Ralph Loop)
All RUNs 38-140 are unexecuted except bugs fixed (RUN36, RUN37) and nothing else.
