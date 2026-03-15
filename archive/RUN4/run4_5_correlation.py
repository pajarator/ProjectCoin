#!/usr/bin/env python3
"""
RUN4.5 - Correlation Analysis & Allocation
Reduce drawdown through diversification.
- Analyze return correlations between coins
- Find optimal allocation (not equal weight)
- Identify redundant coins
"""
import pandas as pd
import numpy as np
import json
import os

DATA_CACHE_DIR = '/home/scamarena/ProjectCoin/data_cache'

COINS = ['DASH','UNI','NEAR','ADA','LTC','SHIB','LINK','ETH','DOT','XRP',
         'ATOM','SOL','DOGE','XLM','AVAX','ALGO','BNB','BTC']

# RUN4.2 optimal PFs
OPTIMAL_PF = {
    'DASH': 2.24, 'UNI': 2.14, 'NEAR': 2.00, 'ADA': 1.99,
    'LTC': 1.88, 'SHIB': 1.85, 'LINK': 1.83, 'ETH': 1.81,
    'DOT': 1.81, 'XRP': 1.76, 'ATOM': 1.71, 'SOL': 1.71,
    'DOGE': 1.69, 'XLM': 1.62, 'AVAX': 1.59, 'ALGO': 1.55,
    'BNB': 1.49, 'BTC': 1.44,
}


def load_returns(name):
    cache_file = f"{DATA_CACHE_DIR}/{name}_USDT_15m_5months.csv"
    if not os.path.exists(cache_file):
        return None
    df = pd.read_csv(cache_file, index_col=0, parse_dates=True)
    # Resample to 1h for less noise in correlation
    hourly = df['c'].resample('1h').last().dropna()
    returns = hourly.pct_change().dropna()
    return returns


def main():
    print("=" * 80)
    print("RUN4.5 - CORRELATION ANALYSIS & ALLOCATION")
    print("=" * 80)

    # Load all returns
    returns_dict = {}
    for coin in COINS:
        r = load_returns(coin)
        if r is not None:
            returns_dict[coin] = r

    print(f"Loaded {len(returns_dict)} coins")

    # Build returns matrix
    returns_df = pd.DataFrame(returns_dict)
    returns_df = returns_df.dropna()
    print(f"Common timeframe: {len(returns_df)} hourly bars")

    # === CORRELATION MATRIX ===
    corr = returns_df.corr()

    print(f"\n{'='*80}")
    print("CORRELATION MATRIX (hourly returns)")
    print(f"{'='*80}\n")

    # Print header
    print(f"{'':>6}", end="")
    for c in COINS:
        print(f"{c:>6}", end="")
    print()

    for c1 in COINS:
        print(f"{c1:>6}", end="")
        for c2 in COINS:
            val = corr.loc[c1, c2]
            print(f"{val:>6.2f}", end="")
        print()

    # === HIGH CORRELATIONS (redundant pairs) ===
    print(f"\n{'='*80}")
    print("HIGHLY CORRELATED PAIRS (>0.7) - potential redundancy")
    print(f"{'='*80}\n")

    high_corr_pairs = []
    for i, c1 in enumerate(COINS):
        for c2 in COINS[i+1:]:
            val = corr.loc[c1, c2]
            if val > 0.7:
                high_corr_pairs.append((c1, c2, val))

    high_corr_pairs.sort(key=lambda x: -x[2])
    for c1, c2, val in high_corr_pairs:
        pf1 = OPTIMAL_PF.get(c1, 0)
        pf2 = OPTIMAL_PF.get(c2, 0)
        keep = c1 if pf1 >= pf2 else c2
        drop = c2 if pf1 >= pf2 else c1
        print(f"  {c1:<6} <-> {c2:<6} r={val:.3f} | Keep {keep} (PF={max(pf1,pf2):.2f}), consider dropping {drop} (PF={min(pf1,pf2):.2f})")

    # === LOW CORRELATIONS (good diversifiers) ===
    print(f"\n{'='*80}")
    print("BEST DIVERSIFIERS (lowest avg correlation)")
    print(f"{'='*80}\n")

    avg_corr = {}
    for c in COINS:
        others = [corr.loc[c, c2] for c2 in COINS if c2 != c]
        avg_corr[c] = np.mean(others)

    sorted_div = sorted(avg_corr.items(), key=lambda x: x[1])
    print(f"  {'Coin':<8} {'Avg Corr':<10} {'PF':<8} {'Diversification Value'}")
    print(f"  {'-'*50}")
    for coin, ac in sorted_div:
        pf = OPTIMAL_PF.get(coin, 0)
        # Diversification value = PF * (1 - avg_corr) -- higher is better
        div_value = pf * (1 - ac)
        print(f"  {coin:<8} {ac:<10.3f} {pf:<8.2f} {div_value:.2f}")

    # === CLUSTER ANALYSIS ===
    print(f"\n{'='*80}")
    print("COIN CLUSTERS (grouped by correlation >0.7)")
    print(f"{'='*80}\n")

    # Simple greedy clustering
    assigned = set()
    clusters = []
    for c1 in COINS:
        if c1 in assigned:
            continue
        cluster = [c1]
        assigned.add(c1)
        for c2 in COINS:
            if c2 in assigned:
                continue
            if corr.loc[c1, c2] > 0.7:
                cluster.append(c2)
                assigned.add(c2)
        clusters.append(cluster)

    for i, cluster in enumerate(clusters):
        pfs = [OPTIMAL_PF.get(c, 0) for c in cluster]
        best = cluster[pfs.index(max(pfs))]
        print(f"  Cluster {i+1}: {', '.join(cluster)}")
        print(f"    Best performer: {best} (PF={max(pfs):.2f})")
        if len(cluster) > 1:
            print(f"    Suggestion: prioritize {best}, others are partially redundant")
        print()

    # === OPTIMAL ALLOCATION ===
    print(f"{'='*80}")
    print("SUGGESTED ALLOCATION (risk-parity weighted by PF & diversification)")
    print(f"{'='*80}\n")

    # Score = PF * (1 - avg_correlation) -- rewards both performance and diversification
    scores = {}
    for coin in COINS:
        pf = OPTIMAL_PF.get(coin, 0)
        ac = avg_corr.get(coin, 0.5)
        scores[coin] = pf * (1 - ac)

    total_score = sum(scores.values())
    allocations = {c: s / total_score * 100 for c, s in scores.items()}

    sorted_alloc = sorted(allocations.items(), key=lambda x: -x[1])
    print(f"  {'Coin':<8} {'Weight':<10} {'PF':<8} {'Avg Corr':<10} {'Score'}")
    print(f"  {'-'*50}")
    for coin, weight in sorted_alloc:
        pf = OPTIMAL_PF.get(coin, 0)
        ac = avg_corr.get(coin, 0)
        print(f"  {coin:<8} {weight:<10.1f}% {pf:<8.2f} {ac:<10.3f} {scores[coin]:.3f}")

    # === RECOMMENDED PORTFOLIO ===
    print(f"\n{'='*80}")
    print("RECOMMENDED: TOP 10 DIVERSIFIED PORTFOLIO")
    print(f"{'='*80}\n")

    # Pick top 10 by score, avoiding highly correlated pairs
    selected = []
    for coin, _ in sorted(scores.items(), key=lambda x: -x[1]):
        # Check if too correlated with already selected
        too_corr = False
        for sel in selected:
            if corr.loc[coin, sel] > 0.8:
                too_corr = True
                break
        if not too_corr:
            selected.append(coin)
        if len(selected) == 10:
            break

    # Reweight selected
    sel_scores = {c: scores[c] for c in selected}
    sel_total = sum(sel_scores.values())
    sel_alloc = {c: s / sel_total * 100 for c, s in sel_scores.items()}

    print(f"  {'Coin':<8} {'Weight':<10} {'PF':<8} {'Avg Corr'}")
    print(f"  {'-'*40}")
    for coin in selected:
        pf = OPTIMAL_PF.get(coin, 0)
        ac = avg_corr.get(coin, 0)
        print(f"  {coin:<8} {sel_alloc[coin]:<10.1f}% {pf:<8.2f} {ac:.3f}")

    portfolio_avg_pf = np.mean([OPTIMAL_PF[c] for c in selected])
    # Average pairwise correlation within portfolio
    sel_corrs = []
    for i, c1 in enumerate(selected):
        for c2 in selected[i+1:]:
            sel_corrs.append(corr.loc[c1, c2])
    portfolio_avg_corr = np.mean(sel_corrs) if sel_corrs else 0

    print(f"\n  Portfolio avg PF: {portfolio_avg_pf:.2f}")
    print(f"  Portfolio avg correlation: {portfolio_avg_corr:.3f}")
    print(f"  Diversification benefit: {(1-portfolio_avg_corr)*100:.0f}%")

    # Save results
    output = {
        'correlation_matrix': corr.to_dict(),
        'high_corr_pairs': [(c1, c2, float(v)) for c1, c2, v in high_corr_pairs],
        'avg_correlations': {k: float(v) for k, v in avg_corr.items()},
        'clusters': clusters,
        'scores': {k: float(v) for k, v in scores.items()},
        'allocations': {k: float(v) for k, v in allocations.items()},
        'recommended_portfolio': selected,
    }
    with open('/home/scamarena/ProjectCoin/correlation_results.json', 'w') as f:
        json.dump(output, f, indent=2)

    print(f"\nResults saved to correlation_results.json")


if __name__ == "__main__":
    main()
