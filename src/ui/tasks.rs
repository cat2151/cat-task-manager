use chrono::Duration;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    app::{App, DailyTask, TaskState},
    storage::UiConfig,
};

use super::{
    base_style, duration::format_elapsed_seconds, emphasized_style, fg_style, monokai_color,
    task_block, MONOKAI_BLUE, MONOKAI_COMMENT, MONOKAI_FG, MONOKAI_GREEN, MONOKAI_ORANGE,
    MONOKAI_PINK, MONOKAI_SELECTION, MONOKAI_YELLOW,
};

const ON_HOLD_ONE_LINE_NOTE: &str =
    "保留中です。このタブは止めて、他タブのタスクを実施してください";
const LIST_HIGHLIGHT_SYMBOL: &str = "> ";
const LIST_HIGHLIGHT_SYMBOL_WIDTH: u16 = 2;

pub(super) fn draw_one_line(frame: &mut Frame, area: Rect, app: &App, ui_config: &UiConfig) {
    if let Some(lines) = one_line_task_lines_with_config(app, ui_config) {
        let task = Paragraph::new(lines.clone())
            .style(base_style())
            .block(task_block());
        frame.render_widget(task, area);
        draw_one_line_overflow_labels(frame, area, &lines);
    } else {
        let empty = Paragraph::new(app.empty_visible_tasks_message())
            .style(base_style())
            .block(task_block());
        frame.render_widget(empty, area);
    }
}

#[cfg(test)]
pub(super) fn one_line_task_lines(app: &App) -> Option<Vec<Line<'_>>> {
    one_line_task_lines_with_config(app, &UiConfig::default())
}

pub(super) fn one_line_task_lines_with_config<'a>(
    app: &'a App,
    ui_config: &UiConfig,
) -> Option<Vec<Line<'a>>> {
    let (_, task) = app.selected_visible_task()?;
    let mut lines = vec![task_line_for_app(
        task,
        app,
        false,
        estimate_style_for_one_line(app, ui_config),
    )];
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
    let lines: Vec<Line<'_>> = visible_tasks
        .iter()
        .map(|(_, task)| task_line_for_app(task, app, false, default_estimate_style()))
        .collect();
    let items: Vec<ListItem> = lines.iter().cloned().map(ListItem::new).collect();

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
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL);
        frame.render_stateful_widget(list, area, &mut state);
        draw_list_overflow_labels(frame, area, &lines, state.offset(), state.selected());
    }
}

pub(super) fn draw_all_list(frame: &mut Frame, area: Rect, app: &App) {
    let lines = all_task_lines(app);
    let items: Vec<ListItem> = lines.iter().cloned().map(ListItem::new).collect();

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
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL);
        frame.render_stateful_widget(list, area, &mut state);
        draw_list_overflow_labels(frame, area, &lines, state.offset(), state.selected());
    }
}

pub(super) fn all_task_lines(app: &App) -> Vec<Line<'_>> {
    app.current_tasks()
        .into_iter()
        .map(|task| task_line_for_app(task, app, true, default_estimate_style()))
        .collect()
}

fn task_line_for_app<'a>(
    task: &'a DailyTask,
    app: &App,
    show_completed_duration: bool,
    estimate_style: Style,
) -> Line<'a> {
    task_line_with_style(
        task,
        app.free_time_display_seconds(task),
        app.free_time_active(),
        app.typical_task_duration_seconds(&task.name),
        show_completed_duration,
        estimate_style,
    )
}

#[cfg(test)]
pub(super) fn task_line(
    task: &DailyTask,
    typical_duration_seconds: Option<i64>,
    show_completed_duration: bool,
) -> Line<'_> {
    task_line_with_style(
        task,
        task.free_time_seconds,
        false,
        typical_duration_seconds,
        show_completed_duration,
        default_estimate_style(),
    )
}

fn task_line_with_style(
    task: &DailyTask,
    free_time_seconds: Option<u64>,
    free_time_active: bool,
    typical_duration_seconds: Option<i64>,
    show_completed_duration: bool,
    estimate_style: Style,
) -> Line<'_> {
    if let Some(seconds) = free_time_seconds {
        return free_time_task_line(task, seconds, free_time_active);
    }

    let mut spans = vec![
        Span::styled("見込み ", fg_style(MONOKAI_BLUE)),
        Span::styled(
            format!(
                "{:>8}  ",
                typical_duration_seconds
                    .map(format_elapsed_seconds)
                    .unwrap_or_else(|| "なし".to_string())
            ),
            estimate_style,
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

fn free_time_task_line(task: &DailyTask, seconds: u64, active: bool) -> Line<'_> {
    let mut spans = vec![
        Span::styled(&task.name, base_style()),
        Span::styled("  ", base_style()),
        Span::styled(
            format!("累積free time {}", format_elapsed_seconds(seconds as i64)),
            fg_style(MONOKAI_GREEN),
        ),
    ];

    if active {
        let label = TaskState::InProgress.label();
        spans.push(Span::styled("  ", base_style()));
        spans.push(Span::styled(label, fg_style(state_color(label))));
    }

    Line::from(spans)
}

fn draw_one_line_overflow_labels(frame: &mut Frame, area: Rect, lines: &[Line<'_>]) {
    let Some(content_area) = task_content_area(area) else {
        return;
    };

    for (index, line) in lines.iter().take(content_area.height as usize).enumerate() {
        let row_area = Rect::new(
            content_area.x,
            content_area.y + index as u16,
            content_area.width,
            1,
        );
        draw_in_progress_overflow_label(frame, row_area, line, 0);
    }
}

fn draw_list_overflow_labels(
    frame: &mut Frame,
    area: Rect,
    lines: &[Line<'_>],
    offset: usize,
    selected: Option<usize>,
) {
    let Some(content_area) = task_content_area(area) else {
        return;
    };

    for (index, line) in lines
        .iter()
        .enumerate()
        .skip(offset)
        .take(content_area.height as usize)
    {
        let row_area = Rect::new(
            content_area.x,
            content_area.y + (index - offset) as u16,
            content_area.width,
            1,
        );
        let content_offset = if selected == Some(index) {
            LIST_HIGHLIGHT_SYMBOL_WIDTH
        } else {
            0
        };
        draw_in_progress_overflow_label(frame, row_area, line, content_offset);
    }
}

fn draw_in_progress_overflow_label(
    frame: &mut Frame,
    row_area: Rect,
    line: &Line<'_>,
    content_offset: u16,
) {
    let Some((label_start, label_width)) = in_progress_label_bounds(line) else {
        return;
    };
    let rendered_label_end = content_offset as usize + label_start + label_width;

    if rendered_label_end <= row_area.width as usize || label_width > row_area.width as usize {
        return;
    }

    let label = TaskState::InProgress.label();
    let x = row_area
        .x
        .saturating_add(row_area.width.saturating_sub(label_width as u16));
    let clear_x = x.saturating_sub(1).max(row_area.x);
    let clear_width = row_area
        .x
        .saturating_add(row_area.width)
        .saturating_sub(clear_x);
    frame.buffer_mut().set_string(
        clear_x,
        row_area.y,
        " ".repeat(clear_width as usize),
        base_style(),
    );
    frame
        .buffer_mut()
        .set_string(x, row_area.y, label, fg_style(state_color(label)));
}

fn in_progress_label_bounds(line: &Line<'_>) -> Option<(usize, usize)> {
    let label = TaskState::InProgress.label();
    let mut start = 0;

    for span in &line.spans {
        if span.content.as_ref() == label {
            return Some((start, span.width()));
        }
        start += span.width();
    }

    None
}

fn task_content_area(area: Rect) -> Option<Rect> {
    if area.width <= 2 || area.height <= 2 {
        return None;
    }

    Some(Rect::new(
        area.x + 1,
        area.y + 1,
        area.width - 2,
        area.height - 2,
    ))
}

fn estimate_style_for_one_line(app: &App, ui_config: &UiConfig) -> Style {
    if !ui_config.estimate_blink.enabled || !app.estimate_blink_context() {
        return default_estimate_style();
    }

    let foreground = monokai_color(ui_config.estimate_blink.foreground);
    let background = monokai_color(ui_config.estimate_blink.background);
    let (foreground, background) = if app.estimate_blink_phase() {
        (background, foreground)
    } else {
        (foreground, background)
    };

    emphasized_style(foreground).bg(background)
}

fn default_estimate_style() -> Style {
    emphasized_style(MONOKAI_GREEN)
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
