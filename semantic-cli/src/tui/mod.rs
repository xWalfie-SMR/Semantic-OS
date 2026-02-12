// tui/mod.rs
// The TUI installer wizard for SemanticOS.
// Walks the user through setup: shell, command style, folder style, new shell behavior.
// Writes the result to ~/.config/semantic/config.toml.
// Does NOT modify the system — config only.

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, stdout};

use crate::config::SemanticConfig;

// -- installer steps --
// The wizard progresses linearly through these steps.
// Users can go forward (Enter) or back (Backspace) at any point.

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Welcome,
    Shell,
    CommandStyle,
    FolderStyle,
    NewShellBehavior,
    Summary,
    Done,
}

impl Step {
    /// Move to the next step in the wizard.
    fn next(self) -> Self {
        match self {
            Step::Welcome => Step::Shell,
            Step::Shell => Step::CommandStyle,
            Step::CommandStyle => Step::FolderStyle,
            Step::FolderStyle => Step::NewShellBehavior,
            Step::NewShellBehavior => Step::Summary,
            Step::Summary => Step::Done,
            Step::Done => Step::Done,
        }
    }

    /// Move to the previous step in the wizard.
    fn prev(self) -> Self {
        match self {
            Step::Welcome => Step::Welcome,
            Step::Shell => Step::Welcome,
            Step::CommandStyle => Step::Shell,
            Step::FolderStyle => Step::CommandStyle,
            Step::NewShellBehavior => Step::FolderStyle,
            Step::Summary => Step::NewShellBehavior,
            Step::Done => Step::Done,
        }
    }

    /// Numeric index for progress indicator (0-based).
    fn index(self) -> usize {
        match self {
            Step::Welcome => 0,
            Step::Shell => 1,
            Step::CommandStyle => 2,
            Step::FolderStyle => 3,
            Step::NewShellBehavior => 4,
            Step::Summary => 5,
            Step::Done => 6,
        }
    }
}

/// Total number of visible steps (Welcome through Summary).
const TOTAL_STEPS: usize = 6;

// -- app state --
// Holds all the state for the TUI: current step, list selections, and options.

struct App {
    step: Step,

    // list selection state for each step (tracks which item is highlighted)
    shell_state: ListState,
    command_style_state: ListState,
    folder_style_state: ListState,
    new_shell_state: ListState,

    // available options for each step
    shells: Vec<&'static str>,
    command_styles: Vec<&'static str>,
    folder_styles: Vec<&'static str>,
    new_shell_options: Vec<(&'static str, &'static str)>, // (value, description)

    should_quit: bool,
    write_error: Option<String>, // set if config write fails on summary
}

impl App {
    fn new() -> Self {
        // initialize all list states with the first item selected
        let mut shell_state = ListState::default();
        shell_state.select(Some(0));
        let mut command_style_state = ListState::default();
        command_style_state.select(Some(0));
        let mut folder_style_state = ListState::default();
        folder_style_state.select(Some(0));
        let mut new_shell_state = ListState::default();
        new_shell_state.select(Some(0));

        App {
            step: Step::Welcome,
            shell_state,
            command_style_state,
            folder_style_state,
            new_shell_state,

            shells: vec!["fish", "bash", "zsh"],
            command_styles: vec!["natural", "traditional", "verbose"],
            folder_styles: vec!["natural", "traditional", "verbose"],
            new_shell_options: vec![
                ("auto-setup", "Automatically configure new shells"),
                ("notify", "Notify when a new shell is detected"),
                ("ignore", "Do nothing"),
            ],

            should_quit: false,
            write_error: None,
        }
    }

    // -- accessors for the currently selected value in each step --

    fn selected_shell(&self) -> &str {
        self.shells[self.shell_state.selected().unwrap_or(0)]
    }

    fn selected_command_style(&self) -> &str {
        self.command_styles[self.command_style_state.selected().unwrap_or(0)]
    }

    fn selected_folder_style(&self) -> &str {
        self.folder_styles[self.folder_style_state.selected().unwrap_or(0)]
    }

    fn selected_new_shell(&self) -> &str {
        self.new_shell_options[self.new_shell_state.selected().unwrap_or(0)].0
    }

    /// Returns the list state and option count for the current step.
    /// None if the current step doesn't have a list (Welcome, Summary, Done).
    fn current_list_state(&mut self) -> Option<(&mut ListState, usize)> {
        match self.step {
            Step::Shell => Some((&mut self.shell_state, self.shells.len())),
            Step::CommandStyle => {
                Some((&mut self.command_style_state, self.command_styles.len()))
            }
            Step::FolderStyle => {
                Some((&mut self.folder_style_state, self.folder_styles.len()))
            }
            Step::NewShellBehavior => {
                Some((&mut self.new_shell_state, self.new_shell_options.len()))
            }
            _ => None,
        }
    }

    // -- navigation --

    fn move_up(&mut self) {
        if let Some((state, len)) = self.current_list_state() {
            let i = state.selected().unwrap_or(0);
            // wrap around to the bottom if at the top
            state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
        }
    }

    fn move_down(&mut self) {
        if let Some((state, len)) = self.current_list_state() {
            let i = state.selected().unwrap_or(0);
            // wrap around to the top if at the bottom
            state.select(Some((i + 1) % len));
        }
    }

    /// Move forward. On the summary step, this writes the config file.
    fn advance(&mut self) {
        if self.step == Step::Summary {
            // build config from all the selections and write it
            let config = SemanticConfig::from_selections(
                self.selected_shell(),
                self.selected_command_style(),
                self.selected_folder_style(),
                self.selected_new_shell(),
            );
            match config.save() {
                Ok(()) => {
                    self.write_error = None;
                    self.step = Step::Done;
                }
                Err(e) => {
                    self.write_error = Some(format!("Failed to write config: {e}"));
                }
            }
        } else {
            self.step = self.step.next();
        }
    }

    fn go_back(&mut self) {
        self.write_error = None;
        self.step = self.step.prev();
    }
}

// -- public entry point --

pub fn run() {
    if let Err(e) = run_inner() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run_inner() -> Result<(), Box<dyn std::error::Error>> {
    // set up terminal for TUI rendering
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // main loop: draw -> wait for input -> repeat
    while !app.should_quit && app.step != Step::Done {
        terminal.draw(|f| draw(f, &mut app))?;
        handle_event(&mut app)?;
    }

    // restore terminal to normal state
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    // print confirmation after exiting the TUI
    if app.step == Step::Done {
        let path = SemanticConfig::config_path();
        println!("Config written to {}", path.display());
        println!("Run `semantic init` to generate shell aliases.");
    }

    Ok(())
}

// ============================================================
// Drawing
// ============================================================
// The screen is split into three sections:
//   1. Progress bar (top) — dots showing which step you're on
//   2. Content (middle) — the actual step content, vertically centered
//   3. Help bar (bottom) — keybindings

fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let layout = Layout::vertical([
        Constraint::Length(3), // progress dots
        Constraint::Min(8),   // main content
        Constraint::Length(3), // help bar
    ])
    .split(area);

    draw_progress(f, layout[0], app);
    draw_content(f, layout[1], app);
    draw_help(f, layout[2], app);
}

/// Draws the progress dots at the top.
/// Completed steps are green, current step is cyan, future steps are gray.
fn draw_progress(f: &mut Frame, area: Rect, app: &App) {
    let step = app.step.index();
    let dots: Vec<Span> = (0..TOTAL_STEPS)
        .map(|i| {
            if i < step {
                Span::styled(" ● ", Style::default().fg(Color::Green))
            } else if i == step {
                Span::styled(" ● ", Style::default().fg(Color::Cyan).bold())
            } else {
                Span::styled(" ○ ", Style::default().fg(Color::DarkGray))
            }
        })
        .collect();

    let progress = Paragraph::new(Line::from(dots))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(progress, area);
}

/// Draws the main content area.
/// Adds horizontal padding and vertically centers the content.
fn draw_content(f: &mut Frame, area: Rect, app: &mut App) {
    // horizontal padding — 5% on each side
    let padded = Layout::horizontal([
        Constraint::Percentage(5),
        Constraint::Percentage(90),
        Constraint::Percentage(5),
    ])
    .split(area);

    // vertically center the content in the available space
    let content_height: u16 = match app.step {
        Step::Welcome => 10,
        Step::Summary => 10,
        _ => 8,
    };
    let vertical_pad = padded[1].height.saturating_sub(content_height) / 2;
    let centered = Layout::vertical([
        Constraint::Length(vertical_pad),
        Constraint::Length(content_height),
        Constraint::Min(0),
    ])
    .split(padded[1]);

    let content_area = centered[1];

    // render the right content for the current step
    match app.step {
        Step::Welcome => draw_welcome(f, content_area),
        Step::Shell => draw_selection(
            f,
            content_area,
            "Which shell do you use?",
            &app.shells.iter().map(|s| (*s, "")).collect::<Vec<_>>(),
            &mut app.shell_state,
        ),
        Step::CommandStyle => draw_selection(
            f,
            content_area,
            "Pick a command style:",
            &[
                ("natural", "goto, list, install, delete"),
                ("traditional", "cd, ls, pacman, rm"),
                ("verbose", "go-to, list-files, install-package"),
            ],
            &mut app.command_style_state,
        ),
        Step::FolderStyle => draw_selection(
            f,
            content_area,
            "Pick a folder style:",
            &[
                ("natural", "/apps, /settings, /logs"),
                ("traditional", "/usr/bin, /etc, /var/log"),
                ("verbose", "/user/applications, /configuration"),
            ],
            &mut app.folder_style_state,
        ),
        Step::NewShellBehavior => draw_selection(
            f,
            content_area,
            "When a new shell is installed:",
            &app.new_shell_options.to_vec(),
            &mut app.new_shell_state,
        ),
        Step::Summary => draw_summary(f, content_area, app),
        Step::Done => {}
    }
}

/// Draws the welcome screen — title, description, config path hint.
fn draw_welcome(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "SemanticOS",
            Style::default()
                .fg(Color::Cyan)
                .bold()
                .add_modifier(Modifier::UNDERLINED),
        )),
        Line::from(""),
        Line::from("Welcome to the SemanticOS setup wizard."),
        Line::from(""),
        Line::from("This will configure how you interact with your system."),
        Line::from("You can change everything later in:"),
        Line::from(Span::styled(
            "  ~/.config/semantic/config.toml",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to get started.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
    f.render_widget(paragraph, area);
}

/// Draws a selection list with a prompt.
/// Each option has a name and an optional description.
/// The selected item gets a cyan background with dark text.
fn draw_selection(
    f: &mut Frame,
    area: Rect,
    prompt: &str,
    options: &[(&str, &str)],
    state: &mut ListState,
) {
    // split into prompt area and list area
    let layout = Layout::vertical([
        Constraint::Length(3), // prompt text
        Constraint::Min(4),   // option list
    ])
    .split(area);

    // render the prompt
    let prompt_widget = Paragraph::new(prompt)
        .style(Style::default().fg(Color::White).bold())
        .alignment(Alignment::Left);
    f.render_widget(prompt_widget, layout[0]);

    let selected = state.selected().unwrap_or(0);

    // build list items with selection styling
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let is_selected = i == selected;

            // arrow marker for the selected item, padding for the rest
            let marker = if is_selected { "  ▸ " } else { "    " };

            // selected item: dark text on colored background
            // unselected: white text, no background
            let name_style = if is_selected {
                Style::default().fg(Color::Black).bold()
            } else {
                Style::default().fg(Color::White)
            };

            let mut spans = vec![
                Span::styled(marker, name_style),
                Span::styled(*name, name_style),
            ];

            // add description text if present (e.g. example commands)
            if !desc.is_empty() {
                let desc_style = if is_selected {
                    Style::default().fg(Color::Black)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(format!("  {desc}"), desc_style));
            }

            // apply background color to the entire row if selected
            let item = ListItem::new(Line::from(spans));
            if is_selected {
                item.style(Style::default().bg(Color::Cyan))
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items).highlight_style(Style::default());
    f.render_stateful_widget(list, layout[1], state);
}

/// Draws the summary screen — shows all selections for review before saving.
fn draw_summary(f: &mut Frame, area: Rect, app: &App) {
    let lines = vec![
        Line::from(Span::styled(
            "Review your choices:",
            Style::default().bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Shell:          ", Style::default().fg(Color::DarkGray)),
            Span::styled(app.selected_shell(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Command style:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.selected_command_style(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Folder style:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.selected_folder_style(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("  New shell:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(app.selected_new_shell(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to save, or Backspace to go back.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    // show error if config write failed
    if let Some(ref err) = app.write_error {
        let mut lines = lines;
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            err.as_str(),
            Style::default().fg(Color::Red).bold(),
        )));
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new(lines);
        f.render_widget(paragraph, area);
    }
}

/// Draws the help bar at the bottom — shows available keybindings for the current step.
fn draw_help(f: &mut Frame, area: Rect, app: &App) {
    let help_text = match app.step {
        Step::Welcome => "Enter: continue  •  q: quit",
        Step::Summary => "Enter: save config  •  Backspace: back  •  q: quit",
        _ => "↑/↓: select  •  Enter: continue  •  Backspace: back  •  q: quit",
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(help, area);
}

// ============================================================
// Event handling
// ============================================================
// Reads keyboard input and updates app state accordingly.
// Only responds to key press events (ignores key release/repeat).

fn handle_event(app: &mut App) -> io::Result<()> {
    if let Event::Key(key) = event::read()? {
        // ignore key release events (some terminals send both press and release)
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                app.should_quit = true;
            }
            KeyCode::Enter => {
                app.advance();
            }
            KeyCode::Backspace => {
                app.go_back();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.move_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.move_down();
            }
            _ => {}
        }
    }

    Ok(())
}
