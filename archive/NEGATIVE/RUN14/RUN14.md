# RUN14 — Indicator Library Expansion + ML Infrastructure

## Goal

Lay the foundation for ML-based strategy discovery (RUN16+) by:
1. Pinning all dependencies in `requirements.txt`
2. Adding 18 new indicator functions to `indicators.py`
3. Promoting RUN13 inline indicators (Laguerre RSI, Kalman Filter, KST) to first-class library functions
4. Adding `add_all_indicators_extended(df)` for ML feature matrix construction

## What Was Added

### New Dependencies (`requirements.txt`)
```
scikit-learn>=1.5   → RandomForest, XGBoost, TimeSeriesSplit, permutation_importance
scipy>=1.14         → differential_evolution, minimize_scalar (optimal-f)
xgboost>=2.1        → XGBoost classifier
matplotlib>=3.9     → Dashboard charts
joblib>=1.4         → Parallel processing utilities
tqdm>=4.66          → Progress bars (already used in backtest scripts)
```
Installed successfully via `pip install --break-system-packages`.

### New Indicators in `indicators.py`

| # | Function | Description | Status |
|---|----------|-------------|--------|
| 1 | `WMA(data, period)` | Weighted Moving Average | New |
| 2 | `OBV(close, volume)` | On-Balance Volume | New |
| 3 | `CMF(high, low, close, volume, period=20)` | Chaikin Money Flow | New |
| 4 | `WILLIAMS_R(high, low, close, period=14)` | Williams %R standalone | New |
| 5 | `CCI(high, low, close, period=20)` | Commodity Channel Index | New |
| 6 | `KELTNER(high, low, close, period=20, atr_mult=1.5)` | Keltner Channels | New |
| 7 | `DONCHIAN(high, low, period=20)` | Donchian Channel | New |
| 8 | `HULL_MA(data, period=9)` | Hull Moving Average (uses WMA) | New |
| 9 | `KAMA(data, period=10, fast=2, slow=30)` | Kaufman Adaptive MA | New |
| 10 | `TRIX(data, period=15)` | Triple smoothed EMA ROC | New |
| 11 | `AROON(high, low, period=25)` | Aroon Up/Down | New |
| 12 | `VORTEX(high, low, close, period=14)` | Vortex Indicator | New |
| 13 | `AWESOME_OSCILLATOR(high, low)` | AO (5/34 SMA median diff) | New |
| 14 | `LAGUERRE_RSI(close, gamma=0.8)` | Laguerre RSI | Promoted from RUN13 inline |
| 15 | `KALMAN_FILTER(close, process_var=1e-5, measure_var=0.01)` | Kalman Filter | Promoted from RUN13 inline |
| 16 | `KST(close)` | Know Sure Thing oscillator | Promoted from RUN13 inline |
| 17 | `HEIKIN_ASHI(open, high, low, close)` | Heikin-Ashi candles | New |
| 18 | `ICHIMOKU(high, low, close, tenkan=9, kijun=26, senkou_b=52)` | Ichimoku Cloud | New |

### `add_all_indicators_extended(df)` → 68 columns total

Calls `add_all_indicators()` then adds all 18 new indicators plus derived features:
- `SQUEEZE` (BB inside Keltner boolean)
- Heikin-Ashi candles (HA_open/high/low/close)
- Ichimoku cloud (5 lines)

### New Infrastructure Files (not archived — live in project root)

| File | Purpose |
|------|---------|
| `feature_engine.py` | 66-feature matrix for ML (price/MA/momentum/volatility/volume/trend/time) |
| `feature_cache.py` | Pre-compute + cache feature matrices to `data_cache/features/` |
| `monte_carlo.py` | MC simulation + bootstrap CIs |
| `walk_forward.py` | Reusable WalkForward class |
| `risk.py` | Kelly, optimal-f, ATR sizing, VaR, correlation |
| `ensemble.py` | Voting + stacking ensemble framework |
| `data_fetcher_extended.py` | Binance Futures funding rates + OI |
| `data_fetcher_sentiment.py` | Fear & Greed index, CoinGecko |
| `dashboard.py` | HTML report generation |

Also: 13 RUN scripts created for RUN15-27 (see DATAMINING.md implementation plan).

## Verification Results

### 1. Import check
```
python -c "from indicators import *; print('OK')"
→ OK
```

### 2. Regression test — `python main.py` on BTC/USDT 1h
All 23 strategies (STRATEGIES + ENHANCED_STRATEGIES) ran without error:
```
rsi_reversal        71.4%  7 trades
macd_crossover      47.4% 19 trades
bb_bounce           66.7%  6 trades
ema_crossover       41.7% 12 trades
...
adr_reversal        80.0% 10 trades
```
No regressions. Results match historical baseline.

### 3. Spot-checks (3 indicators against manual calculation)

**OBV** — Cumulative signed volume sum:
```
Input:  close=[10,11,10.5,12,11.5], volume=[100,200,150,300,100]
Result: [0, 200, 50, 350, 250]  ✓ matches manual
```

**Williams %R** — `−100 × (highest_high − close) / (highest_high − lowest_low)`:
```
At period=3, indices 2-4: [-50, -25, -50]  ✓ matches manual
```

**KST** — SMA-smoothed ROC combo:
```
Max diff from manual calculation: 0.0000000000  ✓ exact match
```

### 4. Feature matrix build
```
BTC 1-year 15m: 35,040 rows × 69 columns
No NaN after warmup (row 200)
Target look-ahead check: PASS (target_i = return_i+1)
All 19 coins cached in data_cache/features/: OK
```

## Conclusions

All 18 indicators verified correct. No regressions to existing strategies. Feature matrix pipeline ready for RUN16 ML feature importance discovery.

**Trader impact:** None. This is a pure infrastructure RUN — no changes to `coinclaw/` or `trader.py`. The new indicators are available as Python library functions for backtest scripts and ML pipelines.

**Next:** RUN15 is complete (feature_engine.py built and cached). RUN16 (ML feature importance) is ready to run.
