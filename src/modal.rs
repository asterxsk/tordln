//! Add-flow modal widget.
//!
//! Pops before a torrent starts. Lets the user edit the download path,
//! toggle + tune a speed limit (checkbox + slider + free numeric box),
//! and pick which files to fetch. Confirm / Cancel to finish.
//!
//! Draws only inside the `area` given by the parent (which dims the
//! background and passes a centered rect). Does not own the terminal.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

// --- design tokens ---
const BORDER: Color = Color::Rgb(50, 50, 60);
const ACCENT: Color = Color::Rgb(160, 120, 255);
const TEXT: Color = Color::Rgb(235, 235, 240);
const DIM: Color = Color::Rgb(140, 140, 150);

/// Which control inside the modal currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalFocus {
    Path,
    LimitToggle,
    Slider,
    LimitValue,
    File(usize),
    Confirm,
    Cancel,
}

/// State + render + input handling for the add-flow modal.
pub struct AddModal {
    pub path: String,
    pub limit_enabled: bool,
    pub limit_value: u32,
    pub files: Vec<(String, bool)>,
    pub file_focus: usize,
    pub focus: ModalFocus,
    pub confirmed: bool,
    pub cancelled: bool,
}

impl AddModal {
    /// Build a fresh modal. All files pre-selected, limit off, value 5.
    pub fn new(default_path: &str, file_names: Vec<String>) -> Self {
        AddModal {
            path: default_path.to_string(),
            limit_enabled: false,
            limit_value: 5,
            files: file_names.into_iter().map(|n| (n, true)).collect(),
            file_focus: 0,
            focus: ModalFocus::Path,
            confirmed: false,
            cancelled: false,
        }
    }

    /// Indices of files the user chose to download.
    pub fn selected_files(&self) -> Vec<usize> {
        self.files
            .iter()
            .enumerate()
            .filter(|(_, (_, sel))| *sel)
            .map(|(i, _)| i)
            .collect()
    }

    /// Render the modal centered within `area`.
    pub fn draw(&self, f: &mut Frame, area: Rect) {
        // Cut a ~60% box out of the given area and center it.
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(area)[1];
        let box_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(outer)[1];

        // Wipe whatever is behind in this region (parent dimming is separate).
        f.render_widget(Clear, box_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER))
            .title(Span::styled(" add torrent ", Style::default().fg(ACCENT)));

        // Inner layout: path / limit / files / buttons.
        let inner = block.inner(box_area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // path
                Constraint::Length(3), // limit row
                Constraint::Min(3),    // files
                Constraint::Length(3), // buttons
            ])
            .split(inner);

        f.render_widget(block, box_area);

        // --- download path ---
        let path_style = if self.focus == ModalFocus::Path {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(TEXT)
        };
        let path_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.focus == ModalFocus::Path {
                Style::default().fg(ACCENT)
            } else {
                Style::default().fg(BORDER)
            })
            .title(" download path ");
        let path_p = Paragraph::new(self.path.as_str())
            .style(path_style)
            .block(path_block);
        f.render_widget(path_p, chunks[0]);

        // --- speed limit row: [checkbox] [slider] [numeric box] ---
        let limit_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(14), // checkbox
                Constraint::Min(16),    // slider
                Constraint::Length(8),  // numeric
            ])
            .split(chunks[1]);

        let toggle_box = if self.limit_enabled { "[x]" } else { "[ ]" };
        let toggle_style = if self.focus == ModalFocus::LimitToggle {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(TEXT)
        };
        let toggle_p = Paragraph::new(Line::from(vec![
            Span::styled(toggle_box, toggle_style),
            Span::styled(" Limit speed", Style::default().fg(TEXT)),
        ]));
        f.render_widget(toggle_p, limit_row[0]);

        // ASCII slider based on limit_value/10.
        let filled = self.limit_value.clamp(0, 10) as usize;
        let bar: String = std::iter::repeat('█').take(filled).collect::<String>()
            + &std::iter::repeat('░').take(10 - filled).collect::<String>();
        let slider_text = format!("[{bar}]");
        let slider_style = if self.focus == ModalFocus::Slider {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(TEXT)
        };
        let slider_p = Paragraph::new(Line::from(Span::styled(
            slider_text,
            slider_style,
        )));
        f.render_widget(slider_p, limit_row[1]);

        // Numeric box on slider's right.
        let num_style = if self.focus == ModalFocus::LimitValue {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(TEXT)
        };
        let num_box = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.focus == ModalFocus::LimitValue {
                Style::default().fg(ACCENT)
            } else {
                Style::default().fg(BORDER)
            })
            .title(" MB/s ");
        let num_p = Paragraph::new(format!("{}", self.limit_value)).style(num_style);
        f.render_widget(num_p.block(num_box), limit_row[2]);

        // --- file checklist ---
        let items: Vec<ListItem> = self
            .files
            .iter()
            .enumerate()
            .map(|(i, (name, sel))| {
                let mark = if *sel { "[x]" } else { "[ ]" };
                let focused = matches!(self.focus, ModalFocus::File(n) if n == i);
                let style = if focused {
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(mark, style),
                    Span::raw(" "),
                    Span::styled(name.clone(), style),
                ]))
            })
            .collect();
        let mut list_state = ListState::default();
        list_state.select(Some(self.file_focus));
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BORDER))
                    .title(" files "),
            )
            .highlight_style(Style::default().bg(Color::Rgb(30, 30, 45)));
        f.render_stateful_widget(list, chunks[2], &mut list_state);

        // --- confirm / cancel ---
        let btn_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[3]);
        let confirm_style = if self.focus == ModalFocus::Confirm {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM)
        };
        let cancel_style = if self.focus == ModalFocus::Cancel {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM)
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled("< Confirm >", confirm_style)))
                .alignment(ratatui::layout::Alignment::Center),
            btn_row[0],
        );
        f.render_widget(
            Paragraph::new(Line::from(Span::styled("< Cancel >", cancel_style)))
                .alignment(ratatui::layout::Alignment::Center),
            btn_row[1],
        );
    }

    /// Keyboard handling. Returns nothing; inspect `confirmed`/`cancelled`.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Tab | KeyCode::Down | KeyCode::Right => self.advance_focus(1),
            KeyCode::BackTab | KeyCode::Up | KeyCode::Left => self.advance_focus(-1),
            KeyCode::Char(' ') => self.toggle(),
            KeyCode::Enter => match self.focus {
                ModalFocus::Confirm => self.confirmed = true,
                ModalFocus::Cancel => self.cancelled = true,
                _ => {}
            },
            KeyCode::Esc => self.cancelled = true,
            KeyCode::Char(c) => self.type_char(c),
            KeyCode::Backspace => self.backspace(),
            _ => {}
        }
    }

    // Move focus forward/back through the control order.
    fn advance_focus(&mut self, dir: i32) {
        let order: Vec<ModalFocus> = {
            let mut v = vec![
                ModalFocus::Path,
                ModalFocus::LimitToggle,
                ModalFocus::Slider,
                ModalFocus::LimitValue,
            ];
            for i in 0..self.files.len() {
                v.push(ModalFocus::File(i));
            }
            v.push(ModalFocus::Confirm);
            v.push(ModalFocus::Cancel);
            v
        };
        let n = order.len() as i32;
        let cur = order
            .iter()
            .position(|m| *m == self.focus)
            .unwrap_or(0) as i32;
        let next = (cur + dir).rem_euclid(n);
        let next_focus = order[next as usize];
        self.focus = next_focus;
        if let ModalFocus::File(i) = next_focus {
            self.file_focus = i;
        }
    }

    // Space toggles the active checkbox/toggle.
    fn toggle(&mut self) {
        match self.focus {
            ModalFocus::LimitToggle => self.limit_enabled = !self.limit_enabled,
            ModalFocus::File(i) => {
                if let Some((_, sel)) = self.files.get_mut(i) {
                    *sel = !*sel;
                }
            }
            _ => {}
        }
    }

    // Insert a typed char into the focused text field.
    fn type_char(&mut self, c: char) {
        match self.focus {
            ModalFocus::Path => self.path.push(c),
            ModalFocus::LimitValue => {
                if c.is_ascii_digit() {
                    // Build candidate, clamp to 1..=10.
                    let mut s = self.limit_value.to_string();
                    s.push(c);
                    if let Ok(v) = s.parse::<u32>() {
                        if (1..=10).contains(&v) {
                            self.limit_value = v;
                        } else if v == 0 {
                            self.limit_value = 1;
                        }
                        // values > 10 ignored (keep current)
                    }
                }
            }
            _ => {}
        }
    }

    fn backspace(&mut self) {
        match self.focus {
            ModalFocus::Path => {
                self.path.pop();
            }
            ModalFocus::LimitValue => {
                let s = self.limit_value.to_string();
                let new = if s.len() <= 1 {
                    1
                } else {
                    s[..s.len() - 1].parse::<u32>().unwrap_or(1)
                };
                self.limit_value = new.clamp(1, 10);
            }
            _ => {}
        }
    }
}
