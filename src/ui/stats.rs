use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{App, HistoryStatsState},
    event::{KeyAction, KeyBindings},
    history_stats::{HistoryStatsReport, TaskNameCount},
    storage::APP_NAME,
};

use super::{
    base_style, emphasized_style, fg_style, spinner, MONOKAI_BLUE, MONOKAI_COMMENT, MONOKAI_GREEN,
    MONOKAI_ORANGE, MONOKAI_SELECTION, MONOKAI_YELLOW,
};

const MAX_STATS_DURATION_MINUTES: i64 = 999;

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
        HistoryStatsState::Ready(report) => draw_report(frame, chunks[1], app, report),
        HistoryStatsState::Error(err) => draw_message(frame, chunks[1], err),
    }

    draw_footer(frame, chunks[2], app, keybindings);
}

fn draw_loading(frame: &mut Frame, area: Rect, app: &App) {
    let message = format!(
        "{} git履歴を集計中です。timeout 60秒",
        spinner(app.spinner_frame())
    );
    draw_message(frame, area, &message);
}

fn draw_report(frame: &mut Frame, area: Rect, app: &App, report: &HistoryStatsReport) {
    let block = themed_block("回数");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    render_line(frame, inner, 0, summary_line(report));
    render_line(frame, inner, 1, typical_task_duration_line(report));

    let task_area = offset_area(inner, 3);
    if report.task_counts.is_empty() {
        frame.render_widget(
            Paragraph::new("taskは見つかりませんでした").style(base_style()),
            task_area,
        );
    } else {
        let items = report
            .task_counts
            .iter()
            .enumerate()
            .map(|(index, task)| ListItem::new(task_count_line(index, task)))
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(app.selected_history_stats_task());
        let list = List::new(items)
            .style(base_style())
            .highlight_style(emphasized_style(MONOKAI_YELLOW).bg(MONOKAI_SELECTION))
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, task_area, &mut state);
    }
}

fn task_count_line(index: usize, task: &TaskNameCount) -> Line<'static> {
    let duration = task
        .typical_task_duration
        .as_ref()
        .map(|duration| format_stats_duration_minutes(duration.elapsed_seconds))
        .unwrap_or_else(|| "なし".to_string());
    Line::from(vec![
        Span::styled(format!("{:>2}. ", index + 1), fg_style(MONOKAI_COMMENT)),
        Span::styled(
            format!("{:>4}回  ", task.count),
            emphasized_style(MONOKAI_YELLOW),
        ),
        Span::styled("目安 ", fg_style(MONOKAI_BLUE)),
        Span::styled(format!("{duration:>6}  "), emphasized_style(MONOKAI_GREEN)),
        Span::styled(task.name.clone(), base_style()),
    ])
}

fn render_line(frame: &mut Frame, area: Rect, line_index: u16, line: Line<'static>) {
    if line_index >= area.height {
        return;
    }
    let area = Rect::new(area.x, area.y + line_index, area.width, 1);
    frame.render_widget(Paragraph::new(line).style(base_style()), area);
}

fn offset_area(area: Rect, y_offset: u16) -> Rect {
    let y_offset = y_offset.min(area.height);
    Rect::new(
        area.x,
        area.y + y_offset,
        area.width,
        area.height.saturating_sub(y_offset),
    )
}

fn typical_task_duration_line(report: &HistoryStatsReport) -> Line<'static> {
    let Some(duration) = &report.typical_task_duration else {
        return Line::from(Span::styled(
            "目安の所要時間 なし",
            fg_style(MONOKAI_COMMENT),
        ));
    };

    Line::from(vec![
        Span::styled("目安の所要時間 ", fg_style(MONOKAI_BLUE)),
        Span::styled(
            format_stats_duration_minutes(duration.elapsed_seconds),
            emphasized_style(MONOKAI_GREEN),
        ),
    ])
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

fn format_stats_duration_minutes(total_seconds: i64) -> String {
    let minutes = (total_seconds / 60).clamp(0, MAX_STATS_DURATION_MINUTES);
    format!("{minutes}min")
}

fn draw_message(frame: &mut Frame, area: Rect, message: &str) {
    let paragraph = Paragraph::new(message.to_string())
        .style(base_style())
        .block(themed_block("回数"))
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
        Span::styled("j/k : scroll", emphasized_style(MONOKAI_YELLOW)),
        Span::styled("  ", base_style()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_stats::TypicalTaskDuration;

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn typical_task_duration_line_shows_duration() {
        let report = HistoryStatsReport {
            scanned_revisions: 1,
            skipped_files: 0,
            timed_out: false,
            typical_task_duration: Some(TypicalTaskDuration {
                elapsed_seconds: 30 * 60,
            }),
            task_counts: Vec::new(),
        };

        let text = line_text(&typical_task_duration_line(&report));

        assert_eq!(text, "目安の所要時間 30min");
    }

    #[test]
    fn task_count_line_shows_typical_duration() {
        let task = TaskNameCount {
            name: "朝食をいただく".to_string(),
            count: 3,
            typical_task_duration: Some(TypicalTaskDuration {
                elapsed_seconds: 30 * 60,
            }),
        };

        let text = line_text(&task_count_line(0, &task));

        assert_eq!(text, " 1.    3回  目安  30min  朝食をいただく");
    }

    #[test]
    fn stats_duration_minutes_are_clamped_and_truncated() {
        assert_eq!(format_stats_duration_minutes(-1), "0min");
        assert_eq!(format_stats_duration_minutes(59), "0min");
        assert_eq!(format_stats_duration_minutes(60), "1min");
        assert_eq!(format_stats_duration_minutes(1000 * 60), "999min");
    }
}
