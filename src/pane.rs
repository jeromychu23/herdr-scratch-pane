use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Position;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Clear};

use crate::config::{AppConfig, Rgb};
use crate::geometry::{box_inner, box_rect, Rect};
use crate::mouse::{hit_test, HitZone};
use crate::scope::{session_name, Scope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtachCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct ActiveResize {
    zone: HitZone,
    start_x: u16,
    start_y: u16,
    start_cfg: AppConfig,
}

enum PaneEvent {
    Output,
    Exit,
}

pub fn dtach_command(
    scope: Scope,
    workspace_id: Option<&str>,
    server_id: Option<&str>,
    state_dir: &Path,
    shell: &str,
    cwd: Option<&Path>,
) -> DtachCommand {
    let socket = state_dir.join(format!(
        "{}.dtach",
        session_name(scope, workspace_id, server_id)
    ));
    DtachCommand {
        program: "dtach".into(),
        args: vec![
            "-A".into(),
            socket.display().to_string(),
            "-z".into(),
            shell.into(),
            "-l".into(),
        ],
        cwd: cwd.map(Path::to_path_buf),
    }
}

pub fn resize_from_drag(
    area: Rect,
    start_cfg: &AppConfig,
    zone: HitZone,
    start_x: u16,
    start_y: u16,
    current_x: u16,
    current_y: u16,
) -> AppConfig {
    let start_box = box_rect(area, start_cfg);
    let dx = i32::from(current_x) - i32::from(start_x);
    let dy = i32::from(current_y) - i32::from(start_y);

    let mut width = i32::from(start_box.width);
    let mut height = i32::from(start_box.height);

    match zone {
        HitZone::ResizeLeft | HitZone::ResizeTopLeft | HitZone::ResizeBottomLeft => width -= dx,
        HitZone::ResizeRight | HitZone::ResizeTopRight | HitZone::ResizeBottomRight => width += dx,
        _ => {}
    }
    match zone {
        HitZone::ResizeTop | HitZone::ResizeTopLeft | HitZone::ResizeTopRight => height -= dy,
        HitZone::ResizeBottom | HitZone::ResizeBottomLeft | HitZone::ResizeBottomRight => {
            height += dy
        }
        _ => {}
    }

    let width_pct = pct(width, area.width, 20);
    let height_pct = pct(height, area.height, 20);
    start_cfg.with_size(width_pct, height_pct)
}

pub fn run(scope: Scope) -> Result<()> {
    if !command_exists("dtach") {
        bail!("dtach is required for persistent floating panes. Install it with `brew install dtach`.");
    }

    let config_path = config_path();
    let mut cfg = load_config(&config_path)?;
    let state_dir = state_dir();
    std::fs::create_dir_all(&state_dir)
        .with_context(|| format!("failed to create state dir {}", state_dir.display()))?;

    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    let area = Rect::new(0, 0, cols, rows);
    let inner = box_inner(area, &cfg);

    let pty = native_pty_system();
    let pair = pty.openpty(pty_size(inner)).context("failed to open PTY")?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let workspace_id = std::env::var("HERDR_WORKSPACE_ID").ok();
    let server_id = std::env::var("HERDR_SERVER_ID").ok();
    let cwd = std::env::var("HERDR_FLOATING_PANE_CWD")
        .ok()
        .map(PathBuf::from);
    let dtach = dtach_command(
        scope,
        workspace_id.as_deref(),
        server_id.as_deref(),
        &state_dir,
        &shell,
        cwd.as_deref(),
    );

    let mut command = CommandBuilder::new(dtach.program);
    for arg in dtach.args {
        command.arg(arg);
    }
    for (key, value) in std::env::vars() {
        command.env(key, value);
    }
    command.env("TERM", "xterm-256color");
    if let Some(cwd) = dtach.cwd {
        if cwd.is_dir() {
            command.cwd(cwd);
        }
    }

    let mut child = pair
        .slave
        .spawn_command(command)
        .context("failed to spawn dtach shell")?;
    drop(pair.slave);

    let parser = Arc::new(Mutex::new(vt100::Parser::new(inner.height, inner.width, 0)));
    let (tx, rx) = mpsc::channel();

    {
        let parser = Arc::clone(&parser);
        let tx = tx.clone();
        let mut reader = pair
            .master
            .try_clone_reader()
            .context("failed to clone PTY reader")?;
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        let _ = tx.send(PaneEvent::Exit);
                        break;
                    }
                    Ok(n) => {
                        parser.lock().unwrap().process(&buf[..n]);
                        if tx.send(PaneEvent::Output).is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }

    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let _ = child.wait();
            let _ = tx.send(PaneEvent::Exit);
        });
    }

    let mut writer = pair
        .master
        .take_writer()
        .context("failed to open PTY writer")?;
    let master = pair.master;

    setup_terminal()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    let mut active_resize: Option<ActiveResize> = None;
    let mut exit = false;

    while !exit {
        {
            let screen = parser.lock().unwrap();
            terminal.draw(|frame| draw(frame, &cfg, screen.screen()))?;
        }

        while event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key) => {
                    if let Some(bytes) = key_to_bytes(key) {
                        writer.write_all(&bytes)?;
                        writer.flush()?;
                    }
                }
                Event::Paste(text) => {
                    writer.write_all(text.as_bytes())?;
                    writer.flush()?;
                }
                Event::Mouse(mouse) => {
                    let size = terminal.size()?;
                    let area = Rect::new(0, 0, size.width, size.height);
                    if handle_mouse(
                        mouse,
                        area,
                        &mut cfg,
                        &mut active_resize,
                        &mut writer,
                        &config_path,
                    )? {
                        exit = true;
                        break;
                    }
                    let inner = box_inner(area, &cfg);
                    let _ = master.resize(pty_size(inner));
                    parser.lock().unwrap().set_size(inner.height, inner.width);
                }
                Event::Resize(width, height) => {
                    let inner = box_inner(Rect::new(0, 0, width, height), &cfg);
                    let _ = master.resize(pty_size(inner));
                    parser.lock().unwrap().set_size(inner.height, inner.width);
                }
                Event::FocusGained | Event::FocusLost => {}
            }
        }

        while let Ok(event) = rx.try_recv() {
            if matches!(event, PaneEvent::Exit) {
                exit = true;
            }
        }
    }

    restore_terminal();
    Ok(())
}

fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Ok(())
}

fn restore_terminal() {
    let _ = execute!(std::io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

fn handle_mouse(
    mouse: MouseEvent,
    area: Rect,
    cfg: &mut AppConfig,
    active_resize: &mut Option<ActiveResize>,
    writer: &mut Box<dyn Write + Send>,
    config_path: &Path,
) -> Result<bool> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let zone = hit_test(area, cfg, mouse.column, mouse.row);
            if zone == HitZone::Minimize {
                return Ok(true);
            }
            if zone.is_resize() {
                *active_resize = Some(ActiveResize {
                    zone,
                    start_x: mouse.column,
                    start_y: mouse.row,
                    start_cfg: cfg.clone(),
                });
                return Ok(false);
            }
            forward_mouse_if_inner(mouse, area, cfg, writer)?;
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(active) = active_resize.as_ref() {
                *cfg = resize_from_drag(
                    area,
                    &active.start_cfg,
                    active.zone,
                    active.start_x,
                    active.start_y,
                    mouse.column,
                    mouse.row,
                );
                save_config(config_path, cfg)?;
            } else {
                forward_mouse_if_inner(mouse, area, cfg, writer)?;
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            *active_resize = None;
            forward_mouse_if_inner(mouse, area, cfg, writer)?;
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            forward_mouse_if_inner(mouse, area, cfg, writer)?;
        }
        _ => {}
    }
    Ok(false)
}

fn forward_mouse_if_inner(
    mouse: MouseEvent,
    area: Rect,
    cfg: &AppConfig,
    writer: &mut Box<dyn Write + Send>,
) -> Result<()> {
    if !cfg.forward_inner_mouse {
        return Ok(());
    }
    if let Some(bytes) = mouse_to_sgr(mouse, box_inner(area, cfg)) {
        writer.write_all(&bytes)?;
        writer.flush()?;
    }
    Ok(())
}

fn mouse_to_sgr(mouse: MouseEvent, inner: Rect) -> Option<Vec<u8>> {
    if !inner.contains(mouse.column, mouse.row) {
        return None;
    }
    let x = mouse.column - inner.x + 1;
    let y = mouse.row - inner.y + 1;
    let (button, suffix) = match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => (0, 'M'),
        MouseEventKind::Down(MouseButton::Middle) => (1, 'M'),
        MouseEventKind::Down(MouseButton::Right) => (2, 'M'),
        MouseEventKind::Drag(MouseButton::Left) => (32, 'M'),
        MouseEventKind::Up(MouseButton::Left)
        | MouseEventKind::Up(MouseButton::Middle)
        | MouseEventKind::Up(MouseButton::Right) => (0, 'm'),
        MouseEventKind::ScrollUp => (64, 'M'),
        MouseEventKind::ScrollDown => (65, 'M'),
        _ => return None,
    };
    Some(format!("\x1b[<{button};{x};{y}{suffix}").into_bytes())
}

fn key_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    if key.modifiers.contains(KeyModifiers::ALT) {
        out.push(0x1b);
    }

    match key.code {
        KeyCode::Char(ch) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let upper = ch.to_ascii_uppercase();
            if upper.is_ascii_alphabetic() {
                out.push((upper as u8) - b'A' + 1);
            } else {
                out.extend(ch.to_string().as_bytes());
            }
        }
        KeyCode::Char(ch) => out.extend(ch.to_string().as_bytes()),
        KeyCode::Enter => out.push(b'\r'),
        KeyCode::Backspace => out.push(0x7f),
        KeyCode::Tab => out.push(b'\t'),
        KeyCode::Esc => out.push(0x1b),
        KeyCode::Left => out.extend(b"\x1b[D"),
        KeyCode::Right => out.extend(b"\x1b[C"),
        KeyCode::Up => out.extend(b"\x1b[A"),
        KeyCode::Down => out.extend(b"\x1b[B"),
        KeyCode::Home => out.extend(b"\x1b[H"),
        KeyCode::End => out.extend(b"\x1b[F"),
        KeyCode::PageUp => out.extend(b"\x1b[5~"),
        KeyCode::PageDown => out.extend(b"\x1b[6~"),
        KeyCode::Delete => out.extend(b"\x1b[3~"),
        KeyCode::Insert => out.extend(b"\x1b[2~"),
        KeyCode::F(n) => out.extend(function_key(n).as_bytes()),
        KeyCode::Null
        | KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::KeypadBegin
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => return None,
        KeyCode::BackTab => out.extend(b"\x1b[Z"),
    }
    Some(out)
}

fn function_key(n: u8) -> String {
    match n {
        1 => "\x1bOP".into(),
        2 => "\x1bOQ".into(),
        3 => "\x1bOR".into(),
        4 => "\x1bOS".into(),
        5..=12 => format!("\x1b[{}~", n + 10),
        _ => String::new(),
    }
}

fn draw(frame: &mut Frame, cfg: &AppConfig, screen: &vt100::Screen) {
    let area = frame.area();
    let area_rect = Rect::new(area.x, area.y, area.width, area.height);
    let outer = box_rect(area_rect, cfg);
    let inner = box_inner(area_rect, cfg);

    let Rgb(br, bg, bb) = cfg.backdrop;
    frame.render_widget(
        Block::default().style(
            Style::default()
                .bg(Color::Rgb(br, bg, bb))
                .fg(Color::DarkGray),
        ),
        area,
    );

    let outer_area = ratatui_rect(outer);
    frame.render_widget(Clear, outer_area);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Line::from(" herdr-floating-pane ").centered())
        .title(Line::from(" [-] ").right_aligned())
        .title_bottom(
            Line::from(format!(
                " {} / {} hides ",
                cfg.key_hint_workspace, cfg.key_hint_session
            ))
            .centered()
            .style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(block, outer_area);

    render_screen(frame.buffer_mut(), ratatui_rect(inner), screen);

    if !screen.hide_cursor() {
        let (row, col) = screen.cursor_position();
        if row < inner.height && col < inner.width {
            frame.set_cursor_position(Position::new(inner.x + col, inner.y + row));
        }
    }
}

fn render_screen(buffer: &mut Buffer, area: ratatui::layout::Rect, screen: &vt100::Screen) {
    let (rows, cols) = screen.size();
    for row in 0..rows.min(area.height) {
        let mut skip_next = false;
        for col in 0..cols.min(area.width) {
            if skip_next {
                skip_next = false;
                continue;
            }
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            let Some(target) = buffer.cell_mut(Position::new(area.x + col, area.y + row)) else {
                continue;
            };
            let contents = cell.contents();
            target.set_symbol(if contents.is_empty() { " " } else { &contents });
            let mut style = Style::default()
                .fg(vt_color(cell.fgcolor()))
                .bg(vt_color(cell.bgcolor()));
            if cell.bold() {
                style = style.add_modifier(Modifier::BOLD);
            }
            if cell.italic() {
                style = style.add_modifier(Modifier::ITALIC);
            }
            if cell.underline() {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            if cell.inverse() {
                style = style.add_modifier(Modifier::REVERSED);
            }
            target.set_style(style);
            skip_next = cell.is_wide();
        }
    }
}

fn vt_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(index) => Color::Indexed(index),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn ratatui_rect(rect: Rect) -> ratatui::layout::Rect {
    ratatui::layout::Rect::new(rect.x, rect.y, rect.width, rect.height)
}

fn pty_size(rect: Rect) -> PtySize {
    PtySize {
        rows: rect.height,
        cols: rect.width,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn pct(size: i32, total: u16, min_pct: u16) -> u16 {
    if total == 0 {
        return min_pct;
    }
    let pct = ((size.max(1) * 100) / i32::from(total)) as u16;
    pct.clamp(min_pct, 100)
}

fn command_exists(program: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {program} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn config_path() -> PathBuf {
    if let Ok(dir) = std::env::var("HERDR_PLUGIN_CONFIG_DIR") {
        return PathBuf::from(dir).join("floating-pane.toml");
    }
    let home = std::env::var_os("HOME").unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(".config/herdr/plugins/config/herdr-floating-pane/floating-pane.toml")
}

fn state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HERDR_PLUGIN_STATE_DIR") {
        return PathBuf::from(dir);
    }
    std::env::temp_dir().join("herdr-floating-pane")
}

fn load_config(path: &Path) -> Result<AppConfig> {
    match std::fs::read_to_string(path) {
        Ok(text) => AppConfig::from_toml(&text).context("failed to parse floating-pane.toml"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn save_config(path: &Path, cfg: &AppConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, cfg.to_toml()?)?;
    Ok(())
}
