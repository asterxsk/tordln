use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;

use crate::config::Settings;
use crate::engine::{TorrentDetails, TorrentInfo};

/// Which sidebar mode is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    Home,
    Active,
    Finished,
    Settings,
}

/// Which pane currently holds keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Right,
}

pub struct App {
    pub settings: Settings,
    pub torrents: Vec<TorrentInfo>,
    pub finished: Vec<TorrentInfo>,
    pub selected: usize,
    pub sidebar_mode: SidebarMode,
    pub focus: Focus,
    pub status: String,
    pub details: Option<TorrentDetails>,
    pub detail_file_focus: usize,
    pub settings_focus: usize,
    pub settings_editing: Option<usize>,
    /// .torrent files discovered in download_dir (shown as a popup on Home).
    pub home_found_files: Vec<String>,
    /// Selected index within `home_found_files` popup.
    pub home_file_focus: usize,
}

impl App {
    pub fn new(settings: Settings) -> Self {
        App {
            settings,
            torrents: vec![],
            finished: vec![],
            selected: 0,
            sidebar_mode: SidebarMode::Home,
            focus: Focus::Right,
            status: "ready".into(),
            details: None,
            detail_file_focus: 0,
            settings_focus: 0,
            settings_editing: None,
            home_found_files: vec![],
            home_file_focus: 0,
        }
    }

    /// Returns the list of torrents for the current mode and its length.
    fn current_list(&self) -> &[TorrentInfo] {
        match self.sidebar_mode {
            SidebarMode::Home => &self.torrents,
            SidebarMode::Active => &self.torrents,
            SidebarMode::Finished => &self.finished,
            SidebarMode::Settings => &self.torrents,
        }
    }

    pub fn draw(&mut self, f: &mut Frame) {
        // Tokens from tokens.css / design.md.
        let paper = Color::Rgb(10, 10, 12);
        let text = Color::Rgb(235, 235, 240);
        let dim = Color::Rgb(140, 140, 150);
        let accent = Color::Rgb(160, 120, 255);
        let border = Style::default().fg(Color::Rgb(50, 50, 60));
        let _error = Color::Rgb(200, 80, 80);
        let _ = paper;

        // One outer frame around the whole screen. Every inner pane then draws
        // ONLY its internal divider edges, so there are no doubled/stacked lines.
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border)
            .title(Span::styled(
                " tordln · TORRENT DOWNLOAD MANAGER ",
                Style::default().fg(text),
            ));
        let area = outer_block.inner(f.area());
        f.render_widget(outer_block, f.area());

        // Inside the frame: header / body / status.
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Min(5),    // body
                Constraint::Length(1), // status
            ])
            .split(area);

        // Header: title text only (no block — outer frame supplies the edge).
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  paste a magnet · drop a .torrent · or set a watch folder",
                Style::default().fg(dim),
            ))),
            layout[0],
        );

        // Body horizontal: sidebar / right.
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(16), // sidebar
                Constraint::Min(40),    // right
            ])
            .split(layout[1]);

        match self.sidebar_mode {
            SidebarMode::Home => {
                self.draw_home(f, body[1], accent, text, dim, border);
            }
            SidebarMode::Settings => {
                // Settings fills the whole right column; no detail pane.
                self.draw_top(f, body[1], accent, text, dim, border);
            }
            _ => {
                // Right split vertical: top list / bottom detail.
                let right = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(60), // top list
                        Constraint::Length(9),      // bottom detail
                    ])
                    .split(body[1]);

                self.draw_top(f, right[0], accent, text, dim, border);
                self.draw_detail(f, right[1], accent, text, dim, border);
            }
        }
        // Sidebar is drawn for every non-Home mode; on Home it's hidden
        // (the Home screen has its own nav hint).
        if self.sidebar_mode != SidebarMode::Home {
            self.draw_sidebar(f, body[0], accent, dim, border);
        }

        // Status: single text line (outer frame supplies the bottom edge).
        f.render_widget(
            Paragraph::new(Line::from(Span::raw(&self.status))),
            layout[2],
        );

        // Home: popup listing discovered .torrent files in download_dir.
        if self.sidebar_mode == SidebarMode::Home && !self.home_found_files.is_empty() {
            self.draw_home_files_popup(f, accent, text, dim, border);
        }
    }

    /// Home screen: TORDLN ASCII banner + quick actions.
    fn draw_home(
        &self,
        f: &mut Frame,
        area: Rect,
        accent: Color,
        text: Color,
        dim: Color,
        border: Style,
    ) {
        let banner = [
            " _____  ____  ____  _   _  _   _  _  _  ",
            "|_   _||  _ \\|  _ \\| | | || \\| || \\| | ",
            "  | |  | |_) | | | | | | ||  \\  |  \\  | ",
            "  | |  |  _ <| |_| | |_| || |\\  | |\\  | ",
            "  |_|  |_| \\_\\\\____|\\___/ |_| \\_|_| \\_| ",
        ];
        let inner = area;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // banner
                Constraint::Min(4),    // spacer / actions
                Constraint::Length(7), // actions block
            ])
            .split(inner);

        // Banner (purple, centered).
        let banner_lines: Vec<Line> = banner
            .iter()
            .map(|l| {
                Line::from(Span::styled(
                    *l,
                    Style::default().fg(accent),
                ))
                .alignment(ratatui::layout::Alignment::Center)
            })
            .collect();
        f.render_widget(
            Paragraph::new(banner_lines).alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );

        // Tagline + hints.
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "terminal torrent download manager",
                Style::default().fg(dim),
            )))
            .alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );

        // Quick actions box.
        let actions = vec![
            Line::from(Span::styled(
                " [P] paste magnet from clipboard ",
                Style::default().fg(text),
            )),
            Line::from(Span::styled(
                " [N] new download (paste link / path) ",
                Style::default().fg(text),
            )),
            Line::from(Span::styled(
                " drop a .torrent file or paste a magnet to begin ",
                Style::default().fg(dim),
            )),
            Line::from(Span::styled(
                " 1 Home · 2 Active · 3 Finished · 4 Settings ",
                Style::default().fg(dim),
            )),
        ];
        f.render_widget(
            Paragraph::new(actions)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border)
                        .title(" quick actions "),
                )
                .alignment(ratatui::layout::Alignment::Center),
            chunks[2],
        );
    }

    /// Popup listing .torrent files found in download_dir (Home only).
    fn draw_home_files_popup(
        &self,
        f: &mut Frame,
        accent: Color,
        text: Color,
        dim: Color,
        border: Style,
    ) {
        // Center a box ~50% width, ~60% height.
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Percentage(70),
                Constraint::Percentage(15),
            ])
            .split(f.area())[1];
        let box_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Percentage(70),
                Constraint::Percentage(15),
            ])
            .split(outer)[1];

        f.render_widget(Clear, box_area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border)
            .title(Span::styled(" found .torrent files ", Style::default().fg(accent)));
        let inner = block.inner(box_area);
        f.render_widget(block, box_area);

        let items: Vec<ListItem> = self
            .home_found_files
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let focused = i == self.home_file_focus;
                let mark = if focused { ">" } else { " " };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", mark),
                        Style::default().fg(if focused { accent } else { dim }),
                    ),
                    Span::styled(
                        name.clone(),
                        Style::default().fg(if focused { text } else { dim }),
                    ),
                ]))
            })
            .collect();
        let mut state = ListState::default();
        state.select(Some(self.home_file_focus));
        f.render_stateful_widget(
            List::new(items).highlight_style(Style::default().bg(Color::Rgb(30, 30, 45))),
            inner,
            &mut state,
        );
    }

    fn draw_sidebar(
        &self,
        f: &mut Frame,
        area: Rect,
        accent: Color,
        dim: Color,
        border: Style,
    ) {
        let items = [
            (SidebarMode::Home, "Home"),
            (SidebarMode::Active, "Active"),
            (SidebarMode::Finished, "Finished"),
            (SidebarMode::Settings, "Settings"),
        ];
        let focused = self.focus == Focus::Sidebar;
        let list_items: Vec<ListItem> = items
            .iter()
            .map(|(mode, label)| {
                let is_current = *mode == self.sidebar_mode;
                let style = if is_current {
                    Style::default()
                        .fg(accent)
                        .add_modifier(Modifier::BOLD)
                } else if focused {
                    Style::default().fg(dim)
                } else {
                    Style::default().fg(dim)
                };
                let prefix = if is_current { ">" } else { " " };
                ListItem::new(Line::from(vec![Span::styled(
                    format!("{prefix} {label}"),
                    style,
                )]))
            })
            .collect();

        // Only the internal vertical divider (right edge); the outer frame
        // supplies the top/bottom/left edges.
        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::RIGHT)
                    .border_style(border)
                    .title(" modes "),
            )
            .highlight_symbol("");
        let mut state = ListState::default();
        let current_idx = match self.sidebar_mode {
            SidebarMode::Home => 0,
            SidebarMode::Active => 1,
            SidebarMode::Finished => 2,
            SidebarMode::Settings => 3,
        };
        state.select(Some(current_idx));
        f.render_stateful_widget(list, area, &mut state);
    }

    fn draw_top(
        &self,
        f: &mut Frame,
        area: Rect,
        accent: Color,
        text: Color,
        dim: Color,
        border: Style,
    ) {
        let right_border = border;

        if self.sidebar_mode == SidebarMode::Settings {
            let s = &self.settings;
            let watch = match &s.watch_folder {
                Some(w) => format!("on ({w})"),
                None => "off".to_string(),
            };
            let lim = match s.global_speed_limit {
                Some(v) => format!("{v} MB/s"),
                None => "off".to_string(),
            };
            // 4 navigable rows: 0 download_dir, 1 watch_folder, 2 clipboard_watch, 3 global_speed_limit
            let rows: [(String, String); 4] = [
                ("download_dir".into(), s.download_dir.clone()),
                ("watch_folder".into(), watch),
                ("clipboard_watch".into(), if s.clipboard_watch { "on" } else { "off" }.into()),
                ("speed_limit".into(), lim),
            ];
            let lines: Vec<Line> = rows
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    let is_focus = self.focus == Focus::Right && i == self.settings_focus;
                    let style = if is_focus {
                        Style::default().fg(accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(text)
                    };
                    let mark = if is_focus { ">" } else { " " };
                    Line::from(vec![
                        Span::styled(format!("{mark} "), style),
                        Span::styled(format!("{k:<16}"), Style::default().fg(dim)),
                        Span::styled(v.clone(), style),
                    ])
                })
                .collect();
            let mut all = vec![Line::from(Span::styled(
                "Settings  (space=toggle, type=edit, enter=save)",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ))];
            all.push(Line::from(Span::raw("")));
            all.extend(lines);
            let block = Block::default()
                .borders(Borders::NONE)
                .border_style(right_border)
                .title(" settings ");
            f.render_widget(Paragraph::new(all).block(block), area);
            return;
        }

        let list = self.current_list();
        let block = Block::default()
            .borders(Borders::NONE)
            .border_style(right_border)
            .title(if self.sidebar_mode == SidebarMode::Finished {
                " finished "
            } else {
                " queue "
            });

        if list.is_empty() {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "no torrents",
                    Style::default().fg(dim),
                )))
                .block(block),
                area,
            );
            return;
        }

        let rows: Vec<Line> = list
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let pct = (t.progress * 100.0) as u32;
                let bar = shade_bar(t.progress, 8);
                let is_selected = i == self.selected;
                let state_label = if self.sidebar_mode == SidebarMode::Finished {
                    "done"
                } else {
                    "live"
                };
                let row_style = if is_selected {
                    Style::default()
                        .fg(accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(text)
                };
                let hint = if self.sidebar_mode == SidebarMode::Finished {
                    "[Remove|Del|.torrent]"
                } else {
                    "[Remove|Del]"
                };
                Line::from(vec![
                    Span::styled(format!("{:>2} ", i + 1), row_style),
                    Span::styled(trim(&t.name, 24), row_style),
                    Span::raw(" "),
                    Span::styled(format!("[{}]", bar), row_style),
                    Span::raw(" "),
                    Span::styled(format!("{pct:>3}%", ), row_style),
                    Span::raw(" "),
                    Span::styled(state_label, Style::default().fg(dim)),
                    Span::raw(" "),
                    Span::styled(hint, Style::default().fg(dim)),
                ])
            })
            .collect();

        f.render_widget(Paragraph::new(rows).block(block), area);
    }

    fn draw_detail(
        &self,
        f: &mut Frame,
        area: Rect,
        accent: Color,
        text: Color,
        dim: Color,
        border: Style,
    ) {
        // Only the internal horizontal divider (top edge) between the queue and
        // the detail pane; the outer frame supplies the bottom/right edges.
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(border)
            .title(" detail ");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let Some(details) = &self.details else {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        "no torrent selected",
                        Style::default().fg(dim),
                    )),
                    Line::from(Span::styled(
                        "add one: paste a magnet link, drop a .torrent file,",
                        Style::default().fg(dim),
                    )),
                    Line::from(Span::styled(
                        "or configure a watch folder in Settings",
                        Style::default().fg(dim),
                    )),
                ]),
                inner,
            );
            return;
        };

        let mut lines: Vec<Line> = Vec::new();

        // Header: name / seeders / leechers / progress.
        lines.push(Line::from(vec![
            Span::styled(
                trim(&details.name, 40),
                Style::default()
                    .fg(accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("seeders ", Style::default().fg(dim)),
            Span::styled(format!("{}", details.seeders), Style::default().fg(text)),
            Span::styled("  leechers ", Style::default().fg(dim)),
            Span::styled(format!("{}", details.leechers), Style::default().fg(text)),
            Span::styled("  progress ", Style::default().fg(dim)),
            Span::styled(
                format!("{:.0}%", details.progress * 100.0),
                Style::default().fg(text),
            ),
        ]));

        // File checklist.
        for (i, file) in details.files.iter().enumerate() {
            let checked = file.selected;
            let mark = if checked { "x" } else { " " };
            let focused_file = self.focus == Focus::Right && i == self.detail_file_focus;
            let style = if focused_file {
                Style::default()
                    .fg(accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(text)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("[{}] ", mark), style),
                Span::styled(trim(&file.name, 46), style),
            ]));
        }

        // Piece map: wrapped grid of solid cells, ~40 wide.
        if details.piece_map.is_empty() {
            lines.push(Line::from(Span::styled(
                "no selection",
                Style::default().fg(dim),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "pieces:",
                Style::default().fg(dim),
            )));
            const WIDTH: usize = 40;
            let mut cur: Vec<Span> = Vec::with_capacity(WIDTH);
            for (idx, done) in details.piece_map.iter().enumerate() {
                let span = Span::styled(
                    "█",
                    Style::default().fg(if *done { accent } else { dim }),
                );
                cur.push(span);
                if cur.len() >= WIDTH || idx + 1 == details.piece_map.len() {
                    lines.push(Line::from(cur.clone()));
                    cur.clear();
                }
            }
        }

        f.render_widget(Paragraph::new(lines), inner);
    }
}

/// Build an inline ASCII shade bar of length `width` for `progress` in [0,1].
fn shade_bar(progress: f32, width: usize) -> String {
    const FULL: char = '█';
    const SHADE: char = '▓';
    const EMPTY: char = '░';
    let p = progress.clamp(0.0, 1.0);
    let filled = (p * width as f32).round() as usize;
    let mut s = String::new();
    for i in 0..width {
        if i < filled {
            s.push(FULL);
        } else if i == filled && p > 0.0 && p < 1.0 && filled < width {
            s.push(SHADE);
        } else {
            s.push(EMPTY);
        }
    }
    s
}

/// Trim a string to `max` chars, appending "…" if truncated.
fn trim(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

pub type Term = Terminal<CrosstermBackend<io::Stdout>>;

pub fn init_terminal() -> anyhow::Result<Term> {
    crossterm::terminal::enable_raw_mode()?;
    let mut out = io::stdout();
    crossterm::execute!(out, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let term = Terminal::new(backend)?;
    Ok(term)
}

pub fn restore_terminal() -> anyhow::Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}
