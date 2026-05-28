use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{App, HistoryStatsState},
    event::{KeyAction, KeyBindings},
    history_stats::HistoryStatsReport,
    storage::APP_NAME,
};

use super::{
    base_style, emphasized_style, fg_style, spinner, MONOKAI_BLUE, MONOKAI_COMMENT, MONOKAI_GREEN,
    MONOKAI_ORANGE, MONOKAI_YELLOW,
};

pub(super) fn draw(frame: &mut Frame, app: &App, keybindings: &KeyBindings) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(APP_NAME, emphasized_style(MONOKAI_GREEN)),
        Span::styled("  過去データ統計", fg_style(MONOKAI_BLUE)),
    ]))
    .style(base_style())
    .block(themed_block("stats"));
    frame.render_widget(header, chunks[0]);

    match app.history_stats() {
        HistoryStatsState::Idle => draw_message(frame, chunks[1], "待機中"),
        HistoryStatsState::Loading => draw_loading(frame, chunks[1], app),
        HistoryStatsState::Ready(report) => draw_report(frame, chunks[1], report),
        HistoryStatsState::Error(err) => draw_message(frame, chunks[1], err),
    }

    draw_footer(frame, chunks[2], app, keybindings);
}

fn draw_loading(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let message = format!(
        "{} git履歴を集計中です。timeout 60秒",
        spinner(app.spinner_frame())
    );
    draw_message(frame, area, &message);
}

fn draw_report(frame: &mut Frame, area: ratatui::layout::Rect, report: &HistoryStatsReport) {
    let mut items = vec![
        ListItem::new(summary_line(report)),
        ListItem::new(Line::from("")),
    ];

    if report.task_counts.is_empty() {
        items.push(ListItem::new(Line::from("taskは見つかりませんでした")));
    } else {
        items.extend(report.task_counts.iter().enumerate().map(|(index, task)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:>2}. ", index + 1), fg_style(MONOKAI_COMMENT)),
                Span::styled(
                    format!("{:>4}回  ", task.count),
                    emphasized_style(MONOKAI_YELLOW),
                ),
                Span::styled(task.name.clone(), base_style()),
            ]))
        }));
    }

    let list = List::new(items)
        .style(base_style())
        .block(themed_block("回数 top 10"));
    frame.render_widget(list, area);
}

fn summary_line(report: &HistoryStatsReport) -> Line<'static> {
    let timeout = if report.timed_out { " timeout" } else { "" };
    Line::from(vec![
        Span::styled(
            format!("revisions {}  ", report.scanned_revisions),
            fg_style(MONOKAI_BLUE),
        ),
        Span::styled(
            format!("skipped {}{}", report.skipped_files, timeout),
            fg_style(MONOKAI_ORANGE),
        ),
    ])
}

fn draw_message(frame: &mut Frame, area: ratatui::layout::Rect, message: &str) {
    let paragraph = Paragraph::new(message.to_string())
        .style(base_style())
        .block(themed_block("回数 top 10"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_footer(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    app: &App,
    keybindings: &KeyBindings,
) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} : tasks", keybindings.label_for(KeyAction::Stats)),
            emphasized_style(MONOKAI_YELLOW),
        ),
        Span::styled("  ", base_style()),
        Span::styled("? : help", emphasized_style(MONOKAI_YELLOW)),
        Span::styled("  ", base_style()),
        Span::styled(app.message(), fg_style(MONOKAI_BLUE)),
    ]))
    .style(base_style())
    .block(themed_block("status"));
    frame.render_widget(footer, area);
}

fn themed_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(fg_style(MONOKAI_COMMENT))
        .style(base_style())
        .title(Line::from(Span::styled(
            title,
            emphasized_style(MONOKAI_GREEN),
        )))
}
