use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::app::{
    App, AppMode, Group, InputTarget, Priority, RowRef, SubTask, TodoItem, TodoStatus,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let outer_block = Block::default()
        .title(" tumeto ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    match app.mode {
        AppMode::Normal => render_normal(frame, app, inner_area),
        AppMode::Input => render_input(frame, app, inner_area),
        AppMode::Search => render_search(frame, app, inner_area),
        AppMode::Help => {
            render_normal(frame, app, inner_area);
            render_help_overlay(frame, area);
        }
        AppMode::EditingNotes => render_editing_notes(frame, app, inner_area),
        AppMode::CategoryPopup => {
            render_normal(frame, app, inner_area);
            render_category_popup(frame, app, area);
        }
        AppMode::GroupInput => {
            render_normal(frame, app, inner_area);
            render_category_popup(frame, app, area);
            render_group_input(frame, app, area);
        }
        AppMode::GroupDeleteConfirm => {
            render_normal(frame, app, inner_area);
            render_category_popup(frame, app, area);
            render_group_delete_confirm(frame, app, area);
        }
        AppMode::Zen => render_zen(frame, app, inner_area),
    }
}

const BIG_DIGITS: [[&str; 5]; 10] = [
    ["█████", "█   █", "█   █", "█   █", "█████"],
    ["  █  ", " ██  ", "  █  ", "  █  ", "█████"],
    ["█████", "    █", "█████", "█    ", "█████"],
    ["█████", "    █", "█████", "    █", "█████"],
    ["█   █", "█   █", "█████", "    █", "    █"],
    ["█████", "█    ", "█████", "    █", "█████"],
    ["█████", "█    ", "█████", "█   █", "█████"],
    ["█████", "    █", "   █ ", "  █  ", "  █  "],
    ["█████", "█   █", "█████", "█   █", "█████"],
    ["█████", "█   █", "█████", "    █", "█████"],
];

const BIG_COLON: [&str; 5] = ["     ", "  █  ", "     ", "  █  ", "     "];

fn big_clock_rows(time: &str) -> Vec<String> {
    let mut rows = vec![String::new(); 5];

    for ch in time.chars() {
        let glyph = match ch {
            ':' => &BIG_COLON,
            digit if digit.is_ascii_digit() => &BIG_DIGITS[digit as usize - '0' as usize],
            _ => continue,
        };

        for (row, segment) in rows.iter_mut().zip(glyph.iter()) {
            if !row.is_empty() {
                row.push_str("  ");
            }
            row.push_str(segment);
        }
    }

    rows
}

fn render_zen(frame: &mut Frame, app: &App, area: Rect) {
    let Some((group, todo)) = app.active_timer_todo() else {
        let message = Paragraph::new("활성 타이머가 없습니다. [ Esc ]로 돌아가세요.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(message, area);
        return;
    };
    let Some(timer) = app.timer.as_ref() else {
        return;
    };

    let content = centered_fixed(90, 10, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(content);

    let clock_style = if timer.running {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };
    let clock_lines: Vec<Line> = big_clock_rows(&timer.display_remaining())
        .into_iter()
        .map(|row| Line::from(Span::styled(row, clock_style)))
        .collect();
    frame.render_widget(
        Paragraph::new(clock_lines).alignment(Alignment::Center),
        rows[0],
    );

    let state = if timer.running { "RUNNING" } else { "PAUSED" };
    frame.render_widget(
        Paragraph::new(format!("{state}  ·  POMO {}", todo.pomodoros))
            .style(if timer.running {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            })
            .alignment(Alignment::Center),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new(format!("[>] {} / {}", group.name, todo.title))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center),
        rows[3],
    );
    frame.render_widget(
        Paragraph::new("[ Space ] 일시정지/재개   [ c ] 완료   [ T ] 취소")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center),
        rows[4],
    );
    frame.render_widget(
        Paragraph::new("[ Esc ] Zen Mode 종료 · 타이머는 계속 실행")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        rows[5],
    );
}

fn render_normal(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_tab_bar(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_footer(frame, chunks[2], FooterMode::Normal);
}

fn render_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    if app.groups.is_empty() {
        return;
    }

    let selected = app.selected_group;
    let separator = " │ ";
    let sep_w = separator.width() as u16;

    let tab_widths: Vec<u16> = app
        .groups
        .iter()
        .map(|g| g.name.width() as u16 + 2)
        .collect();

    let total: u16 =
        tab_widths.iter().sum::<u16>() + sep_w * app.groups.len().saturating_sub(1) as u16;

    let (start, end) = if total <= area.width {
        (0, app.groups.len())
    } else {
        tab_window(&tab_widths, selected, sep_w, area.width.saturating_sub(2))
    };

    let active = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let inactive = Style::default().fg(Color::White);
    let arrow = Style::default().fg(Color::DarkGray);

    let mut spans: Vec<Span> = Vec::new();

    spans.push(if start > 0 {
        Span::styled("◀", arrow)
    } else {
        Span::raw(" ")
    });

    for i in start..end {
        if i > start {
            spans.push(Span::raw(separator));
        }
        let label = format!(" {} ", app.groups[i].name);
        let style = if i == selected { active } else { inactive };
        spans.push(Span::styled(label, style));
    }

    spans.push(if end < app.groups.len() {
        Span::styled("▶", arrow)
    } else {
        Span::raw(" ")
    });

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn tab_window(tab_widths: &[u16], selected: usize, sep_w: u16, max_width: u16) -> (usize, usize) {
    let mut start = selected;
    let mut end = selected + 1;
    let mut width = tab_widths[selected];

    loop {
        let can_left = start > 0;
        let can_right = end < tab_widths.len();

        let left_cost = if can_left {
            tab_widths[start - 1].saturating_add(sep_w)
        } else {
            u16::MAX
        };
        let right_cost = if can_right {
            tab_widths[end].saturating_add(sep_w)
        } else {
            u16::MAX
        };

        if can_right && width.saturating_add(right_cost) <= max_width {
            width = width.saturating_add(right_cost);
            end += 1;
        } else if can_left && width.saturating_add(left_cost) <= max_width {
            width = width.saturating_add(left_cost);
            start -= 1;
        } else {
            break;
        }
    }

    (start, end)
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_list_pane(frame, app, panes[0]);

    // 우측 40%: 상단 대시보드(고정 5줄) + 하단 메모(나머지)
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(panes[1]);

    render_dashboard(frame, app, right[0]);
    render_notes_pane(frame, app, right[1]);
}

fn render_list_pane(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.search_query.is_empty() {
        " 할 일 ".to_string()
    } else {
        format!(" 할 일 (검색: {}) ", app.search_query)
    };

    let panel = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));

    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    render_list(frame, app, inner);
}

fn render_notes_pane(frame: &mut Frame, app: &App, area: Rect) {
    let is_editing = app.mode == AppMode::EditingNotes;
    let is_subtask = matches!(app.current_row(), Some(RowRef::Sub(_, _)));

    let (title, border_color) = match (is_editing, is_subtask) {
        (true, true) => (" 하위 할 일 메모 (편집 중) ", Color::Yellow),
        (true, false) => (" 메모 (편집 중) ", Color::Yellow),
        (false, true) => (" 하위 할 일 메모 ", Color::DarkGray),
        (false, false) => (" 메모 ", Color::DarkGray),
    };

    let panel = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    render_notes_content(frame, app, inner);
}

fn render_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let panel = Block::default()
        .title(" 대시보드 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = panel.inner(area);
    frame.render_widget(panel, area);

    let focus = dashboard_focus(app);
    let dashboard_group = focus
        .map(|(group, _)| group)
        .or_else(|| app.groups.get(app.selected_group));
    let (ratio, pomodoros, done, total) = dashboard_group.map_or((0.0, 0, 0, 0), |group| {
        (
            group.completion_ratio(),
            group.total_pomodoros(),
            group
                .todos
                .iter()
                .filter(|todo| todo.status.is_done())
                .count(),
            group.todos.len(),
        )
    });

    // 대시보드 내부를 좌(토마토 + 타이머) / 우(InProgress + 진척)로 분할한다.
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let timer_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(9), Constraint::Min(0)])
        .split(halves[0]);
    render_tomato(frame, timer_columns[0]);

    let (remaining, timer_state, state_style) = match app.timer.as_ref() {
        Some(timer) if timer.running => (
            timer.display_remaining(),
            "RUNNING",
            Style::default().fg(Color::Green),
        ),
        Some(timer) => (
            timer.display_remaining(),
            "PAUSED",
            Style::default().fg(Color::Yellow),
        ),
        None => (
            "25:00".to_string(),
            "READY",
            Style::default().fg(Color::DarkGray),
        ),
    };

    let timer_info = Paragraph::new(vec![
        Line::from(Span::styled(
            remaining,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(timer_state, state_style)),
        Line::from(Span::styled(
            format!("POMO {pomodoros}"),
            Style::default().fg(Color::DarkGray),
        )),
    ]);
    frame.render_widget(timer_info, timer_columns[1]);

    let focus_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(Color::DarkGray));
    let focus_inner = focus_block.inner(halves[1]);
    frame.render_widget(focus_block, halves[1]);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(1)])
        .split(focus_inner);

    let focus_title = focus
        .map(|(_, todo)| format!("[>] {}", todo.title))
        .unwrap_or_else(|| "대기 중".to_string());
    let focus_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " IN PROGRESS ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{done}/{total}"),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(Span::styled(
            format!(" {focus_title}"),
            if focus.is_some() {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        )),
    ]);
    frame.render_widget(focus_info, rows[0]);

    render_progress_gauge(frame, rows[1], ratio);
}

fn dashboard_focus(app: &App) -> Option<(&Group, &TodoItem)> {
    app.active_timer_todo().or_else(|| {
        let group = app.groups.get(app.selected_group)?;
        let todo = group
            .todos
            .iter()
            .find(|todo| todo.status == TodoStatus::InProgress)?;
        Some((group, todo))
    })
}

/// 진척 막대.
///
/// 문자 글리프의 픽셀 두께에 의존하지 않도록 공백 문자의 배경색으로
/// 채움(Green)과 비움(DarkGray)을 표현한다. 퍼센트 라벨도 자신이 놓인
/// 막대 셀의 배경색을 그대로 유지한다.
fn render_progress_gauge(frame: &mut Frame, area: Rect, ratio: f64) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let ratio = ratio.clamp(0.0, 1.0);
    let filled_end = area.x + (f64::from(area.width) * ratio).round() as u16;

    let bar_background = |x: u16| {
        if x < filled_end {
            Color::Green
        } else {
            Color::DarkGray
        }
    };

    let buf = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.reset();
            cell.set_symbol(" ").set_bg(bar_background(x));
        }
    }

    let percent = (ratio * 100.0).round() as u16;
    let label = format!("{percent}%");
    let label_width = (label.chars().count() as u16).min(area.width);
    let label_col = area.x + (area.width - label_width) / 2;
    let label_row = area.y + area.height / 2;

    for (i, ch) in label.chars().enumerate() {
        let x = label_col + i as u16;
        if x >= area.right() {
            break;
        }

        let background = bar_background(x);
        let foreground = if x < filled_end {
            Color::Black
        } else {
            Color::White
        };

        let cell = &mut buf[(x, label_row)];
        cell.reset();
        cell.set_char(ch).set_style(
            Style::default()
                .fg(foreground)
                .bg(background)
                .add_modifier(Modifier::BOLD),
        );
    }
}

fn render_tomato(frame: &mut Frame, area: Rect) {
    let red = Style::default().fg(Color::Red);
    let green = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(vec![
            Span::styled(" ▄▄", red),
            Span::styled("v", green),
            Span::styled("▄▄", red),
        ]),
        Line::from(Span::styled("███████", red)),
        Line::from(Span::styled(" ▀▀▀▀▀", red)),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_notes_content(frame: &mut Frame, app: &App, area: Rect) {
    let is_editing = app.mode == AppMode::EditingNotes;

    let content = if is_editing {
        app.notes_buffer.as_str()
    } else {
        app.current_notes().unwrap_or("")
    };

    if content.is_empty() && !is_editing {
        let placeholder = Paragraph::new("[ m ] 키로 메모를 작성하세요.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(placeholder, area);
        return;
    }

    let style = if is_editing {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let safe_width = area.width.max(1);
    let wrapped_lines = wrap_text(content, safe_width);

    let lines_for_widget: Vec<Line> = wrapped_lines
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), style)))
        .collect();

    frame.render_widget(Paragraph::new(lines_for_widget), area);

    if is_editing {
        let last_line = wrapped_lines.last().cloned().unwrap_or_default();
        let cursor_x = area.x + last_line.width() as u16;
        let cursor_y = area.y + wrapped_lines.len().saturating_sub(1) as u16;

        frame.set_cursor_position((
            cursor_x.min(area.right().saturating_sub(1)),
            cursor_y.min(area.bottom().saturating_sub(1)),
        ));
    }
}

fn wrap_text(text: &str, panel_width: u16) -> Vec<String> {
    let mut lines = Vec::new();

    for logical_line in text.split('\n') {
        let mut current_line = String::new();
        let mut current_width = 0;

        for c in logical_line.chars() {
            let char_width = c.width().unwrap_or(0) as u16;

            if current_width + char_width > panel_width {
                lines.push(current_line);
                current_line = String::new();
                current_width = 0;
            }
            current_line.push(c);
            current_width += char_width;
        }
        lines.push(current_line);
    }
    lines
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_tab_bar(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_input_box(frame, app, chunks[2]);
    render_footer(frame, chunks[3], FooterMode::Input);
}

fn render_editing_notes(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_tab_bar(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_footer(frame, chunks[2], FooterMode::EditingNotes);
}

fn render_list(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.visible_rows();

    if rows.is_empty() {
        let message = if app.search_query.is_empty() {
            "아직 할 일이 없습니다. [ a ] 를 눌러 추가하세요."
        } else {
            "검색 결과가 없습니다."
        };
        let empty_msg = Paragraph::new(message)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(empty_msg, area);
        return;
    }

    let todos = app.current_todos();
    let items: Vec<ListItem> = rows
        .iter()
        .map(|&row| match row {
            RowRef::Todo(i) => build_todo_item(&todos[i]),
            RowRef::Sub(i, j) => build_subtask_item(&todos[i].subtasks[j]),
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn pomodoro_span(count: u8) -> Option<Span<'static>> {
    if count == 0 {
        return None;
    }
    const MAX_DOTS: u8 = 6;
    let marks = if count <= MAX_DOTS {
        "*".repeat(count as usize)
    } else {
        format!("*x{count}")
    };
    Some(Span::styled(
        format!("  {marks}"),
        Style::default().fg(Color::Red),
    ))
}

fn build_todo_item(todo: &TodoItem) -> ListItem<'_> {
    let fold = if todo.subtasks.is_empty() {
        Span::raw("  ")
    } else if todo.collapsed {
        Span::styled("▸ ", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled("▾ ", Style::default().fg(Color::DarkGray))
    };

    // 완료: 취소선 + DarkGray (우선순위 색보다 완료 표시가 우선)
    if todo.status.is_done() {
        let mut spans = vec![
            fold,
            Span::styled("[x] ", Style::default().fg(Color::Green)),
            Span::styled(
                todo.title.clone(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT),
            ),
        ];
        spans.extend(pomodoro_span(todo.pomodoros));
        return ListItem::new(Line::from(spans));
    }

    let in_progress = matches!(todo.status, TodoStatus::InProgress);

    let checkbox = if in_progress {
        Span::styled(
            "[>] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("[ ] ", Style::default().fg(Color::White))
    };

    // 진행 중이면 포커스 유도를 위해 제목을 강조, 아니면 우선순위 색
    let title_style = if in_progress {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        let color = match todo.priority {
            Priority::High => Color::Red,
            Priority::Medium => Color::Yellow,
            Priority::Low => Color::White,
        };
        Style::default().fg(color)
    };

    let mut spans = vec![
        fold,
        checkbox,
        Span::styled(todo.title.clone(), title_style),
    ];

    if !todo.subtasks.is_empty() {
        let done = todo.subtasks.iter().filter(|s| s.status.is_done()).count();
        spans.push(Span::styled(
            format!(" ({}/{})", done, todo.subtasks.len()),
            Style::default().fg(Color::DarkGray),
        ));
    }

    let badge: Option<Span> = match todo.priority {
        Priority::High => Some(Span::styled(
            "  !!",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Priority::Medium => Some(Span::styled("  ! ", Style::default().fg(Color::Yellow))),
        Priority::Low => None,
    };
    spans.extend(badge);
    spans.extend(pomodoro_span(todo.pomodoros));
    ListItem::new(Line::from(spans))
}

fn build_subtask_item(sub: &SubTask) -> ListItem<'_> {
    let (checkbox, title_style) = match sub.status {
        TodoStatus::Done => (
            Span::styled("[x] ", Style::default().fg(Color::Green)),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::CROSSED_OUT),
        ),
        TodoStatus::InProgress => (
            Span::styled(
                "[>] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Style::default().fg(Color::Cyan),
        ),
        TodoStatus::Todo => (
            Span::styled("[ ] ", Style::default().fg(Color::White)),
            Style::default().fg(Color::Gray),
        ),
    };

    ListItem::new(Line::from(vec![
        Span::styled("    └ ", Style::default().fg(Color::DarkGray)),
        checkbox,
        Span::styled(sub.title.clone(), title_style),
    ]))
}

fn render_input_box(frame: &mut Frame, app: &App, area: Rect) {
    let (title, color) = match app.input_target {
        InputTarget::NewTodo => (" 새 할 일 입력 ", Color::Yellow),
        InputTarget::NewSubtask(_) => (" 하위 할 일 입력 ", Color::Yellow),
        InputTarget::EditTodo(_) | InputTarget::EditSubtask(_, _) => (" 항목 편집 ", Color::Green),
    };

    let input_box = Paragraph::new(app.input_buffer.as_str())
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color)),
        )
        .style(Style::default().fg(color));

    frame.render_widget(input_box, area);

    let cursor_x =
        (area.x + 1 + app.input_buffer.width() as u16).min(area.right().saturating_sub(2));
    let cursor_y = area.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_search(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_search_results(frame, app, chunks[0]);
    render_search_box(frame, app, chunks[1]);
    render_footer(frame, chunks[2], FooterMode::Search);
}

fn render_search_results(frame: &mut Frame, app: &App, area: Rect) {
    let hits = app.global_search_results();
    let title = if app.search_query.trim().is_empty() {
        format!(" 전체 항목 · {}개 ", hits.len())
    } else {
        format!(" 전체 그룹 검색 · {}개 결과 ", hits.len())
    };
    let panel = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);

    if hits.is_empty() {
        frame.render_widget(
            Paragraph::new("검색 결과가 없습니다.")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = hits
        .iter()
        .filter_map(|hit| {
            let group = app.groups.get(hit.group)?;
            let group_path = Span::styled(
                format!("{}  ›  ", group.name),
                Style::default().fg(Color::DarkGray),
            );
            match hit.row {
                RowRef::Todo(todo_index) => {
                    let todo = group.todos.get(todo_index)?;
                    Some(ListItem::new(Line::from(vec![
                        group_path,
                        search_status_span(todo.status),
                        Span::raw(todo.title.clone()),
                    ])))
                }
                RowRef::Sub(todo_index, subtask_index) => {
                    let todo = group.todos.get(todo_index)?;
                    let subtask = todo.subtasks.get(subtask_index)?;
                    Some(ListItem::new(Line::from(vec![
                        group_path,
                        Span::styled(
                            format!("{}  ›  ", todo.title),
                            Style::default().fg(Color::Gray),
                        ),
                        search_status_span(subtask.status),
                        Span::raw(subtask.title.clone()),
                    ])))
                }
            }
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    );
    let mut state = ListState::default();
    state.select(Some(app.search_selected));
    frame.render_stateful_widget(list, inner, &mut state);
}

fn search_status_span(status: TodoStatus) -> Span<'static> {
    match status {
        TodoStatus::Todo => Span::styled("[ ] ", Style::default().fg(Color::White)),
        TodoStatus::InProgress => Span::styled("[>] ", Style::default().fg(Color::Cyan)),
        TodoStatus::Done => Span::styled("[x] ", Style::default().fg(Color::Green)),
    }
}

fn render_search_box(frame: &mut Frame, app: &App, area: Rect) {
    let search_box = Paragraph::new(app.search_query.as_str())
        .block(
            Block::default()
                .title(" 검색 ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .style(Style::default().fg(Color::Magenta));

    frame.render_widget(search_box, area);

    let cursor_x =
        (area.x + 1 + app.search_query.width() as u16).min(area.right().saturating_sub(2));
    frame.set_cursor_position((cursor_x, area.y + 1));
}

fn render_footer(frame: &mut Frame, area: Rect, mode: FooterMode) {
    let text = match mode {
        FooterMode::Normal => {
            "[ Tab/h/l ] 그룹  [ j/k ] 항목  [ Enter ] Zen  [ t/T ] 타이머  [ a/s ] 추가  [ e/d ] 편집  [ / ] 검색  [ ? ] 도움말  [ q ] 종료"
        }
        FooterMode::Input => "[ Enter ] 저장  [ Esc ] 취소",
        FooterMode::EditingNotes => {
            "[ Ctrl+S ] 메모 저장  [ Esc ] 취소  [ Enter ] 줄 바꿈  [ Backspace ] 삭제"
        }
        FooterMode::Search => {
            "[ ↑/↓ ] 결과 이동  [ Enter ] 항목으로 이동  [ Esc ] 취소  [ 문자 ] Fuzzy 검색"
        }
    };

    let footer = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(56, 80, area);

    frame.render_widget(Clear, popup_area);

    let key = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let desc = Style::default().fg(Color::White);
    let head = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(Span::styled(" Normal 모드", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j / ↓      ", key),
            Span::styled("아래로 이동", desc),
        ]),
        Line::from(vec![
            Span::styled("  k / ↑      ", key),
            Span::styled("위로 이동", desc),
        ]),
        Line::from(vec![
            Span::styled("  Space      ", key),
            Span::styled("상태 순환 [ ] → [>] → [x] (상위 순환 시 하위까지)", desc),
        ]),
        Line::from(vec![
            Span::styled("  a / i      ", key),
            Span::styled("새 항목 추가", desc),
        ]),
        Line::from(vec![
            Span::styled("  s          ", key),
            Span::styled("하위 할 일 추가", desc),
        ]),
        Line::from(vec![
            Span::styled("  z          ", key),
            Span::styled("하위 할 일 접기 / 펼치기", desc),
        ]),
        Line::from(vec![
            Span::styled("  e          ", key),
            Span::styled("항목 편집", desc),
        ]),
        Line::from(vec![
            Span::styled("  d / x      ", key),
            Span::styled("항목 삭제", desc),
        ]),
        Line::from(vec![
            Span::styled("  u          ", key),
            Span::styled("삭제 되돌리기 (Undo)", desc),
        ]),
        Line::from(vec![
            Span::styled("  Tab / l    ", key),
            Span::styled("다음 그룹", desc),
        ]),
        Line::from(vec![
            Span::styled("  S-Tab / h  ", key),
            Span::styled("이전 그룹", desc),
        ]),
        Line::from(vec![
            Span::styled("  c          ", key),
            Span::styled("카테고리 점프 팝업", desc),
        ]),
        Line::from(vec![
            Span::styled("  m          ", key),
            Span::styled("메모 편집", desc),
        ]),
        Line::from(vec![
            Span::styled("  p          ", key),
            Span::styled("우선순위 순환 (Low → Medium → High)", desc),
        ]),
        Line::from(vec![
            Span::styled("  t          ", key),
            Span::styled("타이머 시작 / 일시정지 / 재개 (상위 전용)", desc),
        ]),
        Line::from(vec![
            Span::styled("  T          ", key),
            Span::styled("현재 타이머 취소", desc),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", key),
            Span::styled("선택 작업으로 Zen Mode 진입", desc),
        ]),
        Line::from(vec![
            Span::styled("  /          ", key),
            Span::styled("제목 검색 / 필터", desc),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", key),
            Span::styled("도움말 닫기", desc),
        ]),
        Line::from(vec![
            Span::styled("  q          ", key),
            Span::styled("종료", desc),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C     ", key),
            Span::styled("즉시 종료", desc),
        ]),
        Line::from(""),
        Line::from(Span::styled(" 입력 / 편집 모드", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter      ", key),
            Span::styled("저장", desc),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", key),
            Span::styled("취소", desc),
        ]),
        Line::from(vec![
            Span::styled("  Backspace  ", key),
            Span::styled("마지막 문자 삭제", desc),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(" 우선순위 색상", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Red        ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("High", desc),
        ]),
        Line::from(vec![
            Span::styled(
                "  Yellow     ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Medium", desc),
        ]),
        Line::from(vec![
            Span::styled("  White      ", Style::default().fg(Color::White)),
            Span::styled("Low (기본값)", desc),
        ]),
        Line::from(""),
        Line::from(Span::styled(" 검색 모드", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  문자 입력  ", key),
            Span::styled("실시간 필터 (대소문자 무시)", desc),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", key),
            Span::styled("검색 확정 (필터 유지)", desc),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", key),
            Span::styled("검색 해제 (전체 표시)", desc),
        ]),
        Line::from(""),
        Line::from(Span::styled(" 카테고리 팝업", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑ / k      ", key),
            Span::styled("커서 위로", desc),
        ]),
        Line::from(vec![
            Span::styled("  ↓ / j      ", key),
            Span::styled("커서 아래로", desc),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", key),
            Span::styled("해당 그룹으로 이동", desc),
        ]),
        Line::from(vec![
            Span::styled("  n          ", key),
            Span::styled("새 그룹 추가", desc),
        ]),
        Line::from(vec![
            Span::styled("  r          ", key),
            Span::styled("커서 그룹 이름 변경", desc),
        ]),
        Line::from(vec![
            Span::styled("  d          ", key),
            Span::styled("커서 그룹 삭제 (확인)", desc),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", key),
            Span::styled("취소", desc),
        ]),
        Line::from(""),
        Line::from(Span::styled(" 메모 편집 모드", head)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Ctrl+S     ", key),
            Span::styled("메모 저장", desc),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", key),
            Span::styled("편집 취소", desc),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", key),
            Span::styled("줄 바꿈 삽입", desc),
        ]),
        Line::from(vec![
            Span::styled("  Backspace  ", key),
            Span::styled("마지막 문자 삭제", desc),
        ]),
        Line::from(""),
        Line::from(Span::styled("  아무 키나 눌러 닫기", dim)),
    ];

    let popup = Paragraph::new(lines).block(
        Block::default()
            .title(" 도움말 [ ? ] ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(popup, popup_area);
}

fn render_category_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(44, 60, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 카테고리 [ c ] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = app
        .groups
        .iter()
        .enumerate()
        .map(|(i, group)| {
            let marker = if i == app.selected_group {
                "● "
            } else {
                "  "
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(Color::Green)),
                Span::styled(group.name.clone(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    state.select(Some(app.category_cursor));

    frame.render_stateful_widget(list, chunks[0], &mut state);

    let hint =
        Paragraph::new("[ n ] 추가  [ r ] 이름변경  [ d ] 삭제  [ Enter ] 이동  [ Esc ] 닫기")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[1]);
}

fn render_group_input(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_fixed(40, 3, area);
    frame.render_widget(Clear, popup);

    let title = if app.group_editing.is_some() {
        " 그룹 이름 변경 "
    } else {
        " 새 그룹 이름 "
    };

    let input = Paragraph::new(app.input_buffer.as_str())
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .style(Style::default().fg(Color::Green));

    frame.render_widget(input, popup);

    let cursor_x =
        (popup.x + 1 + app.input_buffer.width() as u16).min(popup.right().saturating_sub(2));
    frame.set_cursor_position((cursor_x, popup.y + 1));
}

fn render_group_delete_confirm(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_fixed(52, 6, area);
    frame.render_widget(Clear, popup);

    let (name, count) = app
        .groups
        .get(app.category_cursor)
        .map(|g| (g.name.as_str(), g.todos.len()))
        .unwrap_or(("", 0));

    let lines = vec![
        Line::from(Span::styled(
            format!("'{name}' 그룹을 삭제할까요?"),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            format!("포함된 할 일 {count}개가 함께 삭제됩니다."),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[ y ] 삭제   ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("[ n ] 취소", Style::default().fg(Color::White)),
        ]),
    ];

    let confirm = Paragraph::new(lines).block(
        Block::default()
            .title(" 그룹 삭제 확인 ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );

    frame.render_widget(confirm, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

enum FooterMode {
    Normal,
    Input,
    EditingNotes,
    Search,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn progress_gauge_uses_spaces_and_cell_backgrounds() {
        let backend = TestBackend::new(10, 1);
        let mut terminal = Terminal::new(backend).expect("test terminal should be created");

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_progress_gauge(frame, area, 0.5);
            })
            .expect("progress gauge should render");

        let buffer = terminal.backend().buffer();

        // 폭 10, 비율 50%이므로 x=0..4는 Green, x=5..9는 DarkGray다.
        for x in 0..10 {
            let expected_background = if x < 5 { Color::Green } else { Color::DarkGray };
            assert_eq!(buffer[(x, 0)].bg, expected_background);
        }

        // "50%"는 폭 10의 중앙인 x=3..5에 놓인다.
        let label = (3..=5)
            .map(|x| buffer[(x, 0)].symbol())
            .collect::<Vec<_>>()
            .concat();
        assert_eq!(label, "50%");

        // 라벨이 아닌 막대 셀은 글리프가 아니라 공백이어야 한다.
        for x in [0, 1, 2, 6, 7, 8, 9] {
            assert_eq!(buffer[(x, 0)].symbol(), " ");
        }

        // 라벨은 배경에 따라 읽기 쉬운 전경색을 사용한다.
        assert_eq!(buffer[(3, 0)].fg, Color::Black);
        assert_eq!(buffer[(4, 0)].fg, Color::Black);
        assert_eq!(buffer[(5, 0)].fg, Color::White);
    }

    #[test]
    fn big_clock_has_five_equally_sized_rows() {
        let rows = big_clock_rows("25:00");

        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|row| row.width() == 33));
        assert!(rows.iter().any(|row| row.contains('█')));
    }
}
