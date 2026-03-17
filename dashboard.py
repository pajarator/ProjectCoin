"""
Results dashboard — generates HTML reports with charts.
Uses matplotlib for static charts embedded as base64 in HTML.
"""
import json
import os
import base64
import io
import numpy as np
import pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt


def fig_to_base64(fig) -> str:
    """Convert matplotlib figure to base64 string for embedding in HTML."""
    buf = io.BytesIO()
    fig.savefig(buf, format='png', dpi=100, bbox_inches='tight')
    buf.seek(0)
    b64 = base64.b64encode(buf.read()).decode('utf-8')
    plt.close(fig)
    return b64


def plot_equity_curve(equity_curve: list, title: str = 'Equity Curve') -> str:
    """Plot equity curve, return base64 image."""
    fig, ax = plt.subplots(figsize=(10, 4))
    ax.plot(equity_curve, color='#2196F3')
    ax.fill_between(range(len(equity_curve)), equity_curve, alpha=0.1, color='#2196F3')
    ax.set_title(title)
    ax.set_xlabel('Trade #')
    ax.set_ylabel('Equity ($)')
    ax.grid(True, alpha=0.3)
    return fig_to_base64(fig)


def plot_drawdown(equity_curve: list, title: str = 'Drawdown') -> str:
    """Plot drawdown chart, return base64 image."""
    equity = np.array(equity_curve)
    peak = np.maximum.accumulate(equity)
    dd = (peak - equity) / peak * 100

    fig, ax = plt.subplots(figsize=(10, 3))
    ax.fill_between(range(len(dd)), dd, color='#F44336', alpha=0.5)
    ax.set_title(title)
    ax.set_xlabel('Trade #')
    ax.set_ylabel('Drawdown (%)')
    ax.invert_yaxis()
    ax.grid(True, alpha=0.3)
    return fig_to_base64(fig)


def plot_mc_fan(percentile_curves: dict, title: str = 'Monte Carlo Fan Chart') -> str:
    """Plot Monte Carlo percentile fan chart."""
    fig, ax = plt.subplots(figsize=(10, 5))
    colors = {'p5': '#F44336', 'p25': '#FF9800', 'p50': '#4CAF50', 'p75': '#2196F3', 'p95': '#9C27B0'}

    for key, curve in percentile_curves.items():
        ax.plot(curve, label=key, color=colors.get(key, '#666'))

    if 'p5' in percentile_curves and 'p95' in percentile_curves:
        ax.fill_between(range(len(percentile_curves['p5'])),
                        percentile_curves['p5'], percentile_curves['p95'],
                        alpha=0.1, color='#2196F3')

    ax.set_title(title)
    ax.set_xlabel('Day')
    ax.set_ylabel('Portfolio Value ($)')
    ax.legend()
    ax.grid(True, alpha=0.3)
    return fig_to_base64(fig)


def plot_feature_importance(features: dict, title: str = 'Feature Importance', top_n: int = 20) -> str:
    """Plot horizontal bar chart of feature importance."""
    items = sorted(features.items(), key=lambda x: x[1], reverse=True)[:top_n]
    names = [x[0] for x in items][::-1]
    values = [x[1] for x in items][::-1]

    fig, ax = plt.subplots(figsize=(8, max(4, top_n * 0.3)))
    ax.barh(names, values, color='#2196F3')
    ax.set_title(title)
    ax.set_xlabel('Importance')
    ax.grid(True, alpha=0.3, axis='x')
    return fig_to_base64(fig)


def plot_correlation_heatmap(corr_matrix: pd.DataFrame, title: str = 'Correlation Matrix') -> str:
    """Plot correlation heatmap."""
    fig, ax = plt.subplots(figsize=(10, 8))
    im = ax.imshow(corr_matrix.values, cmap='RdBu_r', vmin=-1, vmax=1)
    ax.set_xticks(range(len(corr_matrix)))
    ax.set_yticks(range(len(corr_matrix)))
    ax.set_xticklabels(corr_matrix.columns, rotation=45, ha='right', fontsize=8)
    ax.set_yticklabels(corr_matrix.index, fontsize=8)
    plt.colorbar(im, ax=ax)
    ax.set_title(title)
    return fig_to_base64(fig)


def generate_html_report(sections: list, title: str = 'ProjectCoin Results Dashboard') -> str:
    """
    Generate HTML report from sections.

    Args:
        sections: list of dicts with 'title', 'content' (HTML string), 'images' (list of base64)

    Returns:
        HTML string
    """
    html = f"""<!DOCTYPE html>
<html>
<head>
<title>{title}</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
       max-width: 1200px; margin: 0 auto; padding: 20px; background: #f5f5f5; }}
h1 {{ color: #1a237e; border-bottom: 3px solid #1a237e; padding-bottom: 10px; }}
h2 {{ color: #283593; margin-top: 30px; }}
.section {{ background: white; border-radius: 8px; padding: 20px; margin: 20px 0;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
table {{ border-collapse: collapse; width: 100%; margin: 10px 0; }}
th, td {{ border: 1px solid #ddd; padding: 8px; text-align: right; }}
th {{ background: #e8eaf6; color: #1a237e; }}
tr:nth-child(even) {{ background: #f5f5f5; }}
td:first-child {{ text-align: left; font-weight: bold; }}
img {{ max-width: 100%; height: auto; margin: 10px 0; }}
.metric {{ display: inline-block; background: #e8eaf6; padding: 10px 20px;
           border-radius: 4px; margin: 5px; }}
.metric .value {{ font-size: 24px; font-weight: bold; color: #1a237e; }}
.metric .label {{ font-size: 12px; color: #666; }}
.good {{ color: #4CAF50; }}
.bad {{ color: #F44336; }}
</style>
</head>
<body>
<h1>{title}</h1>
<p>Generated: {pd.Timestamp.now().strftime('%Y-%m-%d %H:%M')}</p>
"""

    for section in sections:
        html += f'<div class="section"><h2>{section["title"]}</h2>'
        html += section.get('content', '')
        for img_b64 in section.get('images', []):
            html += f'<img src="data:image/png;base64,{img_b64}" />'
        html += '</div>'

    html += '</body></html>'
    return html


def build_dashboard(output_path: str = 'dashboard_report.html'):
    """
    Build dashboard from available results files.
    Looks for run*_results.json and generates charts.
    """
    sections = []

    # RUN16 Feature Importance
    if os.path.exists('run16_1_results.json'):
        with open('run16_1_results.json') as f:
            data = json.load(f)

        summary = data.get('summary', {})
        content = '<div>'
        content += f'<div class="metric"><div class="value">{summary.get("avg_rf_accuracy", 0):.3f}</div><div class="label">RF Accuracy</div></div>'
        content += f'<div class="metric"><div class="value">{summary.get("avg_xgb_accuracy", 0):.3f}</div><div class="label">XGB Accuracy</div></div>'
        content += f'<div class="metric"><div class="value">{summary.get("n_universal_features", 0)}</div><div class="label">Universal Features</div></div>'
        content += '</div>'

        images = []
        universal = data.get('universal_features', {})
        if universal:
            imp_dict = {k: v['avg_importance'] for k, v in universal.items()}
            images.append(plot_feature_importance(imp_dict, 'Universal Feature Importance'))

        sections.append({'title': 'RUN16: ML Feature Importance', 'content': content, 'images': images})

    # RUN17 Monte Carlo
    if os.path.exists('run17_1_results.json'):
        with open('run17_1_results.json') as f:
            data = json.load(f)

        summary = data.get('summary', {})
        content = '<div>'
        content += f'<div class="metric"><div class="value">{summary.get("valid_combos", 0)}</div><div class="label">Valid Combos</div></div>'
        content += f'<div class="metric"><div class="value">{summary.get("flagged_count", 0)}</div><div class="label">Flagged (PF CI < 1.0)</div></div>'
        content += f'<div class="metric"><div class="value">{summary.get("avg_prob_profit", 0):.1%}</div><div class="label">Avg Prob Profit</div></div>'
        content += '</div>'

        if summary.get('flagged_strategies'):
            content += '<h3>Flagged Strategies</h3><table><tr><th>Coin</th><th>Strategy</th><th>Actual PF</th><th>95% CI Lower PF</th></tr>'
            for f_item in summary['flagged_strategies'][:20]:
                content += f'<tr><td>{f_item["coin"]}</td><td>{f_item["strategy"]}</td>'
                content += f'<td>{f_item["actual_pf"]:.2f}</td><td class="bad">{f_item["pf_lower"]:.2f}</td></tr>'
            content += '</table>'

        sections.append({'title': 'RUN17: Monte Carlo Validation', 'content': content, 'images': []})

    # RUN19 Portfolio Risk
    if os.path.exists('run19_2_results.json'):
        with open('run19_2_results.json') as f:
            data = json.load(f)

        content = '<div>'
        content += f'<div class="metric"><div class="value">{data.get("portfolio_var_95", 0):.4f}</div><div class="label">VaR 95%</div></div>'
        content += f'<div class="metric"><div class="value">{data.get("portfolio_sharpe", 0):.3f}</div><div class="label">Portfolio Sharpe</div></div>'
        content += '</div>'

        images = []
        corr = data.get('correlation_matrix')
        if corr:
            corr_df = pd.DataFrame(corr)
            images.append(plot_correlation_heatmap(corr_df, 'Coin Correlation Matrix'))

        sections.append({'title': 'RUN19: Portfolio Risk', 'content': content, 'images': images})

    if not sections:
        sections.append({
            'title': 'No Results Yet',
            'content': '<p>Run the RUN scripts to generate results. Dashboard will auto-populate.</p>',
            'images': []
        })

    html = generate_html_report(sections)

    with open(output_path, 'w') as f:
        f.write(html)
    print(f'Dashboard saved to {output_path}')


if __name__ == '__main__':
    build_dashboard()
