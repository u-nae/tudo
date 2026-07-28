use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::{io, time::Duration};

use crate::app::{App, AppMode};

fn normalize_command_char(c: char) -> char {
    match c {
        'ㅂ' | 'ᄇ' => 'q',
        'ㅈ' | 'ᄌ' => 'w',
        'ㄷ' | 'ᄃ' => 'e',
        'ㄱ' | 'ᄀ' => 'r',
        'ㅅ' | 'ᄉ' => 't',
        'ㅛ' | 'ᅭ' => 'y',
        'ㅕ' | 'ᅧ' => 'u',
        'ㅑ' | 'ᅣ' => 'i',
        'ㅐ' | 'ᅢ' => 'o',
        'ㅔ' | 'ᅦ' => 'p',
        'ㅁ' | 'ᄆ' => 'a',
        'ㄴ' | 'ᄂ' => 's',
        'ㅇ' | 'ᄋ' => 'd',
        'ㄹ' | 'ᄅ' => 'f',
        'ㅎ' | 'ᄒ' => 'g',
        'ㅗ' | 'ᅩ' => 'h',
        'ㅓ' | 'ᅥ' => 'j',
        'ㅏ' | 'ᅡ' => 'k',
        'ㅣ' | 'ᅵ' => 'l',
        'ㅋ' | 'ᄏ' => 'z',
        'ㅌ' | 'ᄐ' => 'x',
        'ㅊ' | 'ᄎ' => 'c',
        'ㅍ' | 'ᄑ' => 'v',
        'ㅠ' | 'ᅲ' => 'b',
        'ㅜ' | 'ᅮ' => 'n',
        'ㅡ' | 'ᅳ' => 'm',
        'ㅃ' | 'ᄈ' => 'Q',
        'ㅉ' | 'ᄍ' => 'W',
        'ㄸ' | 'ᄄ' => 'E',
        'ㄲ' | 'ᄁ' => 'R',
        'ㅆ' | 'ᄊ' => 'T',
        'ㅒ' | 'ᅤ' => 'O',
        'ㅖ' | 'ᅨ' => 'P',
        _ => c,
    }
}

fn normalize_command_key(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char(c) => KeyCode::Char(normalize_command_char(c)),
        other => other,
    }
}

fn normalize_key_for_mode(mode: &AppMode, code: KeyCode) -> KeyCode {
    if matches!(
        mode,
        AppMode::Input | AppMode::EditingNotes | AppMode::Search | AppMode::GroupInput
    ) {
        code
    } else {
        normalize_command_key(code)
    }
}

pub fn handle_events(app: &mut App) -> io::Result<()> {
    if event::poll(Duration::from_millis(100))?
        && let Event::Key(key) = event::read()?
    {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        let command_code = normalize_command_key(key.code);

        if key.modifiers == KeyModifiers::CONTROL && command_code == KeyCode::Char('c') {
            app.should_quit = true;
            return Ok(());
        }

        if key.modifiers == KeyModifiers::CONTROL && command_code == KeyCode::Char('s') {
            if app.mode == AppMode::EditingNotes {
                app.commit_notes();
            }
            return Ok(());
        }

        let code = normalize_key_for_mode(&app.mode, key.code);

        match app.mode {
            AppMode::Normal => handle_normal_mode(app, code),
            AppMode::Input => handle_input_mode(app, code),
            AppMode::Search => handle_search_mode(app, code),
            AppMode::Help => app.toggle_help(),
            AppMode::EditingNotes => handle_editing_notes_mode(app, code),
            AppMode::CategoryPopup => handle_category_popup(app, code),
            AppMode::GroupInput => handle_group_input(app, code),
            AppMode::GroupDeleteConfirm => handle_group_delete_confirm(app, code),
            AppMode::Zen => handle_zen_mode(app, code),
        }
    }
    Ok(())
}

fn handle_normal_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,

        KeyCode::Char('j') | KeyCode::Down => app.move_down(),

        KeyCode::Char('k') | KeyCode::Up => app.move_up(),

        KeyCode::Char(' ') => app.cycle_status(),

        KeyCode::Char('a') | KeyCode::Char('i') => app.enter_input_mode(),

        KeyCode::Char('d') | KeyCode::Char('x') => app.delete_selected(),

        KeyCode::Char('e') => app.enter_edit_mode(),

        KeyCode::Char('u') => app.undo(),

        KeyCode::Char('?') => app.toggle_help(),

        KeyCode::Tab | KeyCode::Char('l') => app.next_group(),

        KeyCode::BackTab | KeyCode::Char('h') => app.prev_group(),

        KeyCode::Char('c') => app.enter_category_popup(),

        KeyCode::Char('m') => app.enter_notes_mode(),

        KeyCode::Char('p') => app.cycle_priority(),

        KeyCode::Char('t') => app.toggle_timer(),

        KeyCode::Char('T') => app.cancel_timer(),

        KeyCode::Char('s') => app.enter_subtask_input(),

        KeyCode::Char('z') => app.toggle_collapse(),

        KeyCode::Char('/') => app.enter_search_mode(),

        KeyCode::Enter => app.enter_zen_mode(),

        _ => {}
    }
}

fn handle_zen_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char(' ') => app.toggle_active_timer(),
        KeyCode::Char('c') => app.complete_active_timer_todo(),
        KeyCode::Char('T') => app.cancel_timer(),
        KeyCode::Esc => app.exit_zen_mode(),
        _ => {}
    }
}

fn handle_input_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.commit_input(),

        KeyCode::Esc => app.cancel_input(),

        KeyCode::Backspace => {
            let _ = app.input_buffer.pop();
        }

        KeyCode::Char(c) => app.input_buffer.push(c),

        _ => {}
    }
}

fn handle_editing_notes_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.cancel_notes(),

        KeyCode::Enter => app.notes_buffer.push('\n'),

        KeyCode::Backspace => {
            let _ = app.notes_buffer.pop();
        }

        KeyCode::Char(c) => app.notes_buffer.push(c),

        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.confirm_search(),
        KeyCode::Esc => app.cancel_search(),
        KeyCode::Up => app.move_search_up(),
        KeyCode::Down => app.move_search_down(),
        KeyCode::Backspace => app.backspace_search(),
        KeyCode::Char(c) => app.push_search(c),
        _ => {}
    }
}

fn handle_category_popup(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => app.category_cursor_up(),
        KeyCode::Down | KeyCode::Char('j') => app.category_cursor_down(),
        KeyCode::Char('n') => app.enter_group_add(),
        KeyCode::Char('r') => app.enter_group_rename(),
        KeyCode::Char('d') => app.enter_group_delete_confirm(),
        KeyCode::Enter => app.confirm_category_popup(),
        KeyCode::Esc => app.cancel_category_popup(),
        _ => {}
    }
}

fn handle_group_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.commit_group_input(),
        KeyCode::Esc => app.cancel_group_input(),
        KeyCode::Backspace => {
            let _ = app.input_buffer.pop();
        }
        KeyCode::Char(c) => app.input_buffer.push(c),
        _ => {}
    }
}

fn handle_group_delete_confirm(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_group_delete(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_group_delete(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_two_set_korean_compatibility_jamo_to_physical_keys() {
        let pairs = [
            ('ㅂ', 'q'),
            ('ㅈ', 'w'),
            ('ㄷ', 'e'),
            ('ㄱ', 'r'),
            ('ㅅ', 't'),
            ('ㅛ', 'y'),
            ('ㅕ', 'u'),
            ('ㅑ', 'i'),
            ('ㅐ', 'o'),
            ('ㅔ', 'p'),
            ('ㅁ', 'a'),
            ('ㄴ', 's'),
            ('ㅇ', 'd'),
            ('ㄹ', 'f'),
            ('ㅎ', 'g'),
            ('ㅗ', 'h'),
            ('ㅓ', 'j'),
            ('ㅏ', 'k'),
            ('ㅣ', 'l'),
            ('ㅋ', 'z'),
            ('ㅌ', 'x'),
            ('ㅊ', 'c'),
            ('ㅍ', 'v'),
            ('ㅠ', 'b'),
            ('ㅜ', 'n'),
            ('ㅡ', 'm'),
        ];

        for (korean, english) in pairs {
            assert_eq!(normalize_command_char(korean), english);
        }
    }

    #[test]
    fn maps_canonical_jamo_and_shifted_t() {
        assert_eq!(normalize_command_char('ᅥ'), 'j');
        assert_eq!(normalize_command_char('ᄎ'), 'c');
        assert_eq!(normalize_command_char('ㅆ'), 'T');
        assert_eq!(normalize_command_char('ᄊ'), 'T');
    }

    #[test]
    fn preserves_hangul_in_text_input_modes() {
        for mode in [
            AppMode::Input,
            AppMode::EditingNotes,
            AppMode::Search,
            AppMode::GroupInput,
        ] {
            assert_eq!(
                normalize_key_for_mode(&mode, KeyCode::Char('ㅓ')),
                KeyCode::Char('ㅓ')
            );
        }
    }

    #[test]
    fn normalizes_hangul_in_command_modes_only() {
        assert_eq!(
            normalize_key_for_mode(&AppMode::Normal, KeyCode::Char('ㅓ')),
            KeyCode::Char('j')
        );
        assert_eq!(
            normalize_key_for_mode(&AppMode::Zen, KeyCode::Char('ㅊ')),
            KeyCode::Char('c')
        );
        assert_eq!(
            normalize_key_for_mode(&AppMode::Zen, KeyCode::Char('ㅆ')),
            KeyCode::Char('T')
        );
    }
}
