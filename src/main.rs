use ratatui::DefaultTerminal;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use clap::Parser;
use std::path::PathBuf;
use rpu::core::RAM;
use rpu::tui::MEMORY_BYTES_PER_ROW;
use rpu::{render, Computer, UiState};
use color_eyre::Result;
use crossterm::event;
use std::fs;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    source: PathBuf
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    let source = fs::read_to_string(&args.source)?;
    let mut computer = Computer::new();
    computer.load_source(&source).unwrap();

    color_eyre::install()?;
    let terminal = ratatui::init();
    let app = App::new(computer);
    let result = run(terminal, app);
    ratatui::restore();
    result
}

/// Thin wrapper holding the headless `Computer` and the TUI view
/// state side by side. The split lets the event loop borrow
/// `&app.computer` and `&mut app.ui` independently when calling
/// `render`, which the borrow checker would not allow if both fields
/// lived inside the same struct.
struct App {
    computer: Computer,
    ui: UiState,
}

impl App {
    fn new(computer: Computer) -> Self {
        Self {
            computer,
            ui: UiState::default(),
        }
    }
}

fn run(
    mut terminal: DefaultTerminal,
    mut app: App,
) -> Result<()> {
    loop {
        terminal.draw(|f| { render(f, &app.computer, &mut app.ui); })?;
        match event::read()? {
            Event::Key(ke) => {
                match ke.code {
                    KeyCode::Esc => {
                        break Ok(())
                    },
                    KeyCode::Char('q') => {
                        break Ok(())
                    },
                    KeyCode::Down => {
                        let new = app.ui.code_list_state.offset() + 1;
                        let current = app.ui.code_list_state.offset_mut();
                        *current = new;
                    },
                    KeyCode::Up => {
                        let old = app.ui.code_list_state.offset();
                        let new = if old == 0 { 0 } else { old - 1 };
                        let current = app.ui.code_list_state.offset_mut();
                        *current = new;
                    },
                    KeyCode::PageDown => {
                        let total_rows = RAM / MEMORY_BYTES_PER_ROW;
                        let last = total_rows.saturating_sub(1);
                        let jump = app.ui.memory_page_rows.max(1);
                        app.ui.memory_selected_row =
                            (app.ui.memory_selected_row + jump).min(last);
                    },
                    KeyCode::PageUp => {
                        let jump = app.ui.memory_page_rows.max(1);
                        app.ui.memory_selected_row =
                            app.ui.memory_selected_row.saturating_sub(jump);
                    },
                    KeyCode::Char('n') => {
                        match app.computer.step() {
                            Ok(()) => { continue; },
                            Err(e) => {
                                app.computer.devices.tty
                                    .push_line(&format!("{:?}", e));
                                continue;
                            }
                        }
                    },
                    _ => {},
                }
            },
            _ => {}
        }
    }
}
