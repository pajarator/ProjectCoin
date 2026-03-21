use crate::config::{self, COINS, INITIAL_CAPITAL};
use crate::coordinator::{MarketMode, Regime};
use crate::state::{SharedState, TradeType};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(frame: &mut Frame, state: &SharedState) {
    let area = frame.area();

    // Split into header, table, summary, log
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Length(1),  // breadth
            Constraint::Length(1),  // column headers
            Constraint::Length(COINS.len() as u16), // coin rows
            Constraint::Length(1),  // separator
            Constraint::Length(2),  // summary
            Constraint::Min(5),    // log
        ])
        .split(area);

    // === HEADER ===
    let total_pnl = state.total_balance() - (COINS.len() as f64 * INITIAL_CAPITAL);
    let scalp_info = format!("ScalpSL:{:.1}% TP:{:.1}%",
        config::SCALP_SL * 100.0, config::SCALP_TP * 100.0);
    let now = chrono::Local::now().format("%H:%M:%S");
    let header = Line::from(vec![
        Span::styled(
            format!(" COINCLAW v11 | "),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:+.2}$", total_pnl),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" | Risk:{:.0}% | {:.0}x | SL:{:.1}% | {} | {}",
                config::RISK * 100.0, config::LEVERAGE, config::STOP_LOSS * 100.0,
                scalp_info, now),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ]);
    let header_widget = Paragraph::new(header);
    frame.render_widget(header_widget, chunks[0]);

    // === BREADTH ===
    let (breadth_str, breadth_color) = match state.market_mode {
        MarketMode::Long => (
            format!(" BREADTH: {:.0}% ({}/{}) - MODE: LONG+ISO (longs + ISO shorts)",
                state.breadth * 100.0, state.bearish_count, state.total_count),
            Color::Green,
        ),
        MarketMode::Short => (
            format!(" BREADTH: {:.0}% ({}/{}) - MODE: SHORT (market-dump shorts)",
                state.breadth * 100.0, state.bearish_count, state.total_count),
            Color::Magenta,
        ),
        MarketMode::IsoShort => (
            format!(" BREADTH: {:.0}% ({}/{}) - MODE: ISO_SHORT (coin-specific shorts)",
                state.breadth * 100.0, state.bearish_count, state.total_count),
            Color::Yellow,
        ),
    };
    let breadth_widget = Paragraph::new(breadth_str)
        .style(Style::default().fg(breadth_color));
    frame.render_widget(breadth_widget, chunks[1]);

    // === COLUMN HEADERS ===
    let col_header = Line::from(vec![
        Span::styled(format!("{:>2} ", "#"), Style::default()),
        Span::styled(format!("{:<5} ", "COIN"), Style::default().fg(Color::Magenta)),
        Span::styled(format!("{:<7} ", "REGIME"), Style::default()),
        Span::styled(format!("{:<12} ", "STRAT"), Style::default()),
        Span::styled(format!("{:>10} ", "PRICE"), Style::default()),
        Span::styled(format!("{:>3} ", "RSI"), Style::default()),
        Span::styled(format!("{:>6} ", "Z-SCORE"), Style::default()),
        Span::styled(format!("{:>5} ", "VOL"), Style::default()),
        Span::styled(format!("{:<6} ", "POS"), Style::default()),
        Span::styled(format!("{:>7} ", "P&L"), Style::default()),
        Span::styled(format!("{:>7} ", "BAL"), Style::default()),
        Span::styled("W", Style::default()),
    ]);
    frame.render_widget(Paragraph::new(col_header), chunks[2]);

    // === COIN ROWS ===
    let mut lines = Vec::new();
    for (idx, cs) in state.coins.iter().enumerate() {
        let ind = cs.ind_15m.as_ref();

        let price = ind.map(|i| i.p).unwrap_or(0.0);
        let rsi = ind.map(|i| i.rsi).unwrap_or(0.0);
        let z = ind.map(|i| i.z).unwrap_or(0.0);
        let vol_r = ind.map(|i| {
            if i.vol_ma > 0.0 { i.vol / i.vol_ma } else { 0.0 }
        }).unwrap_or(0.0);

        let regime_str = cs.regime.to_string();
        let regime_color = match cs.regime {
            Regime::Squeeze => Color::Red,
            Regime::HighVol => Color::Yellow,
            Regime::StrongTrend => Color::Green,
            Regime::WeakTrend => Color::Magenta,
            Regime::Ranging => Color::White,
        };

        let strat_str = cs.active_strat.as_deref().unwrap_or("---");

        let price_str = if price < 0.01 {
            format!("${:>10.6}", price)
        } else if price < 1.0 {
            format!("${:>10.4}", price)
        } else {
            format!("${:>10.2}", price)
        };

        let rsi_color = if rsi < 30.0 { Color::Green }
            else if rsi > 70.0 { Color::Red }
            else { Color::White };

        let z_color = if z < -1.5 { Color::Green }
            else if z < -1.0 { Color::Yellow }
            else if z > 1.5 { Color::Red }
            else { Color::White };

        let vol_color = if vol_r > 1.2 { Color::Green }
            else if vol_r > 0.8 { Color::Yellow }
            else { Color::Red };

        let (pos_str, pnl_str, pos_color) = if let Some(ref pos) = cs.pos {
            let pnl = if pos.dir == "long" {
                (price - pos.e) / pos.e * config::LEVERAGE * 100.0
            } else {
                (pos.e - price) / pos.e * config::LEVERAGE * 100.0
            };
            let is_scalp = pos.trade_type == Some(TradeType::Scalp);
            let label = if pos.dir == "long" {
                if is_scalp { "SCALP+" } else { "LONG" }
            } else {
                if is_scalp { "SCALP-" } else { "SHORT" }
            };
            let color = if pos.dir == "long" {
                if pnl >= 0.0 { Color::Green } else { Color::Red }
            } else {
                if pnl >= 0.0 { Color::Magenta } else { Color::Red }
            };
            (label.to_string(), format!("{:+.1}%", pnl), color)
        } else {
            ("CASH".to_string(), "-".to_string(), Color::White)
        };

        let eff_bal = cs.effective_bal();
        let bal_color = if eff_bal >= INITIAL_CAPITAL { Color::Green } else { Color::Red };

        let win_str = if cs.trades.is_empty() {
            "-".to_string()
        } else {
            format!("{}/{}", cs.win_count(), cs.trades.len())
        };

        let line = Line::from(vec![
            Span::styled(format!("{:>2} ", idx + 1), Style::default()),
            Span::styled(format!("{:<5} ", cs.name), Style::default().fg(Color::Magenta)),
            Span::styled(format!("{:<7} ", regime_str), Style::default().fg(regime_color)),
            Span::styled(format!("{:<12} ", strat_str), Style::default()),
            Span::styled(format!("{} ", price_str), Style::default()),
            Span::styled(format!("{:>3.0} ", rsi), Style::default().fg(rsi_color)),
            Span::styled(format!("{:>+6.2} ", z), Style::default().fg(z_color)),
            Span::styled(format!("{:>4.1}x ", vol_r), Style::default().fg(vol_color)),
            Span::styled(format!("{:<6} ", pos_str), Style::default().fg(pos_color)),
            Span::styled(format!("{:>7} ", pnl_str), Style::default()),
            Span::styled(format!("${:>6.0} ", eff_bal), Style::default().fg(bal_color)),
            Span::styled(win_str, Style::default()),
        ]);
        lines.push(line);
    }
    let table = Paragraph::new(lines);
    frame.render_widget(table, chunks[3]);

    // === SEPARATOR ===
    let sep = Paragraph::new("─".repeat(area.width as usize))
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(sep, chunks[4]);

    // === SUMMARY ===
    let total = state.total_balance();
    let total_pnl = total - (COINS.len() as f64 * INITIAL_CAPITAL);
    let trades = state.total_trades();
    let wins = state.total_wins();
    let wr_str = if trades > 0 {
        format!("{}/{} ({:.0}%)", wins, trades, 100.0 * wins as f64 / trades as f64)
    } else { "-".to_string() };

    let pnl_color = if total_pnl >= 0.0 { Color::Green } else { Color::Red };
    let summary = Line::from(vec![
        Span::styled(
            format!(" TOTAL: ${:.0} (", total),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:+.0}", total_pnl),
            Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(") | {} trades | W: {} | 'q' quit", trades, wr_str),
            Style::default(),
        ),
    ]);
    frame.render_widget(Paragraph::new(summary), chunks[5]);

    // === LOG ===
    let log_area = chunks[6];
    let log_height = log_area.height as usize;
    let entries: Vec<&String> = state.log_entries.iter().collect();
    let start = if entries.len() > log_height { entries.len() - log_height } else { 0 };

    let mut log_lines = Vec::new();
    log_lines.push(Line::from(Span::styled(
        "═".repeat(area.width as usize),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));

    for entry in &entries[start..] {
        let color = if entry.contains("BUY") || entry.contains("SHORT [") {
            Color::Green
        } else if entry.contains("SELL") || entry.contains("COVER") {
            if entry.contains("+") { Color::Green } else { Color::Red }
        } else {
            Color::White
        };
        log_lines.push(Line::from(Span::styled(
            entry.as_str(),
            Style::default().fg(color),
        )));
    }
    let log_widget = Paragraph::new(log_lines);
    frame.render_widget(log_widget, log_area);
}
