use std::{io, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Priority {
    #[default]
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn cycle(&self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Low,
        }
    }

    fn display_sort_rank(&self) -> u8 {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Low => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TodoStatus {
    #[default]
    Todo,
    InProgress,
    Done,
}

impl TodoStatus {
    /// Todo → InProgress → Done → Todo 순환.
    pub fn cycle(self) -> Self {
        match self {
            Self::Todo => Self::InProgress,
            Self::InProgress => Self::Done,
            Self::Done => Self::Todo,
        }
    }

    pub fn is_done(self) -> bool {
        matches!(self, Self::Done)
    }

    fn display_sort_rank(self) -> u8 {
        match self {
            Self::InProgress => 0,
            Self::Todo => 1,
            Self::Done => 2,
        }
    }
}

fn de_status_compat<'de, D>(deserializer: D) -> Result<TodoStatus, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Compat {
        Status(TodoStatus), // 신 포맷: "Todo" / "InProgress" / "Done"
        Legacy(bool),       // 구 포맷: true / false
    }

    Ok(match Compat::deserialize(deserializer)? {
        Compat::Status(s) => s,
        Compat::Legacy(true) => TodoStatus::Done,
        Compat::Legacy(false) => TodoStatus::Todo,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub title: String,

    #[serde(default, alias = "completed", deserialize_with = "de_status_compat")]
    pub status: TodoStatus,

    #[serde(default)]
    pub notes: String,
}

impl SubTask {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            status: TodoStatus::default(),
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub title: String,

    #[serde(default, alias = "completed", deserialize_with = "de_status_compat")]
    pub status: TodoStatus,

    #[serde(default)]
    pub notes: String,

    #[serde(default)]
    pub priority: Priority,

    #[serde(default)]
    pub subtasks: Vec<SubTask>,

    #[serde(default)]
    pub collapsed: bool,

    #[serde(default)]
    pub pomodoros: u8,
}

impl TodoItem {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            status: TodoStatus::default(),
            notes: String::new(),
            priority: Priority::default(),
            subtasks: Vec::new(),
            collapsed: false,
            pomodoros: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,

    #[serde(default)]
    pub todos: Vec<TodoItem>,
}

impl Group {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            todos: Vec::new(),
        }
    }

    /// 상위 할 일 중 완료(Done) 비율. 0.0 ~ 1.0. 빈 그룹은 0.0.
    pub fn completion_ratio(&self) -> f64 {
        if self.todos.is_empty() {
            return 0.0;
        }
        let done = self.todos.iter().filter(|t| t.status.is_done()).count();
        done as f64 / self.todos.len() as f64
    }

    /// 그룹 내 모든 상위 할 일의 누적 뽀모도로 합계.
    pub fn total_pomodoros(&self) -> u32 {
        self.todos.iter().map(|t| t.pomodoros as u32).sum()
    }
}

pub const POMODORO_DURATION: Duration = Duration::from_secs(25 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerTarget {
    pub group: usize,
    pub todo: usize,
}

#[derive(Debug, Clone)]
pub struct PomodoroTimer {
    pub target: TimerTarget,
    pub remaining: Duration,
    pub running: bool,
}

impl PomodoroTimer {
    fn new(target: TimerTarget) -> Self {
        Self {
            target,
            remaining: POMODORO_DURATION,
            running: true,
        }
    }

    pub fn display_remaining(&self) -> String {
        let seconds = self.remaining.as_secs() + u64::from(self.remaining.subsec_nanos() > 0);
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowRef {
    Todo(usize),
    Sub(usize, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchHit {
    pub group: usize,
    pub row: RowRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputTarget {
    #[default]
    NewTodo,
    NewSubtask(usize),
    EditTodo(usize),
    EditSubtask(usize, usize),
}

#[derive(Debug, Clone)]
pub enum LastDeleted {
    Todo {
        group: usize,
        index: usize,
        item: TodoItem,
    },
    Sub {
        group: usize,
        todo: usize,
        index: usize,
        item: SubTask,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Normal,
    Input,
    Help,
    EditingNotes,
    Search,
    CategoryPopup,
    GroupInput,
    GroupDeleteConfirm,
    Zen,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct App {
    pub groups: Vec<Group>,

    #[serde(default)]
    pub selected_group: usize,

    #[serde(default)]
    pub selected: usize,

    #[serde(skip)]
    pub mode: AppMode,

    #[serde(skip)]
    pub input_buffer: String,

    #[serde(skip)]
    pub should_quit: bool,

    #[serde(skip)]
    pub dirty: bool,

    #[serde(skip)]
    pub input_target: InputTarget,

    #[serde(skip)]
    pub last_deleted: Option<LastDeleted>,

    #[serde(skip)]
    pub notes_buffer: String,

    #[serde(skip)]
    pub search_query: String,

    #[serde(skip)]
    pub search_selected: usize,

    #[serde(skip)]
    pub category_cursor: usize,

    #[serde(skip)]
    pub group_editing: Option<usize>,

    #[serde(skip)]
    pub timer: Option<PomodoroTimer>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            groups: vec![Group::new("기본")],
            selected_group: 0,
            selected: 0,
            mode: AppMode::Normal,
            input_buffer: String::new(),
            should_quit: false,
            dirty: false,
            input_target: InputTarget::NewTodo,
            last_deleted: None,
            notes_buffer: String::new(),
            search_query: String::new(),
            search_selected: 0,
            category_cursor: 0,
            group_editing: None,
            timer: None,
        }
    }
}

#[derive(Deserialize)]
struct LegacyApp {
    #[serde(default)]
    todos: Vec<TodoItem>,
    #[serde(default)]
    selected: usize,
}

fn fuzzy_score(candidate: &str, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let candidate: Vec<char> = candidate.to_lowercase().chars().collect();
    let query: Vec<char> = query.to_lowercase().chars().collect();
    let mut score = 0i64;
    let mut from = 0usize;
    let mut previous = None;

    for needle in &query {
        let offset = candidate.get(from..)?.iter().position(|c| c == needle)?;
        let index = from + offset;
        score += 10;
        if previous.is_some_and(|previous| previous + 1 == index) {
            score += 15;
        }
        if previous.is_none() {
            score -= index as i64;
        }
        previous = Some(index);
        from = index + 1;
    }

    score -= candidate.len().saturating_sub(query.len()) as i64;
    Some(score)
}

impl App {
    pub fn load() -> Self {
        let content = std::fs::read_to_string(Self::data_file_path())
            .or_else(|_| std::fs::read_to_string(Self::legacy_data_file_path()));

        let Ok(content) = content else {
            return Self::default();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            return Self::default();
        };

        let app = if value.get("groups").is_some() {
            serde_json::from_value(value).unwrap_or_default()
        } else {
            serde_json::from_value::<LegacyApp>(value)
                .map(Self::from_legacy)
                .unwrap_or_default()
        };

        app.normalize_loaded_state()
    }

    fn from_legacy(legacy: LegacyApp) -> Self {
        Self {
            groups: vec![Group {
                name: "기본".to_string(),
                todos: legacy.todos,
            }],
            selected_group: 0,
            selected: legacy.selected,
            ..Self::default()
        }
    }

    fn normalize_loaded_state(mut self) -> Self {
        let original_group = self.selected_group;
        let original_selected = self.selected;
        let mut repaired = false;

        if self.groups.is_empty() {
            self.groups.push(Group::new("기본"));
            repaired = true;
        }

        self.selected_group = self.selected_group.min(self.groups.len() - 1);
        self.clamp_selection();

        self.dirty =
            repaired || self.selected_group != original_group || self.selected != original_selected;
        self
    }

    pub fn save(&self) -> io::Result<()> {
        let content = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        std::fs::write(Self::data_file_path(), content)
    }

    fn data_file_path() -> PathBuf {
        Self::home_dir().join(".tumeto_data.json")
    }

    fn legacy_data_file_path() -> PathBuf {
        Self::home_dir().join(".tudo_data.json")
    }

    fn home_dir() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
    }

    pub fn cycle_status(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let mut completed_todo = None;
        {
            let Some(group) = self.groups.get_mut(self.selected_group) else {
                return;
            };
            match row {
                RowRef::Todo(i) => {
                    let next = group.todos[i].status.cycle();
                    group.todos[i].status = next;
                    for sub in &mut group.todos[i].subtasks {
                        sub.status = next;
                    }
                    if next.is_done() {
                        completed_todo = Some(i);
                    }
                }
                RowRef::Sub(i, j) => {
                    let sub = &mut group.todos[i].subtasks[j];
                    sub.status = sub.status.cycle();
                }
            }
        }

        if let Some(todo) = completed_todo
            && self.timer.as_ref().is_some_and(|timer| {
                timer.target
                    == TimerTarget {
                        group: self.selected_group,
                        todo,
                    }
            })
        {
            self.timer = None;
        }
        self.dirty = true;
        self.select_row(row);
    }

    pub fn delete_selected(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let group_idx = self.selected_group;
        let Some(group) = self.groups.get_mut(group_idx) else {
            return;
        };
        match row {
            RowRef::Todo(i) => {
                let item = group.todos.remove(i);
                self.last_deleted = Some(LastDeleted::Todo {
                    group: group_idx,
                    index: i,
                    item,
                });
                self.adjust_timer_after_todo_removed(group_idx, i);
            }
            RowRef::Sub(i, j) => {
                let item = group.todos[i].subtasks.remove(j);
                self.last_deleted = Some(LastDeleted::Sub {
                    group: group_idx,
                    todo: i,
                    index: j,
                    item,
                });
            }
        }
        self.dirty = true;
        self.clamp_selection();
    }

    fn adjust_timer_after_todo_removed(&mut self, group: usize, removed: usize) {
        let Some(timer) = self.timer.as_mut() else {
            return;
        };
        if timer.target.group != group {
            return;
        }
        if timer.target.todo == removed {
            self.timer = None;
        } else if timer.target.todo > removed {
            timer.target.todo -= 1;
        }
    }

    fn adjust_timer_after_todo_inserted(&mut self, group: usize, inserted: usize) {
        if let Some(timer) = self.timer.as_mut()
            && timer.target.group == group
            && timer.target.todo >= inserted
        {
            timer.target.todo += 1;
        }
    }

    fn adjust_timer_after_group_removed(&mut self, removed: usize) {
        let Some(timer) = self.timer.as_mut() else {
            return;
        };
        if timer.target.group == removed {
            self.timer = None;
        } else if timer.target.group > removed {
            timer.target.group -= 1;
        }
    }

    pub fn undo(&mut self) {
        let Some(deleted) = self.last_deleted.take() else {
            return;
        };
        match deleted {
            LastDeleted::Todo { group, index, item } => {
                let Some(g) = self.groups.get_mut(group) else {
                    return;
                };
                let at = index.min(g.todos.len());
                g.todos.insert(at, item);
                self.adjust_timer_after_todo_inserted(group, at);
                self.selected_group = group;
                self.dirty = true;
                self.select_row(RowRef::Todo(at));
            }
            LastDeleted::Sub {
                group,
                todo,
                index,
                item,
            } => {
                let Some(g) = self.groups.get_mut(group) else {
                    return;
                };
                let Some(t) = g.todos.get_mut(todo) else {
                    return;
                };
                let at = index.min(t.subtasks.len());
                t.subtasks.insert(at, item);
                t.collapsed = false;
                self.selected_group = group;
                self.dirty = true;
                self.select_row(RowRef::Sub(todo, at));
            }
        }
    }

    pub fn toggle_help(&mut self) {
        self.mode = match self.mode {
            AppMode::Help => AppMode::Normal,

            _ => AppMode::Help,
        };
    }

    pub fn move_down(&mut self) {
        let count = self.visible_rows().len();
        if count > 0 {
            self.selected = (self.selected + 1).min(count - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn enter_edit_mode(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let Some(group) = self.groups.get(self.selected_group) else {
            return;
        };
        let (title, target) = match row {
            RowRef::Todo(i) => (group.todos[i].title.clone(), InputTarget::EditTodo(i)),
            RowRef::Sub(i, j) => (
                group.todos[i].subtasks[j].title.clone(),
                InputTarget::EditSubtask(i, j),
            ),
        };
        self.input_buffer = title;
        self.input_target = target;
        self.mode = AppMode::Input;
    }

    pub fn enter_input_mode(&mut self) {
        self.mode = AppMode::Input;
        self.input_buffer.clear();
        self.input_target = InputTarget::NewTodo;
    }

    pub fn cancel_input(&mut self) {
        self.mode = AppMode::Normal;
        self.input_buffer.clear();
        self.input_target = InputTarget::NewTodo;
    }

    pub fn commit_input(&mut self) {
        let title = self.input_buffer.trim().to_owned();
        let mut to_select: Option<RowRef> = None;

        if !title.is_empty() {
            match self.input_target {
                InputTarget::NewTodo => {
                    if let Some(group) = self.groups.get_mut(self.selected_group) {
                        group.todos.push(TodoItem::new(title));
                        to_select = Some(RowRef::Todo(group.todos.len() - 1));
                        self.dirty = true;
                    }
                }
                InputTarget::NewSubtask(parent) => {
                    if let Some(group) = self.groups.get_mut(self.selected_group)
                        && let Some(todo) = group.todos.get_mut(parent)
                    {
                        todo.subtasks.push(SubTask::new(title));
                        todo.collapsed = false;
                        to_select = Some(RowRef::Sub(parent, todo.subtasks.len() - 1));
                        self.dirty = true;
                    }
                }
                InputTarget::EditTodo(i) => {
                    if let Some(group) = self.groups.get_mut(self.selected_group)
                        && let Some(todo) = group.todos.get_mut(i)
                        && todo.title != title
                    {
                        todo.title = title;
                        self.dirty = true;
                    }
                }
                InputTarget::EditSubtask(i, j) => {
                    if let Some(group) = self.groups.get_mut(self.selected_group)
                        && let Some(todo) = group.todos.get_mut(i)
                        && let Some(sub) = todo.subtasks.get_mut(j)
                        && sub.title != title
                    {
                        sub.title = title;
                        self.dirty = true;
                    }
                }
            }
        }

        self.input_buffer.clear();
        self.input_target = InputTarget::NewTodo;
        self.mode = AppMode::Normal;
        if let Some(row) = to_select {
            self.select_row(row);
        }
        self.clamp_selection();
    }

    pub fn enter_subtask_input(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let parent = match row {
            RowRef::Todo(i) => i,
            RowRef::Sub(i, _) => i,
        };
        self.input_buffer.clear();
        self.input_target = InputTarget::NewSubtask(parent);
        self.mode = AppMode::Input;
    }

    pub fn cycle_priority(&mut self) {
        let Some(RowRef::Todo(i)) = self.current_row() else {
            return;
        };
        if let Some(group) = self.groups.get_mut(self.selected_group) {
            group.todos[i].priority = group.todos[i].priority.cycle();
            self.dirty = true;
        }
        self.select_row(RowRef::Todo(i));
    }

    pub fn toggle_timer(&mut self) {
        let Some(RowRef::Todo(todo_index)) = self.current_row() else {
            return;
        };

        let target = TimerTarget {
            group: self.selected_group,
            todo: todo_index,
        };

        if let Some(timer) = self.timer.as_mut() {
            if timer.target == target {
                timer.running = !timer.running;
            }
            return;
        }

        let Some(todo) = self
            .groups
            .get_mut(target.group)
            .and_then(|group| group.todos.get_mut(target.todo))
        else {
            return;
        };
        if todo.status.is_done() {
            return;
        }

        if todo.status != TodoStatus::InProgress {
            todo.status = TodoStatus::InProgress;
            for subtask in &mut todo.subtasks {
                subtask.status = TodoStatus::InProgress;
            }
            self.dirty = true;
        }

        self.timer = Some(PomodoroTimer::new(target));
        self.select_row(RowRef::Todo(todo_index));
    }

    pub fn cancel_timer(&mut self) {
        let selected_row = self.current_row();
        self.timer = None;
        if self.mode == AppMode::Zen {
            self.mode = AppMode::Normal;
        }
        if let Some(row) = selected_row {
            self.select_row(row);
        }
    }

    pub fn tick_timer(&mut self, elapsed: Duration) {
        let Some(timer) = self.timer.as_mut() else {
            return;
        };
        if !timer.running {
            return;
        }
        if elapsed < timer.remaining {
            timer.remaining -= elapsed;
            return;
        }

        let target = timer.target;
        let selected_row = self.current_row();
        self.timer = None;

        if let Some(todo) = self
            .groups
            .get_mut(target.group)
            .and_then(|group| group.todos.get_mut(target.todo))
        {
            todo.pomodoros = todo.pomodoros.saturating_add(1);
            self.dirty = true;
        }

        if self.mode == AppMode::Zen {
            self.mode = AppMode::Normal;
        }
        if let Some(row) = selected_row {
            self.select_row(row);
        }
    }

    pub fn active_timer_todo(&self) -> Option<(&Group, &TodoItem)> {
        let target = self.timer.as_ref()?.target;
        let group = self.groups.get(target.group)?;
        let todo = group.todos.get(target.todo)?;
        Some((group, todo))
    }

    pub fn enter_zen_mode(&mut self) {
        if self.timer.is_none() {
            self.toggle_timer();
        }
        if self.active_timer_todo().is_some() {
            self.mode = AppMode::Zen;
        }
    }

    pub fn exit_zen_mode(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn toggle_active_timer(&mut self) {
        if let Some(timer) = self.timer.as_mut() {
            timer.running = !timer.running;
        }
    }

    pub fn complete_active_timer_todo(&mut self) {
        let selected_row = self.current_row();
        let Some(target) = self.timer.as_ref().map(|timer| timer.target) else {
            self.mode = AppMode::Normal;
            return;
        };

        if let Some(todo) = self
            .groups
            .get_mut(target.group)
            .and_then(|group| group.todos.get_mut(target.todo))
        {
            todo.status = TodoStatus::Done;
            for subtask in &mut todo.subtasks {
                subtask.status = TodoStatus::Done;
            }
            self.dirty = true;
        }

        self.timer = None;
        self.mode = AppMode::Normal;
        if let Some(row) = selected_row {
            self.select_row(row);
        }
    }

    pub fn enter_notes_mode(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let Some(group) = self.groups.get(self.selected_group) else {
            return;
        };
        let notes = match row {
            RowRef::Todo(i) => group.todos.get(i).map(|todo| todo.notes.as_str()),
            RowRef::Sub(i, j) => group
                .todos
                .get(i)
                .and_then(|todo| todo.subtasks.get(j))
                .map(|subtask| subtask.notes.as_str()),
        };
        let Some(notes) = notes else {
            return;
        };

        self.notes_buffer = notes.to_owned();
        self.mode = AppMode::EditingNotes;
    }

    pub fn commit_notes(&mut self) {
        let Some(row) = self.current_row() else {
            self.mode = AppMode::Normal;
            self.notes_buffer.clear();
            return;
        };
        let Some(group) = self.groups.get_mut(self.selected_group) else {
            self.mode = AppMode::Normal;
            self.notes_buffer.clear();
            return;
        };
        let notes = match row {
            RowRef::Todo(i) => group.todos.get_mut(i).map(|todo| &mut todo.notes),
            RowRef::Sub(i, j) => group
                .todos
                .get_mut(i)
                .and_then(|todo| todo.subtasks.get_mut(j))
                .map(|subtask| &mut subtask.notes),
        };
        let Some(notes) = notes else {
            self.mode = AppMode::Normal;
            self.notes_buffer.clear();
            return;
        };

        if *notes != self.notes_buffer {
            *notes = std::mem::take(&mut self.notes_buffer);
            self.dirty = true;
        } else {
            self.notes_buffer.clear();
        }
        self.mode = AppMode::Normal;
    }

    pub fn cancel_notes(&mut self) {
        self.notes_buffer.clear();
        self.mode = AppMode::Normal;
    }

    pub fn current_notes(&self) -> Option<&str> {
        let row = self.current_row()?;
        let group = self.groups.get(self.selected_group)?;
        match row {
            RowRef::Todo(i) => group.todos.get(i).map(|todo| todo.notes.as_str()),
            RowRef::Sub(i, j) => group
                .todos
                .get(i)?
                .subtasks
                .get(j)
                .map(|subtask| subtask.notes.as_str()),
        }
    }

    pub fn visible_rows(&self) -> Vec<RowRef> {
        let Some(group) = self.groups.get(self.selected_group) else {
            return Vec::new();
        };
        let query = self.search_query.to_lowercase();
        let active_target = self.timer.as_ref().map(|timer| timer.target);
        let mut todo_indices: Vec<_> = group
            .todos
            .iter()
            .enumerate()
            .filter(|(_, todo)| query.is_empty() || todo.title.to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect();
        todo_indices.sort_by_key(|&index| {
            let todo = &group.todos[index];
            let target = TimerTarget {
                group: self.selected_group,
                todo: index,
            };
            let active_timer_rank = u8::from(active_target != Some(target));
            let priority_rank = if todo.status.is_done() {
                0
            } else {
                todo.priority.display_sort_rank()
            };

            (
                todo.status.display_sort_rank(),
                active_timer_rank,
                priority_rank,
            )
        });

        let mut rows = Vec::new();
        for i in todo_indices {
            let todo = &group.todos[i];
            rows.push(RowRef::Todo(i));
            if !todo.collapsed {
                let mut subtask_indices: Vec<_> = (0..todo.subtasks.len()).collect();
                subtask_indices.sort_by_key(|&j| todo.subtasks[j].status.display_sort_rank());
                for j in subtask_indices {
                    rows.push(RowRef::Sub(i, j));
                }
            }
        }
        rows
    }

    pub fn current_row(&self) -> Option<RowRef> {
        self.visible_rows().get(self.selected).copied()
    }

    pub fn current_todos(&self) -> &[TodoItem] {
        self.groups
            .get(self.selected_group)
            .map(|g| g.todos.as_slice())
            .unwrap_or(&[])
    }

    fn clamp_selection(&mut self) {
        let count = self.visible_rows().len();
        self.selected = if count == 0 {
            0
        } else {
            self.selected.min(count - 1)
        };
    }

    fn select_row(&mut self, target: RowRef) {
        if let Some(pos) = self.visible_rows().iter().position(|&r| r == target) {
            self.selected = pos;
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.search_query.clear();
        self.search_selected = 0;
        self.mode = AppMode::Search;
    }

    pub fn push_search(&mut self, c: char) {
        self.search_query.push(c);
        self.search_selected = 0;
    }

    pub fn backspace_search(&mut self) {
        self.search_query.pop();
        self.search_selected = 0;
    }

    pub fn global_search_results(&self) -> Vec<SearchHit> {
        let query = self.search_query.trim();
        let mut scored = Vec::new();
        let mut order = 0usize;

        for (group_index, group) in self.groups.iter().enumerate() {
            let group_score = fuzzy_score(&group.name, query);

            for (todo_index, todo) in group.todos.iter().enumerate() {
                let todo_score = fuzzy_score(&todo.title, query);
                if let Some(score) = [group_score, todo_score].into_iter().flatten().max() {
                    scored.push((
                        score,
                        order,
                        SearchHit {
                            group: group_index,
                            row: RowRef::Todo(todo_index),
                        },
                    ));
                }
                order += 1;

                for (subtask_index, subtask) in todo.subtasks.iter().enumerate() {
                    let subtask_score = fuzzy_score(&subtask.title, query);
                    if let Some(score) = [group_score, todo_score, subtask_score]
                        .into_iter()
                        .flatten()
                        .max()
                    {
                        scored.push((
                            score,
                            order,
                            SearchHit {
                                group: group_index,
                                row: RowRef::Sub(todo_index, subtask_index),
                            },
                        ));
                    }
                    order += 1;
                }
            }
        }

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        scored.into_iter().map(|(_, _, hit)| hit).collect()
    }

    pub fn move_search_down(&mut self) {
        let count = self.global_search_results().len();
        if count > 0 {
            self.search_selected = (self.search_selected + 1).min(count - 1);
        }
    }

    pub fn move_search_up(&mut self) {
        self.search_selected = self.search_selected.saturating_sub(1);
    }

    pub fn confirm_search(&mut self) {
        let Some(hit) = self
            .global_search_results()
            .get(self.search_selected)
            .copied()
        else {
            return;
        };

        if let RowRef::Sub(todo_index, _) = hit.row
            && let Some(todo) = self
                .groups
                .get_mut(hit.group)
                .and_then(|group| group.todos.get_mut(todo_index))
        {
            todo.collapsed = false;
        }

        self.switch_group(hit.group);
        self.search_query.clear();
        self.search_selected = 0;
        self.mode = AppMode::Normal;
        self.select_row(hit.row);
    }

    pub fn cancel_search(&mut self) {
        self.search_query.clear();
        self.search_selected = 0;
        self.mode = AppMode::Normal;
    }

    pub fn next_group(&mut self) {
        if self.groups.len() <= 1 {
            return;
        }
        let next = (self.selected_group + 1) % self.groups.len();
        self.switch_group(next);
    }

    pub fn prev_group(&mut self) {
        if self.groups.len() <= 1 {
            return;
        }
        let previous = (self.selected_group + self.groups.len() - 1) % self.groups.len();
        self.switch_group(previous);
    }

    pub fn switch_group(&mut self, index: usize) {
        if index >= self.groups.len() || index == self.selected_group {
            return;
        }
        self.selected_group = index;
        self.reset_group_view();
        self.dirty = true;
    }

    fn reset_group_view(&mut self) {
        self.selected = 0;
        self.search_query.clear();
    }

    pub fn toggle_collapse(&mut self) {
        let Some(row) = self.current_row() else {
            return;
        };
        let parent = match row {
            RowRef::Todo(i) => i,
            RowRef::Sub(i, _) => i,
        };
        if let Some(group) = self.groups.get_mut(self.selected_group) {
            let Some(todo) = group.todos.get_mut(parent) else {
                return;
            };
            if todo.subtasks.is_empty() {
                return;
            }
            todo.collapsed = !todo.collapsed;
        }
        self.select_row(RowRef::Todo(parent));
        self.clamp_selection();
    }

    pub fn enter_category_popup(&mut self) {
        self.category_cursor = self.selected_group;
        self.mode = AppMode::CategoryPopup;
    }

    pub fn category_cursor_up(&mut self) {
        self.category_cursor = self.category_cursor.saturating_sub(1);
    }

    pub fn category_cursor_down(&mut self) {
        if self.category_cursor + 1 < self.groups.len() {
            self.category_cursor += 1;
        }
    }

    pub fn confirm_category_popup(&mut self) {
        self.switch_group(self.category_cursor);
        self.mode = AppMode::Normal;
    }

    pub fn cancel_category_popup(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn enter_group_add(&mut self) {
        self.group_editing = None;
        self.input_buffer.clear();
        self.mode = AppMode::GroupInput;
    }

    pub fn enter_group_rename(&mut self) {
        if let Some(group) = self.groups.get(self.category_cursor) {
            self.input_buffer = group.name.clone();
            self.group_editing = Some(self.category_cursor);
            self.mode = AppMode::GroupInput;
        }
    }

    pub fn commit_group_input(&mut self) {
        let name = self.input_buffer.trim().to_owned();
        if !name.is_empty() {
            match self.group_editing {
                Some(i) => {
                    if let Some(group) = self.groups.get_mut(i)
                        && group.name != name
                    {
                        group.name = name;
                        self.dirty = true;
                    }
                }
                None => {
                    self.groups.push(Group::new(name));
                    self.category_cursor = self.groups.len() - 1;
                    self.dirty = true;
                }
            }
        }
        self.input_buffer.clear();
        self.group_editing = None;
        self.mode = AppMode::CategoryPopup;
    }

    pub fn cancel_group_input(&mut self) {
        self.input_buffer.clear();
        self.group_editing = None;
        self.mode = AppMode::CategoryPopup;
    }

    pub fn enter_group_delete_confirm(&mut self) {
        if self.groups.len() <= 1 {
            return;
        }
        self.mode = AppMode::GroupDeleteConfirm;
    }

    pub fn confirm_group_delete(&mut self) {
        if self.groups.len() <= 1 {
            self.mode = AppMode::CategoryPopup;
            return;
        }
        let target = self.category_cursor.min(self.groups.len() - 1);
        self.groups.remove(target);
        self.adjust_timer_after_group_removed(target);

        if self.selected_group == target {
            self.selected_group = self.selected_group.min(self.groups.len() - 1);
            self.reset_group_view();
        } else if self.selected_group > target {
            self.selected_group -= 1;
        }

        self.category_cursor = self.category_cursor.min(self.groups.len() - 1);
        self.last_deleted = None;
        self.dirty = true;
        self.mode = AppMode::CategoryPopup;
    }

    pub fn cancel_group_delete(&mut self) {
        self.mode = AppMode::CategoryPopup;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_todo() -> App {
        let mut app = App::default();
        let mut todo = TodoItem::new("집중할 작업");
        todo.subtasks.push(SubTask::new("세부 작업"));
        app.groups[0].todos.push(todo);
        app
    }

    #[test]
    fn timer_starts_pauses_resumes_and_completes() {
        let mut app = app_with_todo();

        app.toggle_timer();
        let timer = app.timer.as_ref().expect("timer should start");
        assert!(timer.running);
        assert_eq!(timer.display_remaining(), "25:00");
        assert_eq!(app.groups[0].todos[0].status, TodoStatus::InProgress);
        assert_eq!(
            app.groups[0].todos[0].subtasks[0].status,
            TodoStatus::InProgress
        );

        app.toggle_timer();
        assert!(!app.timer.as_ref().unwrap().running);
        app.tick_timer(Duration::from_secs(10));
        assert_eq!(app.timer.as_ref().unwrap().display_remaining(), "25:00");

        app.toggle_timer();
        app.tick_timer(Duration::from_secs(1));
        assert_eq!(app.timer.as_ref().unwrap().display_remaining(), "24:59");

        app.tick_timer(POMODORO_DURATION);
        assert!(app.timer.is_none());
        assert_eq!(app.groups[0].todos[0].pomodoros, 1);
        assert!(app.dirty);
    }

    #[test]
    fn timer_target_tracks_insertions_and_deletions() {
        let mut app = App::default();
        app.groups[0].todos = vec![
            TodoItem::new("앞 작업"),
            TodoItem::new("집중할 작업"),
            TodoItem::new("뒤 작업"),
        ];
        app.selected = 1;
        app.toggle_timer();

        app.select_row(RowRef::Todo(0));
        app.delete_selected();
        assert_eq!(app.timer.as_ref().unwrap().target.todo, 0);
        assert_eq!(app.active_timer_todo().unwrap().1.title, "집중할 작업");

        app.undo();
        assert_eq!(app.timer.as_ref().unwrap().target.todo, 1);
        assert_eq!(app.active_timer_todo().unwrap().1.title, "집중할 작업");

        app.select_row(RowRef::Todo(1));
        app.delete_selected();
        assert!(app.timer.is_none());
    }

    #[test]
    fn completing_active_todo_cancels_timer_without_adding_pomodoro() {
        let mut app = app_with_todo();
        app.toggle_timer();

        app.cycle_status();

        assert!(app.groups[0].todos[0].status.is_done());
        assert!(app.timer.is_none());
        assert_eq!(app.groups[0].todos[0].pomodoros, 0);
    }

    #[test]
    fn entering_zen_starts_timer_and_pause_does_not_depend_on_selection() {
        let mut app = app_with_todo();

        app.enter_zen_mode();
        assert_eq!(app.mode, AppMode::Zen);
        assert!(app.timer.as_ref().unwrap().running);

        app.selected = 0;
        app.toggle_active_timer();
        assert!(!app.timer.as_ref().unwrap().running);
        app.toggle_active_timer();
        assert!(app.timer.as_ref().unwrap().running);
    }

    #[test]
    fn completing_todo_from_zen_finishes_task_and_exits() {
        let mut app = app_with_todo();
        app.enter_zen_mode();

        app.complete_active_timer_todo();

        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.timer.is_none());
        assert!(app.groups[0].todos[0].status.is_done());
        assert!(app.groups[0].todos[0].subtasks[0].status.is_done());
        assert_eq!(app.groups[0].todos[0].pomodoros, 0);
    }

    #[test]
    fn timer_completion_exits_zen_and_adds_one_pomodoro() {
        let mut app = app_with_todo();
        app.enter_zen_mode();

        app.tick_timer(POMODORO_DURATION);

        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.timer.is_none());
        assert_eq!(app.groups[0].todos[0].pomodoros, 1);
    }

    #[test]
    fn global_search_finds_items_across_groups_and_inside_collapsed_todos() {
        let app = App {
            groups: vec![
                Group {
                    name: "업무".into(),
                    todos: vec![TodoItem::new("API 문서 정리")],
                },
                Group {
                    name: "개인".into(),
                    todos: vec![TodoItem {
                        subtasks: vec![SubTask::new("Rust 소유권 복습")],
                        collapsed: true,
                        ..TodoItem::new("학습 계획")
                    }],
                },
            ],
            search_query: "rust".into(),
            ..App::default()
        };

        assert_eq!(
            app.global_search_results(),
            vec![SearchHit {
                group: 1,
                row: RowRef::Sub(0, 0),
            }]
        );
    }

    #[test]
    fn fuzzy_search_matches_subsequences_and_ranks_tighter_matches_first() {
        assert!(fuzzy_score("Refactor timer", "rft").is_some());
        assert!(
            fuzzy_score("rust", "rust").unwrap()
                > fuzzy_score("review rust notes", "rust").unwrap()
        );
    }

    #[test]
    fn confirming_global_search_moves_to_result_and_expands_parent() {
        let mut app = App {
            groups: vec![
                Group::new("업무"),
                Group {
                    name: "개인".into(),
                    todos: vec![TodoItem {
                        subtasks: vec![SubTask::new("Rust 소유권 복습")],
                        collapsed: true,
                        ..TodoItem::new("학습 계획")
                    }],
                },
            ],
            ..App::default()
        };
        app.enter_search_mode();
        app.search_query = "소유권".into();

        app.confirm_search();

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.selected_group, 1);
        assert_eq!(app.current_row(), Some(RowRef::Sub(0, 0)));
        assert!(!app.groups[1].todos[0].collapsed);
        assert!(app.search_query.is_empty());
        assert!(app.dirty);
    }

    #[test]
    fn canceling_global_search_preserves_original_location() {
        let mut app = App {
            groups: vec![
                Group {
                    name: "업무".into(),
                    todos: vec![TodoItem::new("첫 작업"), TodoItem::new("둘째 작업")],
                },
                Group {
                    name: "개인".into(),
                    todos: vec![TodoItem::new("개인 작업")],
                },
            ],
            selected: 1,
            ..App::default()
        };
        app.enter_search_mode();
        app.push_search('개');

        app.cancel_search();

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.selected_group, 0);
        assert_eq!(app.current_row(), Some(RowRef::Todo(1)));
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn legacy_subtask_without_notes_deserializes_with_empty_notes() {
        let subtask: SubTask =
            serde_json::from_str(r#"{"title":"기존 하위 작업","status":"Todo"}"#).unwrap();

        assert!(subtask.notes.is_empty());
    }

    #[test]
    fn subtask_notes_can_be_opened_and_committed() {
        let mut app = app_with_todo();
        app.selected = 1;

        app.enter_notes_mode();
        assert_eq!(app.mode, AppMode::EditingNotes);
        assert!(app.notes_buffer.is_empty());

        app.notes_buffer = "하위 작업 메모".into();
        app.commit_notes();

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.groups[0].todos[0].subtasks[0].notes, "하위 작업 메모");
        assert_eq!(app.current_notes(), Some("하위 작업 메모"));
        assert!(app.dirty);
    }

    #[test]
    fn canceling_subtask_notes_preserves_original_content() {
        let mut app = app_with_todo();
        app.groups[0].todos[0].subtasks[0].notes = "원본".into();
        app.selected = 1;

        app.enter_notes_mode();
        app.notes_buffer.push_str(" 변경");
        app.cancel_notes();

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.current_notes(), Some("원본"));
        assert!(app.notes_buffer.is_empty());
        assert!(!app.dirty);
    }

    #[test]
    fn todo_notes_still_use_the_same_editing_flow() {
        let mut app = app_with_todo();

        app.enter_notes_mode();
        app.notes_buffer = "상위 작업 메모".into();
        app.commit_notes();

        assert_eq!(app.groups[0].todos[0].notes, "상위 작업 메모");
        assert_eq!(app.current_notes(), Some("상위 작업 메모"));
    }

    #[test]
    fn visible_rows_sorts_subtasks_by_status_without_reordering_storage() {
        let mut todo = TodoItem::new("상위 작업");
        todo.subtasks = vec![
            SubTask {
                status: TodoStatus::Todo,
                ..SubTask::new("대기 1")
            },
            SubTask {
                status: TodoStatus::Done,
                ..SubTask::new("완료 1")
            },
            SubTask {
                status: TodoStatus::InProgress,
                ..SubTask::new("진행 1")
            },
            SubTask {
                status: TodoStatus::Todo,
                ..SubTask::new("대기 2")
            },
            SubTask {
                status: TodoStatus::InProgress,
                ..SubTask::new("진행 2")
            },
            SubTask {
                status: TodoStatus::Done,
                ..SubTask::new("완료 2")
            },
        ];
        let mut app = App::default();
        app.groups[0].todos.push(todo);

        assert_eq!(
            app.visible_rows(),
            vec![
                RowRef::Todo(0),
                RowRef::Sub(0, 2),
                RowRef::Sub(0, 4),
                RowRef::Sub(0, 0),
                RowRef::Sub(0, 3),
                RowRef::Sub(0, 1),
                RowRef::Sub(0, 5),
            ]
        );
        assert_eq!(
            app.groups[0].todos[0]
                .subtasks
                .iter()
                .map(|subtask| subtask.title.as_str())
                .collect::<Vec<_>>(),
            vec!["대기 1", "완료 1", "진행 1", "대기 2", "진행 2", "완료 2"]
        );
    }

    #[test]
    fn cycling_subtask_status_keeps_the_same_subtask_selected_after_sorting() {
        let mut todo = TodoItem::new("상위 작업");
        todo.subtasks = vec![
            SubTask::new("상태를 바꿀 작업"),
            SubTask {
                status: TodoStatus::InProgress,
                ..SubTask::new("기존 진행 작업")
            },
        ];
        let mut app = App::default();
        app.groups[0].todos.push(todo);
        app.select_row(RowRef::Sub(0, 0));

        app.cycle_status();

        assert_eq!(
            app.groups[0].todos[0].subtasks[0].status,
            TodoStatus::InProgress
        );
        assert_eq!(app.current_row(), Some(RowRef::Sub(0, 0)));
    }

    #[test]
    fn visible_rows_sorts_todo_blocks_by_status_timer_and_priority() {
        let todo = |title, status, priority| TodoItem {
            status,
            priority,
            ..TodoItem::new(title)
        };
        let mut active = todo("타이머 진행 Low", TodoStatus::InProgress, Priority::Low);
        active.subtasks = vec![
            SubTask {
                status: TodoStatus::Done,
                ..SubTask::new("완료 하위")
            },
            SubTask {
                status: TodoStatus::InProgress,
                ..SubTask::new("진행 하위")
            },
        ];
        let mut app = App::default();
        app.groups[0].todos = vec![
            todo("대기 Medium", TodoStatus::Todo, Priority::Medium),
            todo("완료 High", TodoStatus::Done, Priority::High),
            todo("진행 High", TodoStatus::InProgress, Priority::High),
            todo("대기 High", TodoStatus::Todo, Priority::High),
            active,
            todo("대기 Low", TodoStatus::Todo, Priority::Low),
            todo("완료 Low", TodoStatus::Done, Priority::Low),
        ];
        app.timer = Some(PomodoroTimer::new(TimerTarget { group: 0, todo: 4 }));

        assert_eq!(
            app.visible_rows(),
            vec![
                RowRef::Todo(4),
                RowRef::Sub(4, 1),
                RowRef::Sub(4, 0),
                RowRef::Todo(2),
                RowRef::Todo(3),
                RowRef::Todo(0),
                RowRef::Todo(5),
                RowRef::Todo(1),
                RowRef::Todo(6),
            ]
        );
        assert_eq!(
            app.groups[0]
                .todos
                .iter()
                .map(|todo| todo.title.as_str())
                .collect::<Vec<_>>(),
            vec![
                "대기 Medium",
                "완료 High",
                "진행 High",
                "대기 High",
                "타이머 진행 Low",
                "대기 Low",
                "완료 Low",
            ]
        );
    }

    #[test]
    fn cycling_todo_priority_keeps_the_same_todo_selected_after_sorting() {
        let mut app = App::default();
        app.groups[0].todos = vec![
            TodoItem::new("기존 Low"),
            TodoItem::new("우선순위를 바꿀 작업"),
        ];
        app.select_row(RowRef::Todo(1));

        app.cycle_priority();

        assert_eq!(app.groups[0].todos[1].priority, Priority::Medium);
        assert_eq!(app.current_row(), Some(RowRef::Todo(1)));
        assert_eq!(app.visible_rows(), vec![RowRef::Todo(1), RowRef::Todo(0)]);
    }

    #[test]
    fn cycling_todo_status_keeps_the_same_todo_selected_after_sorting() {
        let mut high = TodoItem::new("대기 High");
        high.priority = Priority::High;

        let mut app = App::default();
        app.groups[0].todos = vec![high, TodoItem::new("진행 중으로 바꿀 Low")];
        app.select_row(RowRef::Todo(1));

        app.cycle_status();

        assert_eq!(app.groups[0].todos[1].status, TodoStatus::InProgress);
        assert_eq!(app.visible_rows()[0], RowRef::Todo(1));
        assert_eq!(app.current_row(), Some(RowRef::Todo(1)));
    }

    #[test]
    fn timer_sort_changes_keep_the_current_todo_selected() {
        let mut high = TodoItem::new("기존 진행 High");
        high.status = TodoStatus::InProgress;
        high.priority = Priority::High;

        let mut app = App::default();
        app.groups[0].todos = vec![high, TodoItem::new("타이머를 시작할 Low")];
        app.select_row(RowRef::Todo(1));

        app.toggle_timer();
        assert_eq!(app.visible_rows()[0], RowRef::Todo(1));
        assert_eq!(app.current_row(), Some(RowRef::Todo(1)));

        app.cancel_timer();
        assert_eq!(app.visible_rows()[0], RowRef::Todo(0));
        assert_eq!(app.current_row(), Some(RowRef::Todo(1)));
    }

    #[test]
    fn selected_group_survives_a_json_round_trip() {
        let app = App {
            groups: vec![Group::new("업무"), Group::new("개인")],
            selected_group: 1,
            ..App::default()
        };

        let json = serde_json::to_string(&app).unwrap();
        let restored: App = serde_json::from_str(&json).unwrap();
        let restored = restored.normalize_loaded_state();

        assert_eq!(restored.selected_group, 1);
        assert!(!restored.dirty);
    }

    #[test]
    fn group_navigation_marks_persistent_state_dirty_and_resets_the_view() {
        let mut app = App {
            groups: vec![Group::new("업무"), Group::new("개인")],
            selected: 3,
            search_query: "rust".into(),
            ..App::default()
        };

        app.next_group();

        assert_eq!(app.selected_group, 1);
        assert_eq!(app.selected, 0);
        assert!(app.search_query.is_empty());
        assert!(app.dirty);
    }

    #[test]
    fn switching_to_the_current_or_invalid_group_is_a_no_op() {
        let mut app = App {
            groups: vec![Group::new("업무"), Group::new("개인")],
            ..App::default()
        };

        app.switch_group(0);
        app.switch_group(99);

        assert_eq!(app.selected_group, 0);
        assert!(!app.dirty);
    }

    #[test]
    fn loaded_state_repairs_empty_groups_and_out_of_range_selection() {
        let empty = App {
            groups: Vec::new(),
            selected_group: 7,
            selected: 9,
            ..App::default()
        }
        .normalize_loaded_state();

        assert_eq!(empty.groups.len(), 1);
        assert_eq!(empty.selected_group, 0);
        assert_eq!(empty.selected, 0);
        assert!(empty.dirty);

        let out_of_range = App {
            groups: vec![Group::new("업무"), Group::new("개인")],
            selected_group: 7,
            selected: 9,
            ..App::default()
        }
        .normalize_loaded_state();

        assert_eq!(out_of_range.selected_group, 1);
        assert_eq!(out_of_range.selected, 0);
        assert!(out_of_range.dirty);
    }
}
