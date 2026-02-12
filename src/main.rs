#[cfg(windows)]
use arboard::Clipboard;
#[cfg(windows)]
use arboard::Clipboard;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use rand::prelude::*;
use rand::rng;
use rand::seq::SliceRandom;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::{self, Stdout, Write};
#[cfg(unix)]
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const NUMBERS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!#$%&()*+";
const DEFAULT_LETTERS: i32 = 6;
const DEFAULT_UPPERCASE: i32 = 2;
const DEFAULT_SYMBOLS: i32 = 2;
const DEFAULT_NUMBERS: i32 = 4;
const MIN_VALUE: i32 = 0;
const MAX_VALUE: i32 = 64;
const CLIPBOARD_MESSAGE_DURATION: Duration = Duration::from_secs(2);
const FOCUS_FIELDS: usize = 4;
const FOCUS_GENERATE: usize = 4;
const FOCUS_COPY: usize = 5;
const FOCUS_QUIT: usize = 6;

fn check_password_strength(password: &str) -> &'static str {
    let length_criteria = password.len() >= 10;
    let uppercase_criteria = password.chars().any(|ch| ch.is_ascii_uppercase());
    let lowercase_criteria = password.chars().any(|ch| ch.is_ascii_lowercase());
    let number_criteria = password.chars().any(|ch| ch.is_ascii_digit());
    let symbol_criteria = password.chars().any(|ch| SYMBOLS.contains(&(ch as u8)));

    let criteria_met = [
        length_criteria,
        uppercase_criteria,
        lowercase_criteria,
        number_criteria,
        symbol_criteria,
    ]
    .iter()
    .filter(|&&c| c)
    .count();

    if criteria_met == 5 {
        "Strong"
    } else if criteria_met >= 4 {
        "Moderate"
    } else if criteria_met >= 3 {
        "Weak"
    } else {
        "Do not use!!!!"
    }
}

fn generate_password(
    letters: i32,
    uppercase: i32,
    symbols: i32,
    numbers: i32,
    rng: &mut impl Rng,
) -> String {
    let mut generated: Vec<u8> = Vec::new();

    for _ in 0..letters {
        generated.push(*LETTERS.choose(rng).unwrap());
    }
    for _ in 0..uppercase {
        let letter = *LETTERS.choose(rng).unwrap();
        generated.push(letter.to_ascii_uppercase());
    }
    for _ in 0..symbols {
        generated.push(*SYMBOLS.choose(rng).unwrap());
    }
    for _ in 0..numbers {
        generated.push(*NUMBERS.choose(rng).unwrap());
    }

    generated.shuffle(rng);
    String::from_utf8(generated).unwrap_or_default()
}

struct App {
    letters: i32,
    uppercase: i32,
    symbols: i32,
    numbers: i32,
    focus: usize,
    password: String,
    strength: String,
    status: String,
    status_until: Option<Instant>,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            letters: DEFAULT_LETTERS,
            uppercase: DEFAULT_UPPERCASE,
            symbols: DEFAULT_SYMBOLS,
            numbers: DEFAULT_NUMBERS,
            focus: 0,
            password: String::new(),
            strength: "".to_string(),
            status: "".to_string(),
            status_until: None,
        };
        app.generate_password();
        app
    }

    fn generate_password(&mut self) {
        let mut rng = rng();
        self.password = generate_password(
            self.letters,
            self.uppercase,
            self.symbols,
            self.numbers,
            &mut rng,
        );
        self.strength = check_password_strength(&self.password).to_string();
    }

    fn clamp_value(value: i32) -> i32 {
        value.clamp(MIN_VALUE, MAX_VALUE)
    }

    fn update_value(&mut self, delta: i32) {
        match self.focus {
            0 => self.letters = Self::clamp_value(self.letters + delta),
            1 => self.uppercase = Self::clamp_value(self.uppercase + delta),
            2 => self.symbols = Self::clamp_value(self.symbols + delta),
            3 => self.numbers = Self::clamp_value(self.numbers + delta),
            _ => {}
        }
    }

    fn clear_status_if_expired(&mut self) {
        if let Some(deadline) = self.status_until {
            if Instant::now() >= deadline {
                self.status.clear();
                self.status_until = None;
            }
        }
    }
}

#[cfg(unix)]
fn copy_to_clipboard(value: &str) -> bool {
    let mut child = match Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };

    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(value.as_bytes()).is_err() {
            return false;
        }
    } else {
        return false;
    }

    child.wait().is_ok()
}

#[cfg(windows)]
fn copy_to_clipboard(value: &str) -> bool {
    let mut clipboard = match Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(_) => return false,
    };

    clipboard.set_text(value.to_string()).is_ok()
}

#[cfg(not(any(unix, windows)))]
fn copy_to_clipboard(_: &str) -> bool {
    false
}

fn ui(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let outer = Block::default().borders(Borders::ALL).title("GenPass");
    frame.render_widget(outer, size);

    let inner = size.inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
        ])
        .split(inner);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "Password Generator",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("  Use arrows (h, j, k, l) or +/- to adjust. Enter to generate."),
    ]));
    frame.render_widget(header, chunks[0]);

    let fields = [
        ("Letters", app.letters),
        ("Uppercase", app.uppercase),
        ("Symbols", app.symbols),
        ("Numbers", app.numbers),
    ];

    let field_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(chunks[1]);

    for (index, ((label, value), area)) in fields.iter().zip(field_chunks.iter()).enumerate() {
        let is_active = index == app.focus;
        let line = Line::from(vec![
            Span::styled(
                format!("{label:<10}"),
                if is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            ),
            Span::raw("  "),
            Span::styled(
                format!("{value:>3}"),
                if is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(line), *area);
    }

    let actions = ["Generate", "Copy to clipboard", "Quit"];
    let actions_block = Block::default().borders(Borders::ALL).title("Actions");
    frame.render_widget(actions_block, chunks[2]);
    let inner_actions = chunks[2].inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    let action_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner_actions);

    for (index, label) in actions.iter().enumerate() {
        let focus_index = FOCUS_GENERATE + index;
        let is_active = app.focus == focus_index;
        let style = if is_active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        let line = Line::from(vec![Span::styled(format!("> {label}"), style)]);
        frame.render_widget(Paragraph::new(line), action_rows[index]);
    }

    let strength_ratio = match app.strength.as_str() {
        "Strong" => 1.0,
        "Moderate" => 0.6,
        "Weak" => 0.3,
        "Do not use!!!!" => 0.0,
        _ => 0.0,
    };

    let strength_color = match app.strength.as_str() {
        "Strong" => Color::Green,
        "Moderate" => Color::Yellow,
        "Weak" => Color::Red,
        _ => Color::Gray,
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Strength"))
        .gauge_style(Style::default().fg(strength_color))
        .ratio(strength_ratio);

    let output = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "Generated Password",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::raw(&app.password)]),
        Line::from(vec![Span::styled(
            format!("Strength: {}", app.strength),
            Style::default().fg(strength_color),
        )]),
    ])
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL).title("Output"));

    // render widgets
    frame.render_widget(gauge, chunks[4]);
    frame.render_widget(output, chunks[3]);

    if !app.status.is_empty() {
        let status_area = Rect {
            x: inner.x,
            y: inner.y + inner.height - 1,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                &app.status,
                Style::default().fg(Color::Magenta),
            ))),
            status_area,
        );
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|frame| ui(frame, &app))?;
        app.clear_status_if_expired();

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(KeyEvent {
                code,
                modifiers,
                kind,
                ..
            }) = event::read()?
            {
                if kind != KeyEventKind::Press {
                    continue;
                }
                match (code, modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => return Ok(()),
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                        app.focus = app.focus.saturating_sub(1);
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                        app.focus = (app.focus + 1).min(FOCUS_QUIT);
                    }
                    (KeyCode::Left, _) | (KeyCode::Char('-'), _) | (KeyCode::Char('h'), _) => {
                        app.update_value(-1);
                    }
                    (KeyCode::Right, _)
                    | (KeyCode::Char('+'), _)
                    | (KeyCode::Char('='), _)
                    | (KeyCode::Char('l'), _) => {
                        app.update_value(1);
                    }
                    (KeyCode::Char('g'), _) | (KeyCode::Enter, _) => {
                        if app.focus >= FOCUS_FIELDS {
                            match app.focus {
                                FOCUS_GENERATE => {
                                    app.generate_password();
                                    terminal.draw(|frame| ui(frame, &app))?;
                                }
                                FOCUS_COPY => {
                                    if copy_to_clipboard(&app.password) {
                                        app.status = "Copied to clipboard.".to_string();
                                    } else {
                                        app.status = "Clipboard unavailable.".to_string();
                                    }
                                    app.status_until =
                                        Some(Instant::now() + CLIPBOARD_MESSAGE_DURATION);
                                    terminal.draw(|frame| ui(frame, &app))?;
                                }
                                FOCUS_QUIT => return Ok(()),
                                _ => {}
                            }
                        } else {
                            app.generate_password();
                            terminal.draw(|frame| ui(frame, &app))?;
                        }
                    }
                    (KeyCode::Char('c'), _) | (KeyCode::Char('C'), _) => {
                        if copy_to_clipboard(&app.password) {
                            app.status = "Copied to clipboard.".to_string();
                        } else {
                            app.status = "Clipboard unavailable.".to_string();
                        }
                        app.status_until = Some(Instant::now() + CLIPBOARD_MESSAGE_DURATION);
                        terminal.draw(|frame| ui(frame, &app))?;
                    }
                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                        app.generate_password();
                        terminal.draw(|frame| ui(frame, &app))?;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

#[cfg(test)]
mod tests;
