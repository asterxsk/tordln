mod config;
mod engine;
mod modal;
mod pickup;
mod ui;

use std::sync::Arc;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use ratatui::widgets::Clear;
use tokio::sync::mpsc;

use engine::Engine;
use pickup::{is_torrent_file, is_torrent_source, Pickup, spawn_clipboard, spawn_folder_watch};
use ui::{App, Focus, SidebarMode};

/// A pending source waiting for the add-modal.
enum Pending {
    Url(String),
    File(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    let settings = config::load();
    let engine = Arc::new(Engine::new(&settings.download_dir).await?);

    let (tx, mut rx) = mpsc::channel::<Pickup>(32);

    if settings.clipboard_watch {
        spawn_clipboard(tx.clone());
    }
    if let Some(folder) = &settings.watch_folder {
        spawn_folder_watch(folder.clone(), tx.clone());
    }

    // Drag-drop: a .torrent path passed as first CLI arg -> straight to modal.
    let mut pending: Option<Pending> = None;
    if let Some(path) = std::env::args().nth(1) {
        if is_torrent_file(std::path::Path::new(&path)) {
            pending = Some(Pending::File(path));
        }
    }

    let mut app = App::new(settings);

    // Boot: scan download_dir for .torrent files -> Home popup.
    app.home_found_files = scan_dir_for_torrents(&app.settings.download_dir);

    // Boot: if clipboard_watch and a magnet/.torrent URL is on the clipboard,
    // auto-paste it into the pre-download modal.
    if app.settings.clipboard_watch {
        if let Some(link) = read_clipboard_link() {
            if is_torrent_source(&link) {
                pending = Some(Pending::Url(link.trim().to_string()));
            }
        }
    }

    let mut term = ui::init_terminal()?;

    // Active add-modal + its source, if any.
    let mut add_modal: Option<(modal::AddModal, Pending)> = None;

    loop {
        // Refresh torrent lists from engine (auto-move finished by progress).
        let all = engine.list();
        app.torrents = all
            .iter()
            .filter(|t| t.progress < 1.0)
            .cloned()
            .collect();
        app.finished = all
            .iter()
            .filter(|t| t.progress >= 1.0)
            .cloned()
            .collect();
        app.torrents.retain(|t| t.state != "Finished");

        // Refresh detail pane for the selected torrent in the current view.
        if app.sidebar_mode != SidebarMode::Settings {
            if let Some(t) = current_selected(&app) {
                app.details = engine.details(t.id);
            } else {
                app.details = None;
            }
        }

        // Open modal from a pending source when none is active.
        if add_modal.is_none() {
            if let Some(p) = pending.take() {
                let m = modal::AddModal::new(&app.settings.download_dir, Vec::new());
                add_modal = Some((m, p));
            }
        }

        // Drain pickup events into pending (only when no modal open).
        if add_modal.is_none() {
            while let Ok(p) = rx.try_recv() {
                match p {
                    Pickup::Status(_s) => {}
                    Pickup::Url(u) if is_torrent_source(&u) => {
                        if app.settings.clipboard_watch {
                            pending = Some(Pending::Url(u.trim().to_string()));
                            break;
                        }
                    }
                    Pickup::File(f) => {
                        // Only auto-add if file is under the configured watch folder.
                        if let Some(wf) = &app.settings.watch_folder {
                            if std::path::Path::new(&f).starts_with(wf) {
                                pending = Some(Pending::File(f));
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        term.draw(|f| {
            app.draw(f);
            if let Some((m, _)) = &add_modal {
                f.render_widget(Clear, f.area());
                m.draw(f, f.area());
            }
        })?;

        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) => {
                if let Some((m, _src)) = add_modal.as_mut() {
                    m.handle_key(key);
                    if m.confirmed {
                        // Take ownership of the modal+source so we can move src.
                        let (m, src) = add_modal.take().unwrap();
                        // Apply download path (global session dir used for v1).
                        let _ = &m.path;
                        // TODO: per-torrent speed limit + file selection need
                        // librqbit options not exposed publicly yet.
                        let res = match &src {
                            Pending::Url(u) => engine.add_url(u).await,
                            Pending::File(fl) => engine.add_file(fl).await,
                        };
                        let _ = res; // result surfaced via torrent list / errors
                    } else if m.cancelled {
                        add_modal = None;
                    }
                    continue;
                }
                handle_app_key(&mut app, key, &engine, &mut pending).await;
            }
            Event::Mouse(me) => {
                if add_modal.is_some() {
                    continue;
                }
                if let MouseEventKind::Down(_) = me.kind {
                    if me.column < 20 {
                        app.focus = Focus::Sidebar;
                        app.sidebar_mode = if me.row < 6 {
                            SidebarMode::Active
                        } else if me.row < 11 {
                            SidebarMode::Finished
                        } else {
                            SidebarMode::Settings
                        };
                    } else {
                        app.focus = Focus::Right;
                        let row = me.row.saturating_sub(4) as usize;
                        if row < current_list_len(&app) {
                            app.selected = row;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn current_selected(app: &App) -> Option<engine::TorrentInfo> {
    let list = match app.sidebar_mode {
        SidebarMode::Home => return None,
        SidebarMode::Active => &app.torrents,
        SidebarMode::Finished => &app.finished,
        SidebarMode::Settings => return None,
    };
    list.get(app.selected).cloned()
}

fn current_list_len(app: &App) -> usize {
    match app.sidebar_mode {
        SidebarMode::Active => app.torrents.len(),
        SidebarMode::Finished => app.finished.len(),
        SidebarMode::Settings => 0,
        SidebarMode::Home => 0,
    }
}

/// Scan a directory for `.torrent` files; return their (stripped) names.
fn scan_dir_for_torrents(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "torrent").unwrap_or(false) {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    out.push(name.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

/// Read the clipboard; return its text if available.
fn read_clipboard_link() -> Option<String> {
    match arboard::Clipboard::new() {
        Ok(mut c) => c.get_text().ok(),
        Err(_) => None,
    }
}

async fn handle_app_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    engine: &Engine,
    pending: &mut Option<Pending>,
) {
    match key.code {
        KeyCode::Char('q') => std::process::exit(0),
        KeyCode::Char('1') => {
            app.sidebar_mode = SidebarMode::Home;
            app.selected = 0;
        }
        KeyCode::Char('2') => {
            app.sidebar_mode = SidebarMode::Active;
            app.selected = 0;
        }
        KeyCode::Char('3') => {
            app.sidebar_mode = SidebarMode::Finished;
            app.selected = 0;
        }
        KeyCode::Char('4') => app.sidebar_mode = SidebarMode::Settings,
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Sidebar => Focus::Right,
                Focus::Right => Focus::Sidebar,
            };
        }
        KeyCode::Up => match app.focus {
            Focus::Sidebar => {
                app.sidebar_mode = match app.sidebar_mode {
                    SidebarMode::Home => SidebarMode::Settings,
                    SidebarMode::Active => SidebarMode::Home,
                    SidebarMode::Finished => SidebarMode::Active,
                    SidebarMode::Settings => SidebarMode::Finished,
                };
            }
            Focus::Right => {
                if app.sidebar_mode == SidebarMode::Home && !app.home_found_files.is_empty() {
                    if app.home_file_focus > 0 {
                        app.home_file_focus -= 1;
                    }
                } else if app.selected > 0 {
                    app.selected -= 1;
                }
            }
        },
        KeyCode::Down => match app.focus {
            Focus::Sidebar => {
                app.sidebar_mode = match app.sidebar_mode {
                    SidebarMode::Home => SidebarMode::Active,
                    SidebarMode::Active => SidebarMode::Finished,
                    SidebarMode::Finished => SidebarMode::Settings,
                    SidebarMode::Settings => SidebarMode::Home,
                };
            }
            Focus::Right => {
                if app.sidebar_mode == SidebarMode::Home && !app.home_found_files.is_empty() {
                    if app.home_file_focus + 1 < app.home_found_files.len() {
                        app.home_file_focus += 1;
                    }
                } else if app.selected + 1 < current_list_len(app) {
                    app.selected += 1;
                }
            }
        },
        // Home: open pre-download modal for the highlighted found file.
        KeyCode::Enter if app.sidebar_mode == SidebarMode::Home && !app.home_found_files.is_empty() => {
            let name = app.home_found_files[app.home_file_focus].clone();
            let path = format!("{}/{}", app.settings.download_dir, name);
            *pending = Some(Pending::File(path));
        }
        // Home: paste magnet from clipboard.
        KeyCode::Char('p') | KeyCode::Char('P') if app.sidebar_mode == SidebarMode::Home => {
            if let Some(link) = read_clipboard_link() {
                if is_torrent_source(&link) {
                    *pending = Some(Pending::Url(link.trim().to_string()));
                }
                // (feedback previously shown in the corner status bar)
            }
        }
        // Home: new download — open the pre-download modal with the clipboard
        // link (copy a magnet first, then press N). The modal shows the path
        // + file picker; Enter starts it.
        KeyCode::Char('n') | KeyCode::Char('N') if app.sidebar_mode == SidebarMode::Home => {
            if let Some(link) = read_clipboard_link() {
                if is_torrent_source(&link) {
                    *pending = Some(Pending::Url(link.trim().to_string()));
                }
            }
        }
        // Active row actions.
        KeyCode::Char('r') if app.sidebar_mode == SidebarMode::Active => {
            // Remove from list (keep files): delete with delete_files=false.
            if let Some(t) = current_selected(app) {
                let _ = engine
                    .session
                    .delete(librqbit::api::TorrentIdOrHash::Id(t.id), false)
                    .await;
            }
        }
        KeyCode::Char('d') if app.sidebar_mode == SidebarMode::Active => {
            // Delete completely (remove files).
            if let Some(t) = current_selected(app) {
                let _ = engine
                    .session
                    .delete(librqbit::api::TorrentIdOrHash::Id(t.id), true)
                    .await;
            }
        }
        // Finished row actions.
        KeyCode::Char('d') if app.sidebar_mode == SidebarMode::Finished => {
            if let Some(t) = current_selected(app) {
                let _ = engine
                    .session
                    .delete(librqbit::api::TorrentIdOrHash::Id(t.id), true)
                    .await;
            }
        }
        KeyCode::Char('t') if app.sidebar_mode == SidebarMode::Finished => {
            // Delete .torrent metadata only (keep files): remove from list.
            if let Some(t) = current_selected(app) {
                let _ = engine
                    .session
                    .delete(librqbit::api::TorrentIdOrHash::Id(t.id), false)
                    .await;
            }
        }
        // Settings editing (only when Settings mode + Right focus).
        KeyCode::Char(' ') if app.sidebar_mode == SidebarMode::Settings && app.focus == Focus::Right && app.settings_editing.is_none() => {
            if app.settings_focus == 2 {
                app.settings.clipboard_watch = !app.settings.clipboard_watch;
                let _ = config::save(&app.settings);
            } else if app.settings_focus == 1 {
                // Toggle watch folder on/off: clear or set to default Downloads/tordln-watch.
                if app.settings.watch_folder.is_some() {
                    app.settings.watch_folder = None;
                } else {
                    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".into());
                    app.settings.watch_folder = Some(format!("{home}/Downloads/tordln-watch"));
                }
                let _ = config::save(&app.settings);
            }
        }
        KeyCode::Char('e') if app.sidebar_mode == SidebarMode::Settings && app.focus == Focus::Right && app.settings_editing.is_none() => {
            // Enter edit mode for the focused text row (0,1,3 are text; 2 is bool).
            if app.settings_focus != 2 {
                app.settings_editing = Some(app.settings_focus);
            }
        }
        KeyCode::Enter if app.sidebar_mode == SidebarMode::Settings && app.focus == Focus::Right => {
            if let Some(row) = app.settings_editing {
                // Commit textual edit.
                if row == 3 {
                    // Normalize speed limit: parse, clamp 1..=10, or None if empty.
                    let v = app.settings.global_speed_limit;
                    app.settings.global_speed_limit = v;
                }
                app.settings_editing = None;
            }
            let _ = config::save(&app.settings);
        }
        KeyCode::Esc if app.sidebar_mode == SidebarMode::Settings => {
            app.settings_editing = None;
        }
        KeyCode::Backspace if app.sidebar_mode == SidebarMode::Settings && app.settings_editing.is_some() => {
            match app.settings_editing {
                Some(0) => { app.settings.download_dir.pop(); }
                Some(1) => { if let Some(w) = &mut app.settings.watch_folder { w.pop(); } }
                Some(3) => {
                    let s = app.settings.global_speed_limit.map(|v| v.to_string()).unwrap_or_default();
                    let s = if s.len() <= 1 { String::new() } else { s[..s.len()-1].into() };
                    app.settings.global_speed_limit = s.parse::<u32>().ok().filter(|v| (1..=10).contains(v));
                }
                _ => {}
            }
        }
        KeyCode::Char(c) if app.sidebar_mode == SidebarMode::Settings && app.settings_editing.is_some() => {
            match app.settings_editing {
                Some(0) => app.settings.download_dir.push(c),
                Some(1) => { if let Some(w) = &mut app.settings.watch_folder { w.push(c); } }
                Some(3) if c.is_ascii_digit() => {
                    let mut s = app.settings.global_speed_limit.map(|v| v.to_string()).unwrap_or_default();
                    s.push(c);
                    if let Ok(v) = s.parse::<u32>() {
                        if (1..=10).contains(&v) { app.settings.global_speed_limit = Some(v); }
                        else if v == 0 { app.settings.global_speed_limit = Some(1); }
                    }
                }
                _ => {}
            }
        }
        KeyCode::Up if app.sidebar_mode == SidebarMode::Settings && app.focus == Focus::Right && app.settings_editing.is_none() => {
            if app.settings_focus > 0 { app.settings_focus -= 1; }
        }
        KeyCode::Down if app.sidebar_mode == SidebarMode::Settings && app.focus == Focus::Right && app.settings_editing.is_none() => {
            if app.settings_focus + 1 < 4 { app.settings_focus += 1; }
        }
        // File toggle in detail pane (real librqbit file selection).
        KeyCode::Char(' ') if app.sidebar_mode == SidebarMode::Active => {
            if let Some(t) = current_selected(app) {
                let idx = app.detail_file_focus;
                let _ = engine.set_file_selected(t.id, idx, true).await;
            }
        }
        _ => {}
    }
}
