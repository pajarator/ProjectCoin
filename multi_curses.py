#!/usr/bin/env python3
"""
Multi-Coin Paper Trading - Adaptive Regime Detection
Detects market conditions per coin and selects the best backtested strategy.
"""
import ccxt
import pandas as pd
import time
import curses
import json
import os
import argparse
from datetime import datetime
from collections import deque

STATE_FILE = '/home/scamarena/ProjectCoin/trading_state.json'
LOG_FILE = '/home/scamarena/ProjectCoin/trading_log.txt'

parser = argparse.ArgumentParser()
parser.add_argument('--reset', action='store_true', help='Reset all balances and trades')
args = parser.parse_args()

if args.reset:
    for f in (STATE_FILE, LOG_FILE):
        if os.path.exists(f):
            os.remove(f)
    print("State reset!")

# === TOP 20 COINS ===
# === COINS (from RUN3 validated assignments, TRX removed: only loser at -6.1%) ===
# Preferred strategy is used when regime allows it; adaptive overrides in squeeze
COINS = [
    {'symbol': 'DASH/USDT',  'tf': '15m', 'name': 'DASH',  'pref': 'adr_rev'},     # +89.3%
    {'symbol': 'LINK/USDT',  'tf': '15m', 'name': 'LINK',  'pref': 'vwap_rev'},    # +35.6%
    {'symbol': 'AVAX/USDT',  'tf': '15m', 'name': 'AVAX',  'pref': 'bb_bounce'},   # +31.8%
    {'symbol': 'LTC/USDT',   'tf': '15m', 'name': 'LTC',   'pref': 'adr_rev'},     # +31.6%
    {'symbol': 'DOT/USDT',   'tf': '15m', 'name': 'DOT',   'pref': 'mean_rev'},    # +29.0%
    {'symbol': 'UNI/USDT',   'tf': '15m', 'name': 'UNI',   'pref': 'mean_rev'},    # +28.3%
    {'symbol': 'XLM/USDT',   'tf': '15m', 'name': 'XLM',   'pref': 'vwap_rev'},    # +23.9%
    {'symbol': 'ADA/USDT',   'tf': '15m', 'name': 'ADA',   'pref': 'mean_rev'},    # +22.3%
    {'symbol': 'ATOM/USDT',  'tf': '15m', 'name': 'ATOM',  'pref': 'dual_rsi'},    # +17.2%
    {'symbol': 'NEAR/USDT',  'tf': '15m', 'name': 'NEAR',  'pref': 'mean_rev'},    # +16.1%
    {'symbol': 'SOL/USDT',   'tf': '15m', 'name': 'SOL',   'pref': 'vwap_rev'},    # +16.1%
    {'symbol': 'ETH/USDT',   'tf': '15m', 'name': 'ETH',   'pref': 'mean_rev'},    # +13.2%
    {'symbol': 'XRP/USDT',   'tf': '15m', 'name': 'XRP',   'pref': 'vwap_rev'},    # +12.9%
    {'symbol': 'DOGE/USDT',  'tf': '15m', 'name': 'DOGE',  'pref': 'mean_rev'},    # +11.1%
    {'symbol': 'BTC/USDT',   'tf': '15m', 'name': 'BTC',   'pref': 'bb_bounce'},   # +4.3%
    {'symbol': 'SHIB/USDT',  'tf': '15m', 'name': 'SHIB',  'pref': 'mean_rev'},    # +3.8%
    {'symbol': 'BNB/USDT',   'tf': '15m', 'name': 'BNB',   'pref': 'mean_rev'},    # +2.7%
    {'symbol': 'ALGO/USDT',  'tf': '15m', 'name': 'ALGO',  'pref': 'vwap_rev'},    # +1.9%
    {'symbol': 'MATIC/USDT', 'tf': '15m', 'name': 'MATIC', 'pref': 'mean_rev'},    # (new)
]

INITIAL_CAPITAL = 100
RISK = 0.10              # 10% per trade (validated in RUN3)
LEVERAGE = 5             # 5x leverage
STOP_LOSS = 0.015        # 1.5% stop loss (validated: 33% of exits)
MIN_HOLD_CANDLES = 8     # 8 candles = 2 hours (validated: lets winners run to +4.28% avg)
FEE = 0.001
SLIP = 0.0005
LOG_LINES = 50

# === REGIME DEFINITIONS ===
REGIME_RANGING = 'RANGE'
REGIME_WEAK_TREND = 'WTREND'
REGIME_STRONG_TREND = 'STREND'
REGIME_HIGH_VOL = 'HIVOL'
REGIME_SQUEEZE = 'SQUEEZE'

class Logger:
    def __init__(self, log_file):
        self.log_file = log_file
        self.entries = deque(maxlen=LOG_LINES)
        if os.path.exists(log_file):
            with open(log_file, 'r') as f:
                for line in f:
                    self.entries.append(line.rstrip())

    def log(self, msg):
        timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        entry = f"[{timestamp}] {msg}"
        self.entries.append(entry)
        with open(self.log_file, 'a') as f:
            f.write(entry + '\n')
        return entry

logger = Logger(LOG_FILE)


def fmt_price(p):
    """Format price with appropriate decimals for micro-priced coins."""
    if p < 0.01:
        return f"${p:.6f}"
    elif p < 1:
        return f"${p:.4f}"
    return f"${p:.2f}"


def detect_regime(i):
    """Detect market regime from indicators."""
    adx = i['adx']
    bb_width = i['bb_width']
    bb_width_avg = i['bb_width_avg']

    if not pd.isna(bb_width_avg) and bb_width < bb_width_avg * 0.6:
        return REGIME_SQUEEZE
    elif not pd.isna(bb_width_avg) and bb_width > bb_width_avg * 1.5:
        return REGIME_HIGH_VOL
    elif not pd.isna(adx) and adx > 30:
        return REGIME_STRONG_TREND
    elif not pd.isna(adx) and adx > 20:
        return REGIME_WEAK_TREND
    else:
        return REGIME_RANGING


class Trader:
    def __init__(self, s, tf, name, pref):
        self.sym = s
        self.tf = tf
        self.name = name
        self.pref = pref            # validated preferred strategy from RUN3
        self.ex = ccxt.binance({'enableRateLimit': True})
        self.bal = INITIAL_CAPITAL
        self.pos = None
        self.trades = []
        self.cooldown = 0
        self.candles_held = 0
        self.regime = '...'
        self.active_strat = None
        self.load_state()

    def load_state(self):
        if os.path.exists(STATE_FILE):
            try:
                with open(STATE_FILE, 'r') as f:
                    state = json.load(f)
                if self.name in state:
                    self.bal = state[self.name].get('bal', INITIAL_CAPITAL)
                    self.pos = state[self.name].get('pos', None)
                    self.trades = state[self.name].get('trades', [])
                    self.candles_held = state[self.name].get('candles_held', 0)
                    self.cooldown = state[self.name].get('cooldown', 0)
            except:
                pass

    def save_state(self):
        state = {}
        if os.path.exists(STATE_FILE):
            try:
                with open(STATE_FILE, 'r') as f:
                    state = json.load(f)
            except:
                pass
        state[self.name] = {
            'bal': self.bal,
            'pos': self.pos,
            'trades': self.trades,
            'candles_held': self.candles_held,
            'cooldown': self.cooldown,
        }
        with open(STATE_FILE, 'w') as f:
            json.dump(state, f)

    def ind(self):
        try:
            d = self.ex.fetch_ohlcv(self.sym, self.tf, limit=50)
            df = pd.DataFrame(d, columns=['t','o','h','l','c','v'])

            sma20 = df['c'].rolling(20).mean()
            sma9 = df['c'].rolling(9).mean()
            std20 = df['c'].rolling(20).std()

            # RSI
            delta = df['c'].diff()
            gain = (delta.where(delta > 0, 0)).rolling(14).mean()
            loss = (-delta.where(delta < 0, 0)).rolling(14).mean()
            rs = gain / loss
            rsi = 100 - (100 / (1 + rs))

            # RSI short (7-period for dual_rsi)
            gain7 = (delta.where(delta > 0, 0)).rolling(7).mean()
            loss7 = (-delta.where(delta < 0, 0)).rolling(7).mean()
            rs7 = gain7 / loss7
            rsi7 = 100 - (100 / (1 + rs7))

            # MACD
            ema12 = df['c'].ewm(span=12, adjust=False).mean()
            ema26 = df['c'].ewm(span=26, adjust=False).mean()
            macd = ema12 - ema26
            signal = macd.ewm(span=9, adjust=False).mean()

            # Volume SMA
            vol_ma = df['v'].rolling(20).mean()

            # ATR
            high_low = df['h'] - df['l']
            high_close = abs(df['h'] - df['c'].shift())
            low_close = abs(df['l'] - df['c'].shift())
            tr = pd.concat([high_low, high_close, low_close], axis=1).max(axis=1)
            atr = tr.rolling(14).mean()

            # ADX
            plus_dm = high_low.where((df['h'] - df['h'].shift()) > (df['l'].shift() - df['l']), 0)
            minus_dm = high_low.where((df['l'].shift() - df['l']) > (df['h'] - df['h'].shift()), 0)
            plus_di = 100 * (plus_dm.rolling(14).mean() / atr)
            minus_di = 100 * (minus_dm.rolling(14).mean() / atr)
            dx = 100 * abs(plus_di - minus_di) / (plus_di + minus_di)
            adx = dx.rolling(14).mean()

            # Bollinger Bands
            bb_upper = sma20 + 2 * std20
            bb_lower = sma20 - 2 * std20

            # VWAP (rolling 20)
            typical_price = (df['h'] + df['l'] + df['c']) / 3
            vwap = (typical_price * df['v']).rolling(20).sum() / df['v'].rolling(20).sum()

            # BB width for regime detection
            bb_w = bb_upper - bb_lower
            bb_w_avg = bb_w.rolling(20).mean()

            # Guard against NaN from unfilled rolling windows
            if pd.isna(sma20.iloc[-1]) or pd.isna(std20.iloc[-1]) or std20.iloc[-1] == 0:
                return None

            return {
                'p': df['c'].iloc[-1],
                'z': (df['c'].iloc[-1] - sma20.iloc[-1]) / std20.iloc[-1],
                'sma20': sma20.iloc[-1],
                'sma9': sma9.iloc[-1],
                'bb_lo': bb_lower.iloc[-1],
                'bb_hi': bb_upper.iloc[-1],
                'bb_width': bb_w.iloc[-1],
                'bb_width_avg': bb_w_avg.iloc[-1],
                'vol': df['v'].iloc[-1],
                'vol_ma': vol_ma.iloc[-1],
                'rsi': rsi.iloc[-1],
                'rsi7': rsi7.iloc[-1],
                'macd': macd.iloc[-1],
                'macd_signal': signal.iloc[-1],
                'macd_hist': macd.iloc[-1] - signal.iloc[-1],
                'atr': atr.iloc[-1],
                'adx': adx.iloc[-1],
                'vwap': vwap.iloc[-1],
                'high_24': df['h'].rolling(24).max().iloc[-1],
                'low_24': df['l'].rolling(24).min().iloc[-1],
            }
        except Exception as e:
            logger.log(f"ERROR {self.name}: {e}")
            return None

    def entry(self, i, strat):
        if self.cooldown > 0:
            self.cooldown -= 1
            return False

        # Skip if any key indicator is NaN
        if any(pd.isna(i.get(k, float('nan'))) for k in ('rsi', 'adx', 'vol_ma', 'bb_width_avg')):
            return False

        # Don't enter if price is already above SMA20 (immediate SMA exit) or z > 0.5 (immediate Z0 exit)
        if i['p'] > i['sma20'] or i['z'] > 0.5:
            return False

        if strat == 'vwap_rev':
            # VWAP Reversion: z-score dip + below VWAP + volume (86.8% avg)
            return i['z'] < -1.5 and i['p'] < i['vwap'] and i['vol'] > i['vol_ma'] * 1.2

        elif strat == 'bb_bounce':
            # BB Bounce: price at lower band + volume (81.4% avg)
            return i['p'] <= i['bb_lo'] * 1.02 and i['vol'] > i['vol_ma'] * 1.3

        elif strat == 'dual_rsi':
            # Dual RSI: both RSI-14 and RSI-7 oversold in trend (79.3% avg)
            rsi7 = i.get('rsi7', i['rsi'])
            return i['rsi'] < 40 and rsi7 < 30 and i['sma9'] > i['sma20']

        elif strat == 'adr_rev':
            # ADR Reversal: price in bottom 25% of 24-candle range (77.6% avg)
            adr = i['high_24'] - i['low_24']
            if adr <= 0:
                return False
            return i['p'] <= i['low_24'] + adr * 0.25 and i['vol'] > i['vol_ma'] * 1.1

        elif strat == 'mean_rev':
            # Mean Reversion: z-score dip (validated in RUN3: +13-89% across coins)
            return i['z'] < -1.5

        return False

    def exit(self, i):
        if not self.pos:
            return None

        # Update high price for trailing stop
        if i['p'] > self.pos.get('high', self.pos['e']):
            self.pos['high'] = i['p']

        # Only count new candles (price changes), not repeated ticks
        if i['p'] != self.pos.get('last_price'):
            self.candles_held += 1
            self.pos['last_price'] = i['p']
        self.save_state()

        pnl = (i['p'] - self.pos['e']) / self.pos['e']
        held = self.candles_held

        # 1. Stop Loss (validated: 33% of exits in RUN3)
        if pnl <= -STOP_LOSS:
            self.candles_held = 0
            return 'SL', pnl, held

        # 2. Signal exits: only after MIN_HOLD_CANDLES (8 = 2hrs) AND in profit
        #    (validated: 67% of exits in RUN3 — lets winners run to +4.28% avg)
        if pnl > 0 and self.candles_held >= MIN_HOLD_CANDLES:
            # Price crosses above SMA20
            if i['p'] > i['sma20']:
                self.candles_held = 0
                return 'SMA', pnl, held
            # Z-score reverts to mean
            if i['z'] > 0.5:
                self.candles_held = 0
                return 'Z0', pnl, held

        return None

    def buy(self, p, regime, strat, i):
        if self.pos:
            return
        trade_amt = self.bal * RISK
        sz = (trade_amt * LEVERAGE) / p
        fee = trade_amt * FEE
        self.bal = self.bal - trade_amt - fee
        self.pos = {'e': p, 's': sz, 'high': p}
        self.save_state()

        vol_r = i['vol'] / i['vol_ma'] if i['vol_ma'] > 0 else 0
        why = self._entry_reason(strat, i)
        logger.log(
            f"BUY {self.name} [{regime}>{strat}] @ {fmt_price(p)} | "
            f"RSI:{i['rsi']:.0f} Z:{i['z']:+.2f} ADX:{i['adx']:.0f} Vol:{vol_r:.1f}x | "
            f"{why} | Cost:${trade_amt:.2f} Bal:${self.bal:.2f}"
        )

    def _entry_reason(self, strat, i):
        """Human-readable explanation of why entry triggered."""
        vol_r = i['vol'] / i['vol_ma'] if i['vol_ma'] > 0 else 0
        if strat == 'vwap_rev':
            return f"Z {i['z']:+.2f}<-1.5, P<VWAP({fmt_price(i['vwap'])}), Vol {vol_r:.1f}x>1.2x"
        elif strat == 'bb_bounce':
            return f"P({fmt_price(i['p'])})<=BB_lo({fmt_price(i['bb_lo'])})*1.02, Vol {vol_r:.1f}x>1.3x"
        elif strat == 'dual_rsi':
            rsi7 = i.get('rsi7', i['rsi'])
            return f"RSI14:{i['rsi']:.0f}<40, RSI7:{rsi7:.0f}<30, SMA9>SMA20(uptrend)"
        elif strat == 'adr_rev':
            adr = i['high_24'] - i['low_24']
            threshold = i['low_24'] + adr * 0.25
            return f"P({fmt_price(i['p'])})<=ADR25%({fmt_price(threshold)}), Vol {vol_r:.1f}x>1.1x"
        elif strat == 'mean_rev':
            return f"Z {i['z']:+.2f}<-1.5, price below mean"
        return "unknown"

    def sell(self, p, reason, i, held):
        if not self.pos:
            return

        proceeds = self.pos['s'] * p
        fee = proceeds * FEE
        net_proceeds = proceeds - fee
        cost = self.pos['s'] * self.pos['e']
        margin = cost / LEVERAGE
        pnl = net_proceeds - cost
        pnl_pct = (pnl / margin) * 100

        self.bal += margin + pnl
        self.trades.append({'pnl': pnl, 'reason': reason})
        entry_price = self.pos['e']
        high_price = self.pos.get('high', entry_price)
        self.pos = None
        self.cooldown = 2
        self.candles_held = 0
        self.save_state()

        vol_r = i['vol'] / i['vol_ma'] if i['vol_ma'] > 0 else 0
        why = self._exit_reason(reason, i, entry_price, high_price)
        logger.log(
            f"SELL {self.name} ({reason}) @ {fmt_price(p)} | "
            f"RSI:{i['rsi']:.0f} Z:{i['z']:+.2f} MACD:{i['macd_hist']:+.4f} | "
            f"{why} | Held:{held} PnL:${pnl:.2f}({pnl_pct:+.1f}%) Bal:${self.bal:.2f}"
        )
        return pnl

    def _exit_reason(self, reason, i, entry_price, high_price):
        """Human-readable explanation of why exit triggered."""
        price_chg = (i['p'] - entry_price) / entry_price * 100
        if reason == 'SL':
            return f"Stop loss hit: {price_chg:+.2f}% from entry {fmt_price(entry_price)}"
        elif reason == 'SMA':
            return f"Price {fmt_price(i['p'])} crossed above SMA20 {fmt_price(i['sma20'])}"
        elif reason == 'Z0':
            return f"Z-score reverted to {i['z']:+.2f} (>0.5)"
        return "unknown"


traders = [Trader(c['symbol'], c['tf'], c['name'], c['pref']) for c in COINS]

def main(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(1)

    curses.start_color()
    curses.init_pair(1, curses.COLOR_CYAN, curses.COLOR_BLACK)
    curses.init_pair(2, curses.COLOR_WHITE, curses.COLOR_BLACK)
    curses.init_pair(3, curses.COLOR_GREEN, curses.COLOR_BLACK)
    curses.init_pair(4, curses.COLOR_RED, curses.COLOR_BLACK)
    curses.init_pair(5, curses.COLOR_YELLOW, curses.COLOR_BLACK)
    curses.init_pair(6, curses.COLOR_MAGENTA, curses.COLOR_BLACK)

    while True:
        stdscr.clear()
        h, term_w = stdscr.getmaxyx()

        min_height = 30
        if h < min_height:
            stdscr.addstr(0, 0, f"Terminal too small! Need {min_height} rows, you have {h}. Resize and try again.")
            stdscr.refresh()
            time.sleep(2)
            continue

        log_height = max(10, h - 30)
        table_height = h - log_height - 4

        # === HEADER ===
        stdscr.addstr(0, 0, "=" * min(95, term_w-1), curses.color_pair(1) | curses.A_BOLD)
        stdscr.addstr(1, 0, f" COINCLAW v3 - {len(COINS)} COINS | {len(COINS)}x${INITIAL_CAPITAL} = ${len(COINS)*INITIAL_CAPITAL}", curses.color_pair(1) | curses.A_BOLD)
        stdscr.addstr(2, 0, f" Risk:{RISK*100:.0f}% | {LEVERAGE}x LEV | SL:{STOP_LOSS*100:.1f}% | Hold:{MIN_HOLD_CANDLES}candles | Exit:SL+SMA+Z0 | {datetime.now().strftime('%H:%M:%S')}")
        stdscr.addstr(3, 0, "=" * min(95, term_w-1), curses.color_pair(1) | curses.A_BOLD)

        y = 5
        # Column positions
        C_NUM=0; C_COIN=3; C_REGIME=11; C_STRAT=19; C_PRICE=30; C_RSI=42; C_Z=47; C_VOL=55; C_POS=61; C_PNL=68; C_BAL=76; C_WIN=85
        stdscr.addstr(y, C_NUM,    " #")
        stdscr.addstr(y, C_COIN,   "COIN")
        stdscr.addstr(y, C_REGIME, "REGIME")
        stdscr.addstr(y, C_STRAT,  "STRAT")
        stdscr.addstr(y, C_PRICE,  "      PRICE")
        stdscr.addstr(y, C_RSI,    "RSI")
        stdscr.addstr(y, C_Z,      "Z-SCORE")
        stdscr.addstr(y, C_VOL,    "VOL")
        stdscr.addstr(y, C_POS,    "POS")
        stdscr.addstr(y, C_PNL,    "P&L")
        stdscr.addstr(y, C_BAL,    "BAL")
        stdscr.addstr(y, C_WIN,    "W")
        y += 1
        stdscr.addstr(y, 0, "-" * min(95, term_w-1), curses.color_pair(1))
        y += 1

        total = 0
        wins = 0
        trades = 0

        for idx, t in enumerate(traders):
            if idx >= 20:
                break

            try:
                i = t.ind()
            except:
                i = None

            if i is None:
                continue

            # Detect regime; use preferred strategy unless squeeze
            regime = detect_regime(i)
            t.regime = regime
            strat = None if regime == REGIME_SQUEEZE else t.pref
            t.active_strat = strat

            # Process signals
            if not t.pos and strat is not None and t.entry(i, strat):
                t.buy(i['p'], regime, strat, i)
            elif t.pos:
                result = t.exit(i)
                if result:
                    reason, pnl, held = result
                    t.sell(i['p'], reason, i, held)

            # Position & P&L
            if t.pos:
                pnl = (i['p'] - t.pos['e']) / t.pos['e'] * LEVERAGE * 100
                pos = "LONG"
                pnl_str = f"{pnl:+.1f}%"
                pos_color = curses.color_pair(3) if pnl >= 0 else curses.color_pair(4)
            else:
                pos = "CASH"
                pnl_str = "-"
                pos_color = curses.color_pair(2)

            bal_color = curses.color_pair(3) if t.bal >= INITIAL_CAPITAL else curses.color_pair(4)

            # Z-score color
            z = i['z']
            z_str = f"{z:+.2f}"
            if z < -1.5:
                z_color = curses.color_pair(3)
            elif z < -1.0:
                z_color = curses.color_pair(5)
            elif z > 1.5:
                z_color = curses.color_pair(4)
            else:
                z_color = curses.color_pair(2)

            # RSI color
            rsi = i['rsi']
            rsi_str = f"{rsi:.0f}"
            if rsi < 30:
                rsi_color = curses.color_pair(3)
            elif rsi > 70:
                rsi_color = curses.color_pair(4)
            else:
                rsi_color = curses.color_pair(2)

            vol_r = i['vol'] / i['vol_ma'] if i['vol_ma'] > 0 else 0
            vol_str = f"{vol_r:.1f}x"
            vol_color = curses.color_pair(3) if vol_r > 1.2 else (curses.color_pair(5) if vol_r > 0.8 else curses.color_pair(4))

            # Regime color
            if regime == REGIME_SQUEEZE:
                regime_color = curses.color_pair(4)
            elif regime == REGIME_HIGH_VOL:
                regime_color = curses.color_pair(5)
            elif regime == REGIME_STRONG_TREND:
                regime_color = curses.color_pair(3)
            elif regime == REGIME_WEAK_TREND:
                regime_color = curses.color_pair(6)
            else:
                regime_color = curses.color_pair(2)

            strat_str = strat if strat else "---"

            win_count = sum(1 for x in t.trades if x['pnl'] > 0)
            win_str = f"{win_count}/{len(t.trades)}" if t.trades else "-"

            wins += win_count
            trades += len(t.trades)

            stdscr.addstr(y, C_NUM, f"{idx+1:2}")
            stdscr.addstr(y, C_COIN, f"{t.name:<7}", curses.color_pair(6))
            stdscr.addstr(y, C_REGIME, f"{regime:<7}", regime_color)
            stdscr.addstr(y, C_STRAT, f"{strat_str:<10}")
            # Auto-format price: more decimals for micro-priced coins
            price = i['p']
            if price < 0.01:
                price_str = f"${price:>10.6f}"
            elif price < 1:
                price_str = f"${price:>10.4f}"
            else:
                price_str = f"${price:>10.2f}"
            stdscr.addstr(y, C_PRICE, price_str)
            stdscr.addstr(y, C_RSI, f"{rsi_str:>3}", rsi_color)
            stdscr.addstr(y, C_Z, f"{z_str:>6}", z_color)
            stdscr.addstr(y, C_VOL, f"{vol_str:>4}", vol_color)
            stdscr.addstr(y, C_POS, f"{pos:<5}", pos_color)
            stdscr.addstr(y, C_PNL, f"{pnl_str:>7}")
            stdscr.addstr(y, C_BAL, f"${t.bal:>7.0f}", bal_color)
            stdscr.addstr(y, C_WIN, f"{win_str}")

            y += 1
            total += t.bal

        # Total row
        y += 1
        stdscr.addstr(y, 0, "-" * min(95, term_w-1), curses.color_pair(1))
        y += 1

        total_pnl = total - len(COINS)*INITIAL_CAPITAL
        pnl_color = curses.color_pair(3) if total_pnl >= 0 else curses.color_pair(4)
        wr_str = f"{wins}/{trades}" if trades > 0 else "-"
        wr_pct = f"({100*wins/trades:.0f}%)" if trades > 0 else ""

        stdscr.addstr(y, 0, f" TOTAL: ${total:.0f} (", curses.color_pair(1) | curses.A_BOLD)
        stdscr.addstr(f"{total_pnl:.0f})", pnl_color | curses.A_BOLD)
        stdscr.addstr(f" | {trades} trades | W: {wr_str} {wr_pct} | 'q' quit")

        # === BOTTOM WINDOW: Log ===
        log_y = table_height + 1
        stdscr.addstr(log_y, 0, "=" * min(95, term_w-1), curses.color_pair(1) | curses.A_BOLD)
        log_y += 1
        stdscr.addstr(log_y, 0, " TRANSACTION LOG", curses.color_pair(1) | curses.A_BOLD)
        log_y += 1
        stdscr.addstr(log_y, 0, "-" * min(95, term_w-1), curses.color_pair(1))
        log_y += 1

        log_entries = list(logger.entries)[-log_height:]
        for entry in log_entries:
            if log_y >= h - 1:
                break
            if 'BUY' in entry:
                color = curses.color_pair(3)
            elif 'SELL' in entry:
                color = curses.color_pair(3) if '+' in entry else curses.color_pair(4)
            else:
                color = curses.color_pair(2)
            # Wrap long lines across multiple rows
            w = term_w - 1
            for start in range(0, len(entry), w):
                if log_y >= h - 1:
                    break
                stdscr.addstr(log_y, 0, entry[start:start+w], color)
                log_y += 1

        stdscr.refresh()

        key = stdscr.getch()
        if key == ord('q'):
            break

        time.sleep(5)

curses.wrapper(main)
