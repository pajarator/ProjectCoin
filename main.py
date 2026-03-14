"""
Main runner - test all strategies and find the best ones.
"""
import pandas as pd
import numpy as np
from data_fetcher import fetch_ohlcv, DEFAULT_PAIRS
from indicators import add_all_indicators
from backtester import Backtester
from strategies import STRATEGIES
from strategies_enhanced import ENHANCED_STRATEGIES
import warnings
warnings.filterwarnings('ignore')

# Combine all strategies
ALL_STRATEGIES = {**STRATEGIES, **ENHANCED_STRATEGIES}

def test_strategy(df: pd.DataFrame, strategy_func, **kwargs) -> dict:
    """Test a single strategy on a dataframe."""
    try:
        # Get signals
        entry, exit = strategy_func(df, **kwargs)
        
        # Add indicators for reference
        df_with_indicators = add_all_indicators(df.copy())
        
        # Run backtest
        bt = Backtester(df, initial_balance=10000, fee=0.001, slippage=0.0005)
        result = bt.run(entry, exit)
        
        return {
            'strategy': strategy_func.__name__,
            'total_trades': result.total_trades,
            'win_rate': result.win_rate,
            'avg_win': result.avg_win,
            'avg_loss': result.avg_loss,
            'profit_factor': result.profit_factor,
            'max_drawdown': result.max_drawdown,
            'sharpe': result.Sharpe_ratio,
            'trades': result.trades,
            'success': True
        }
    except Exception as e:
        return {
            'strategy': strategy_func.__name__,
            'error': str(e),
            'success': False
        }

def test_all_strategies(symbol: str = 'BTC/USDT', timeframe: str = '1h', 
                        limit: int = 500) -> pd.DataFrame:
    """
    Test all strategies on a symbol and return results sorted by win rate.
    """
    print(f"\n{'='*60}")
    print(f"Testing all strategies on {symbol} ({timeframe})")
    print(f"{'='*60}\n")
    
    # Fetch data
    df = fetch_ohlcv(symbol, timeframe, limit)
    if df.empty:
        print("Failed to fetch data")
        return pd.DataFrame()
    
    # Add all indicators
    df = add_all_indicators(df)
    
    results = []
    
    for name, func in ALL_STRATEGIES.items():
        print(f"Testing {name}...", end=" ")
        
        # Different strategies need different params
        kwargs = {}
        
        result = test_strategy(df, func, **kwargs)
        
        if result['success']:
            print(f"✓ {result['total_trades']} trades, {result['win_rate']:.1f}% win rate")
            results.append(result)
        else:
            print(f"✗ Error: {result.get('error', 'Unknown')}")
    
    # Create results DataFrame
    if results:
        results_df = pd.DataFrame(results)
        results_df = results_df.sort_values('win_rate', ascending=False)
        
        print(f"\n{'='*60}")
        print("TOP STRATEGIES BY WIN RATE:")
        print(f"{'='*60}")
        for _, row in results_df.head(10).iterrows():
            print(f"  {row['strategy']:25} | {row['win_rate']:5.1f}% | {row['total_trades']:3} trades | PF: {row['profit_factor']:.2f}")
        
        return results_df
    
    return pd.DataFrame()

def find_best_strategy(symbol: str = 'BTC/USDT', timeframe: str = '1h',
                       min_trades: int = 10, target_win_rate: float = 70.0) -> dict:
    """
    Find strategy that meets target win rate.
    """
    results_df = test_all_strategies(symbol, timeframe)
    
    if results_df.empty:
        return {'error': 'No results'}
    
    # Filter by minimum trades
    filtered = results_df[results_df['total_trades'] >= min_trades]
    
    # Get best
    best = filtered.iloc[0] if not filtered.empty else results_df.iloc[0]
    
    print(f"\n{'='*60}")
    print(f"BEST STRATEGY: {best['strategy']}")
    print(f"Win Rate: {best['win_rate']:.1f}%")
    print(f"Profit Factor: {best['profit_factor']:.2f}")
    print(f"Total Trades: {best['total_trades']}")
    print(f"{'='*60}")
    
    return best.to_dict()

def optimize_strategy(strategy_name: str, symbol: str = 'BTC/USDT',
                     timeframe: str = '1h') -> dict:
    """
    Try different parameter combinations for a strategy.
    """
    if strategy_name not in ALL_STRATEGIES:
        return {'error': f'Strategy {strategy_name} not found'}
    
    func = ALL_STRATEGIES[strategy_name]
    df = fetch_ohlcv(symbol, timeframe, 500)
    df = add_all_indicators(df)
    
    print(f"\nOptimizing {strategy_name}...")
    
    # Define parameter ranges based on strategy
    param_grid = []
    
    if 'rsi' in strategy_name:
        for ro in [25, 30, 35]:
            for rbo in [65, 70, 75]:
                param_grid.append({'rsi_oversold': ro, 'rsi_overbought': rbo})
    
    elif 'macd' in strategy_name:
        # MACD has no params in current implementation
        param_grid = [{}]
    
    elif 'bb' in strategy_name:
        for period in [15, 20, 25]:
            for std in [1.5, 2.0, 2.5]:
                param_grid.append({'bb_period': period, 'bb_std': std})
    
    elif 'ema' in strategy_name:
        for fast in [5, 9, 12]:
            for slow in [15, 21, 30]:
                if fast < slow:
                    param_grid.append({'fast_period': fast, 'slow_period': slow})
    
    else:
        param_grid = [{}]
    
    results = []
    for params in param_grid:
        result = test_strategy(df.copy(), func, **params)
        if result['success']:
            result['params'] = params
            results.append(result)
    
    if results:
        results = sorted(results, key=lambda x: x['win_rate'], reverse=True)
        best = results[0]
        
        print(f"\nBest parameters: {best['params']}")
        print(f"Win rate: {best['win_rate']:.1f}%")
        print(f"Profit factor: {best['profit_factor']:.2f}")
        
        return best
    
    return {'error': 'No valid parameters found'}

def test_multiple_symbols(timeframe: str = '1h', min_win_rate: float = 50.0) -> pd.DataFrame:
    """
    Test best strategy across multiple symbols.
    """
    print(f"\n{'='*60}")
    print("Testing top strategies across multiple symbols")
    print(f"{'='*60}\n")
    
    all_results = []
    
    for symbol in DEFAULT_PAIRS[:10]:  # Test top 10 pairs
        print(f"\n--- {symbol} ---")
        results_df = test_all_strategies(symbol, timeframe)
        
        if not results_df.empty:
            results_df['symbol'] = symbol
            all_results.append(results_df)
    
    if all_results:
        combined = pd.concat(all_results)
        combined = combined.sort_values('win_rate', ascending=False)
        
        print(f"\n{'='*60}")
        print("TOP 20 RESULTS ACROSS ALL SYMBOLS:")
        print(f"{'='*60}")
        print(combined[['symbol', 'strategy', 'win_rate', 'total_trades', 'profit_factor']].head(20).to_string())
        
        return combined
    
    return pd.DataFrame()

if __name__ == "__main__":
    # Quick test
    results = test_all_strategies('BTC/USDT', '1h', 500)
