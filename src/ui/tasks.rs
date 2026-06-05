use chrono::Duration;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, DailyTask, TaskState};

use super::{
    base_style, duration::format_elapsed_seconds, emphasized_style, fg_style, task_block,
    MONOKAI_BLUE, MONOKAI_COMMENT, MONOKAI_FG, MONOKAI_GREEN, MONOKAI_ORANGE, MONOKAI_PINK,
    MONOKAI_SELECTION, MONOKAI_YELLOW,
};

const ON_HOLD_ONE_LINE_NOTE: &str =
    "保留中です。このタブは止めて、他タブのタスクを実施してください";

pub(super) fn draw_one_line(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(lines) = one_line_task_lines(app) {
        let task = Paragraph::new(lines)
            .style(base_style())
            .block(task_block());
        frame.render_widget(task, area);
    } else {
        let empty = Paragraph::new(app.empty_visible_tasks_message())
            .style(base_style())
            .block(task_block());
        frame.render_widget(empty, area);
    }
}

pub(super) fn one_line_task_lines(app: &App) -> Option<Vec<Line<'_>>> {
    let (_, task) = app.selected_visible_task()?;
    let mut lines = vec![task_line_for_app(task, app, false)];
    if task.state == TaskState::OnHold && !app.current_tab_is_all() {
        lines.push(Line::from(Span::styled(
            ON_HOLD_ONE_LINE_NOTE,
            fg_style(MONOKAI_ORANGE),
        )));
    }
    Some(lines)
}

pub(super) fn draw_incomplete_list(frame: &mut Frame, area: Rect, app: &App) {
    let visible_tasks = app.visible_tasks();
    let items: Vec<ListItem> = visible_tasks
        .iter()
        .map(|(_, task)| ListItem::new(task_line_for_app(task, app, false)))
        .collect();

    if items.is_empty() {
        let empty = Paragraph::new(app.empty_visible_tasks_message())
            .style(base_style())
            .block(task_block());
        frame.render_widget(empty, area);
    } else {
        let mut state = ListState::default();
        state.select(Some(app.selected_visible().min(items.len() - 1)));
        let list = List::new(items)
            .style(base_style())
            .block(task_block())
            .highlight_style(emphasized_style(MONOKAI_YELLOW).bg(MONOKAI_SELECTION))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut state);
    }
}

pub(super) fn draw_all_list(frame: &mut Frame, area: Rect, app: &App) {
    let lines = all_task_lines(app);
    let items: Vec<ListItem> = lines.into_iter().map(ListItem::new).collect();

    if items.is_empty() {
        let empty = Paragraph::new("タスクはありません")
            .style(base_style())
            .block(task_block());
        frame.render_widget(empty, area);
    } else {
        let mut state = ListState::default();
        state.select(app.selected_task_index());
        let list = List::new(items)
            .style(base_style())
            .block(task_block())
            .highlight_style(emphasized_style(MONOKAI_YELLOW).bg(MONOKAI_SELECTION))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut state);
    }
}

pub(super) fn all_task_lines(app: &App) -> Vec<Line<'_>> {
    app.current_tasks()
        .into_iter()
        .map(|task| task_line_for_app(task, app, true))
        .collect()
}

fn task_line_for_app<'a>(
    task: &'a DailyTask,
    app: &App,
    show_completed_duration: bool,
) -> Line<'a> {
    task_line(
        task,
        app.typical_task_duration_seconds(&task.name),
        show_completed_duration,
    )
}

pub(super) fn task_line(
    task: &DailyTask,
    typical_duration_seconds: Option<i64>,
    show_completed_duration: bool,
) -> Line<'_> {
    let mut spans = vec![
        Span::styled("見込み ", fg_style(MONOKAI_BLUE)),
        Span::styled(
            format!(
                "{:>8}  ",
                typical_duration_seconds
                    .map(format_elapsed_seconds)
                    .unwrap_or_else(|| "なし".to_string())
            ),
            emphasized_style(MONOKAI_GREEN).add_modifier(Modifier::SLOW_BLINK),
        ),
        Span::styled(&task.name, base_style()),
        Span::styled("  ", base_style()),
        Span::styled(
            task.state.label(),
            fg_style(state_color(task.state.label())),
        ),
    ];

    if let Some(started_at) = task.started_at {
        spans.push(Span::styled("  ", base_style()));
        spans.push(Span::styled(
            format!("開始 {}", started_at.format("%H:%M")),
            fg_style(MONOKAI_BLUE),
        ));
    }

    if show_completed_duration {
        if let Some(duration) = completed_work_duration(task) {
            spans.push(Span::styled("  ", base_style()));
            spans.push(Span::styled(
                format!("作業時間 {}", format_work_duration(duration)),
                fg_style(MONOKAI_GREEN),
            ));
        }
    }

    Line::from(spans)
}

fn completed_work_duration(task: &DailyTask) -> Option<Duration> {
    if task.state != TaskState::Done {
        return None;
    }

    let duration = task.completed_at? - task.started_at?;
    (duration >= Duration::zero()).then_some(duration)
}

fn format_work_duration(duration: Duration) -> String {
    format_elapsed_seconds(duration.num_seconds())
}

fn state_color(label: &str) -> Color {
    match label {
        "未着手" => MONOKAI_COMMENT,
        "実施中" => MONOKAI_GREEN,
        "保留" => MONOKAI_ORANGE,
        "後回し" => MONOKAI_YELLOW,
        "完了" => MONOKAI_BLUE,
        "時間切れ" => MONOKAI_PINK,
        _ => MONOKAI_FG,
    }
}
