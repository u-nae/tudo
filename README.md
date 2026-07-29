# tumeto

> **tui** + **todo** + **pomodoro**= **tumeto**

A fast, keyboard-driven terminal todo manager built with [ratatui](https://ratatui.rs).

Groups, subtasks, per-item notes, priorities, search, and undo — all from the keyboard.

## Install

```bash
cargo install tumeto
```

## Usage

Run `tumeto` in any terminal. Data is stored at `~/.tumeto_data.json`.

### Optional Nerd Font icons

tumeto uses standard Unicode icons by default, so a patched font is not required.

To enable Nerd Font icons:

```bash
TUMETO_ICONS=nerd tumeto
```

Use a **Nerd Font Mono** variant in your terminal, such as JetBrainsMono Nerd
Font Mono or Hack Nerd Font Mono. The Mono variant keeps private-use icons to
one terminal cell.

If icons appear as boxes or spacing looks uneven, remove `TUMETO_ICONS` or set
it to `unicode`.

### Zen focus clock

Press `Enter` on a todo to enter Zen Mode. The clock uses a large 8×8 pixel
font on standard terminals, a quadrant-cell font on medium terminals, and a
plain `MM:SS` fallback when space is limited.

### Keys

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `Enter` | Enter Zen Mode for the selected or active timer |
| `t` / `T` | Start or pause / cancel the timer |
| `Space` | Toggle complete (cascades to subtasks) |
| `a` | Add todo |
| `s` | Add subtask |
| `e` | Edit |
| `d` | Delete |
| `u` | Undo delete |
| `z` | Fold / unfold subtasks |
| `m` | Edit notes |
| `p` | Cycle priority |
| `Tab` / `h` / `l` | Switch group |
| `c` | Category jump popup |
| `/` | Search |
| `?` | Help |
| `q` / `Ctrl+C` | Quit |

## License

Licensed under either of MIT or Apache-2.0 at your option.
