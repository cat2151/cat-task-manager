use chrono::Duration;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{App, DailyTask, TaskState, ViewMode},
    event::{KeyAction, KeyBindings},
    storage::APP_NAME,
};

const MONOKAI_BG: Color = Color::Rgb(39, 40, 34);
const MONOKAI_FG: Color = Color::Rgb(248, 248, 242);
const MONOKAI_COMMENT: Color = Color::Rgb(117, 113, 94);
const MONOKAI_SELECTION: Color = Color::Rgb(73, 72, 62);
const MONOKAI_PINK: Color = Color::Rgb(249, 38, 114);
const MONOKAI_GREEN: Color = Color::Rgb(166, 226, 46);
const MONOKAI_YELLOW: Color = Color::Rgb(230, 219, 116);
const MONOKAI_ORANGE: Color = Color::Rgb(253, 151, 31);
const MONOKAI_BLUE: Color = Color::Rgb(102, 217, 239);
const TAB_SEPARATOR: &str = " | ";
const TAB_SEPARATOR_WIDTH: u16 = 3;
const ON_HOLD_ONE_LINE_NOTE: &str =
    "保留中です。このタブは止めて、他タブのタスクを実施してください";

pub fn draw(frame: &mut Frame, app: &App, keybindings: &KeyBindings) {
    frame.render_widget(Block::default().style(base_style()), frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let header_spans = vec![
        Span::styled(app.current_date.to_string(), base_style()),
        Span::styled(
            format!("  表示: {}", app.view_mode().label()),
            fg_style(MONOKAI_BLUE),
        ),
    ];

    let header = Paragraph::new(Line::from(header_spans))
        .style(base_style())
        .block(themed_block(APP_NAME));
    frame.render_widget(header, chunks[0]);

    match app.view_mode() {
        ViewMode::OneLine => draw_one_line(frame, chunks[1], app),
        ViewMode::Incomplete => draw_incomplete_list(frame, chunks[1], app),
        ViewMode::All => draw_all_list(frame, chunks[1], app),
    }

    draw_tab_bar(frame, chunks[1], app);

    draw_footer(frame, chunks[2], app);

    if app.show_help() {
        draw_help(frame, frame.area(), keybindings);
    }

    if app.has_background_work() {
        draw_background_overlay(frame, frame.area(), app);
    }
}

fn draw_one_line(frame: &mut Frame, area: Rect, app: &App) {
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

fn one_line_task_lines(app: &App) -> Option<Vec<Line<'_>>> {
    let (_, task) = app.selected_visible_task()?;
    let mut lines = vec![task_line(task, false)];
    if task.state == TaskState::OnHold && !app.current_tab_is_all() {
        lines.push(Line::from(Span::styled(
            ON_HOLD_ONE_LINE_NOTE,
            fg_style(MONOKAI_ORANGE),
        )));
    }
    Some(lines)
}

fn draw_incomplete_list(frame: &mut Frame, area: Rect, app: &App) {
    let visible_tasks = app.visible_tasks();
    let items: Vec<ListItem> = visible_tasks
        .iter()
        .map(|(_, task)| ListItem::new(task_line(task, false)))
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
            .highlight_style(
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .bg(MONOKAI_SELECTION)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut state);
    }
}

fn draw_all_list(frame: &mut Frame, area: Rect, app: &App) {
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
            .highlight_style(
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .bg(MONOKAI_SELECTION)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut state);
    }
}

fn all_task_lines(app: &App) -> Vec<Line<'_>> {
    app.current_tasks()
        .into_iter()
        .map(|task| task_line(task, true))
        .collect()
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("? : help", emphasized_style(MONOKAI_YELLOW)),
        Span::styled("  ", base_style()),
        Span::styled(app.message(), fg_style(MONOKAI_BLUE)),
    ]))
    .style(base_style())
    .block(themed_block("status"));
    frame.render_widget(footer, area);
}

fn draw_help(frame: &mut Frame, area: Rect, keybindings: &KeyBindings) {
    let area = centered_rect(58, 14, area);
    let help = Paragraph::new(help_lines(keybindings))
        .style(base_style())
        .block(themed_block("help"));

    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}

fn draw_background_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let area = centered_rect(70, 7, area);
    let spinner = spinner(app.spinner_frame());
    let message = app.background_message().unwrap_or("background処理中です");
    let overlay = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("{spinner} background処理中"),
            emphasized_style(MONOKAI_YELLOW),
        )),
        Line::from(""),
        Line::from(Span::styled(message.to_string(), base_style())),
    ])
    .style(base_style())
    .block(themed_block("background"))
    .wrap(Wrap { trim: false });

    frame.render_widget(Clear, area);
    frame.render_widget(overlay, area);
}

fn help_lines(keybindings: &KeyBindings) -> Vec<Line<'static>> {
    vec![
        help_line(keybindings.label_for(KeyAction::Next), "次のタスクへ移動"),
        help_line(
            keybindings.label_for(KeyAction::Previous),
            "前のタスクへ移動",
        ),
        help_line(keybindings.label_for(KeyAction::Advance), "開始/完了"),
        help_line(
            keybindings.label_for(KeyAction::Hold),
            "保留（他タブへ）/再開",
        ),
        help_line(keybindings.label_for(KeyAction::Defer), "後回し"),
        help_line(keybindings.label_for(KeyAction::NextTab), "次のタブ"),
        help_line(keybindings.label_for(KeyAction::PreviousTab), "前のタブ"),
        help_line(keybindings.label_for(KeyAction::ToggleView), "表示切替"),
        help_line(keybindings.label_for(KeyAction::Edit), "現在のタブのmd編集"),
        help_line(keybindings.label_for(KeyAction::Quit), "終了"),
        help_line(keybindings.label_for(KeyAction::Help), "help 表示/閉じる"),
        help_line("esc", "help を閉じる"),
    ]
}

fn help_line(key: impl Into<String>, description: &str) -> Line<'static> {
    let key = key.into();
    Line::from(vec![
        Span::styled(format!("{key:<12}"), emphasized_style(MONOKAI_YELLOW)),
        Span::styled(description.to_string(), base_style()),
    ])
}

fn draw_tab_bar(frame: &mut Frame, area: Rect, app: &App) {
    if area.width < 3 || area.height == 0 {
        return;
    }

    let mut x = area.x.saturating_add(1);
    let area_right = area.x.saturating_add(area.width.saturating_sub(1));
    for index in 0..app.display_tab_count() {
        if x >= area_right {
            break;
        }

        if index > 0 {
            let remaining_width = area_right - x;
            if remaining_width <= TAB_SEPARATOR_WIDTH {
                break;
            }

            let separator_area = Rect::new(x, area.y, TAB_SEPARATOR_WIDTH, 1);
            let separator =
                Paragraph::new(TAB_SEPARATOR).style(fg_style(MONOKAI_COMMENT).bg(MONOKAI_BG));
            frame.render_widget(separator, separator_area);
            x = x.saturating_add(TAB_SEPARATOR_WIDTH);
        }

        let remaining_width = area_right - x;
        let label = app.display_tab_label(index).unwrap_or_default();
        let width = tab_width(label).min(remaining_width);
        if width == 0 {
            break;
        }
        let selected = index == app.selected_tab();
        let label_style = if selected {
            emphasized_style(MONOKAI_BG).bg(MONOKAI_YELLOW)
        } else {
            fg_style(MONOKAI_COMMENT).bg(MONOKAI_BG)
        };
        let label = clipped_tab_label(label, width);
        let tab_area = Rect::new(x, area.y, width, 1);
        let tab_widget =
            Paragraph::new(Line::from(Span::styled(label, label_style))).style(label_style);

        frame.render_widget(tab_widget, tab_area);
        x = x.saturating_add(width);
    }
}

fn tab_width(label: &str) -> u16 {
    label.chars().count() as u16
}

fn clipped_tab_label(label: &str, width: u16) -> String {
    let max_len = width as usize;
    if label.chars().count() <= max_len {
        return label.to_string();
    }

    label
        .chars()
        .take(max_len.saturating_sub(1))
        .chain("~".chars())
        .collect()
}

fn task_line(task: &DailyTask, show_completed_duration: bool) -> Line<'_> {
    let mut spans = vec![
        Span::styled(format!("{:>2}. ", task.order), fg_style(MONOKAI_COMMENT)),
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
    let total_seconds = duration.num_seconds();
    if total_seconds < 60 {
        return format!("{total_seconds}秒");
    }

    let total_minutes = total_seconds / 60;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;

    match (hours, minutes) {
        (0, minutes) => format!("{minutes}分"),
        (hours, 0) => format!("{hours}時間"),
        (hours, minutes) => format!("{hours}時間{minutes}分"),
    }
}

fn spinner(frame: usize) -> &'static str {
    const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
    FRAMES[frame % FRAMES.len()]
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

fn task_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(fg_style(MONOKAI_COMMENT))
        .style(base_style())
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn base_style() -> Style {
    Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG)
}

fn fg_style(color: Color) -> Style {
    base_style().fg(color)
}

fn emphasized_style(color: Color) -> Style {
    fg_style(color).add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests;
