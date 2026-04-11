use ratatui::DefaultTerminal;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use clap::Parser;
use std::path::PathBuf;
use rpu::core::RAM;
use rpu::tui::MEMORY_BYTES_PER_ROW;
use rpu::{render, Computer, BootError, UiState};
use color_eyre::Result;
use crossterm::event;
use std::fs;
use miette::{GraphicalReportHandler, GraphicalTheme};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    source: PathBuf
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    let source = fs::read_to_string(&args.source)?;
    let filename = args.source.display().to_string();
    let mut computer = Computer::new();
    match computer.load_source_named(&filename, &source) {
        Ok(()) => {}
        Err(BootError::Compilation(e)) => {
            eprintln!("{:?}", miette::Report::new(e));
            std::process::exit(1);
        }
        Err(BootError::ProgramTooBig(size)) => {
            eprintln!("error: program too big ({size} bytes, max {})", RAM);
            std::process::exit(1);
        }
    }

    color_eyre::install()?;
    let terminal = ratatui::init();
    let app = App::new(computer, args.source);
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
    source_path: PathBuf,
}

impl App {
    fn new(computer: Computer, source_path: PathBuf) -> Self {
        Self {
            computer,
            ui: UiState::default(),
            source_path,
        }
    }

    /// Re-read the source file from disk, compile it, and swap in a
    /// fresh `Computer` on success. On any failure (I/O, compile,
    /// too big) the old computer keeps running untouched and the
    /// formatted error is stashed in `ui.error_popup` for the next
    /// render to display. A successful reload also resets the view
    /// cursors, since old memory-row / scroll positions may no
    /// longer make sense against the new program.
    fn reload(&mut self) {
        match self.try_reload() {
            Ok(fresh) => {
                self.computer = fresh;
                self.ui.code_list_state = Default::default();
                self.ui.memory_selected_row = 0;
            }
            Err(msg) => {
                self.ui.error_popup = Some(msg);
            }
        }
    }

    fn try_reload(&self) -> Result<Computer, String> {
        let source = fs::read_to_string(&self.source_path)
            .map_err(|e| format!("could not read {}: {e}",
                self.source_path.display()))?;
        let filename = self.source_path.display().to_string();
        let mut fresh = Computer::new();
        match fresh.load_source_named(&filename, &source) {
            Ok(()) => Ok(fresh),
            Err(BootError::Compilation(e)) => Err(format_compile_error(e)),
            Err(BootError::ProgramTooBig(size)) => {
                Err(format!("program too big ({size} bytes, max {})", RAM))
            }
        }
    }
}

/// Render a miette `CompilationError` to a plain-text (no ANSI)
/// string using the nocolor unicode theme, so it can be dropped
/// verbatim into a ratatui `Paragraph`.
fn format_compile_error(err: rpu::programs::CompilationError) -> String {
    let mut out = String::new();
    let handler = GraphicalReportHandler::new_themed(
        GraphicalTheme::unicode_nocolor()
    );
    // `render_report` writes via `fmt::Write`; infallible for String.
    let _ = handler.render_report(&mut out, &err);
    out
}

fn run(
    mut terminal: DefaultTerminal,
    mut app: App,
) -> Result<()> {
    loop {
        terminal.draw(|f| { render(f, &app.computer, &mut app.ui); })?;
        match event::read()? {
            Event::Key(ke) => {
                // Modal popup swallows the next key press: dismiss
                // the popup and skip the key's normal action so
                // users can't accidentally reload-again or step
                // while clearing an error.
                if app.ui.error_popup.is_some() {
                    app.ui.error_popup = None;
                    continue;
                }
                match ke.code {
                    KeyCode::Esc => {
                        break Ok(())
                    },
                    KeyCode::Char('q') => {
                        break Ok(())
                    },
                    KeyCode::Char('r') => {
                        app.reload();
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
