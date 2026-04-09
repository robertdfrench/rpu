//! TUI rendering. Everything that turns CPU state into pixels lives
//! here. The headless `Computer` API in `computer.rs` knows nothing
//! about ratatui — that separation is what lets `tests/cpu.rs` run
//! without a terminal, and what lets `tests/render.rs` exercise the
//! renderer with `ratatui::backend::TestBackend`.
//!
//! Entry point: [`render`]. Everything else is private.

use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::layout:: Constraint;
use ratatui::layout:: Direction;
use ratatui::         Frame;
use ratatui::layout:: Layout;
use ratatui::text::   Line;
use ratatui::widgets::List;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::layout:: Rect;
use ratatui::widgets::Row;
use ratatui::text::   Span;
use ratatui::style::  Style;
use ratatui::style::  Stylize;
use ratatui::widgets::Table;

use crate::Computer;
use crate::Lcd;
use crate::Tty;
use crate::core::RAM;
use crate::programs::Program;

/// How many memory bytes are shown per row in the memory pane. The
/// PgUp/PgDn key handlers in `main.rs` need this constant too, so it
/// is `pub`.
pub const MEMORY_BYTES_PER_ROW: usize = 8;

/// View state for the TUI — everything the renderer needs that is
/// *not* part of the CPU itself. Held separately from `Computer` for
/// two reasons:
///
/// 1. The renderer can be called from tests with a `TestBackend`
///    without dragging in any CPU mutability.
/// 2. It makes the boundary explicit: rendering may freely mutate
///    `UiState`, but it takes `&Computer` (not `&mut`), so it
///    physically cannot scribble on CPU state.
#[derive(Default)]
pub struct UiState {
    /// ratatui's built-in scroll/selection state for the code pane's
    /// `List` widget. The renderer overwrites the selected index on
    /// every frame to point at whichever source line corresponds to
    /// the current `pc`, so the highlighted line tracks execution.
    /// The *scroll offset* inside this struct is what the Up/Down
    /// arrow keys mutate in the event loop, letting the user scroll
    /// the code window independently of the highlight.
    pub code_list_state: ListState,

    /// Index of the highlighted memory row, in 8-byte rows (so row 3
    /// covers addresses 24..32). Drives both the highlight color and
    /// the scroll position of the memory pane — the renderer centers
    /// this row in the visible window.
    pub memory_selected_row: usize,

    /// How many memory rows fit in the pane on the most recent frame.
    /// Set as a side effect of `render_memory` (it knows the pane
    /// height) and read by the PgUp/PgDn key handlers in `main.rs` so
    /// they can jump a full page at a time. Starts at 0; the first
    /// frame populates it before any key event can fire.
    pub memory_page_rows: usize,
}

/// Pre-computed rectangles for every pane in the TUI. Built fresh on
/// every frame from the current terminal size. Internal to this
/// module — outside callers go through [`render`].
struct Layouts {
    code: Rect,
    help: Rect,
    lcd0: Rect,
    lcd1: Rect,
    memory: Rect,
    printer: Rect,
    power_led: Rect,
    registers: Rect,
    special_registers: Rect,
}

impl Layouts {
    /// Splits the terminal into three vertical strips (code on the
    /// left, devices in the middle, debug tools on the right), then
    /// further subdivides each strip.
    fn new(frame: &Frame) -> Self {
        // Top-level: code area (flexes), devices column (~31 cols),
        // tools column (~55 cols).
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(100),
                Constraint::Min(31),
                Constraint::Min(55),
            ])
            .split(frame.area());

        // Left strip: code list fills, with a 4-row help block pinned
        // to the bottom.
        let lefthand_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(4)
            ])
            .split(layout[0]);
        let code = lefthand_layout[0];
        let help = lefthand_layout[1];

        // Middle strip: two 7-row LCDs, a 3-row power LED, then the
        // tty/error console takes whatever's left.
        let devices_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(7),
                Constraint::Length(7),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .split(layout[1]);
        let lcd0 = devices_layout[0];
        let lcd1 = devices_layout[1];
        let power_led = devices_layout[2];
        let printer = devices_layout[3];

        // Right strip: GP registers (4 rows), special registers
        // (4 rows), then the memory pane gets the rest.
        let tools_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Fill(1),
            ])
            .split(layout[2]);
        let registers = tools_layout[0];
        let special_registers = tools_layout[1];
        let memory = tools_layout[2];

        Self {
            code,
            help,
            lcd0,
            lcd1,
            memory,
            registers,
            special_registers,
            printer,
            power_led,
        }
    }
}

/// Draws one full frame of the TUI for `computer`, mutating only the
/// view state in `ui` (which row is selected, where the code list is
/// scrolled, how many memory rows fit on screen). The `&Computer`
/// (not `&mut`) is the load-bearing detail: rendering provably cannot
/// touch CPU state, so it is safe to call from tests at arbitrary
/// points.
///
/// `computer.program` must be `Some` — the TUI is only meaningful
/// with a program loaded, and `main()` always loads one before
/// calling `render`. Tests must also call `load_source` first.
pub fn render(
    frame: &mut Frame,
    computer: &Computer,
    ui: &mut UiState,
) {
    let layouts = Layouts::new(frame);

    let program = computer.program.as_ref()
        .expect("render called before a program was loaded");

    render_code(
        program,
        computer.core.register_file.pc,
        &mut ui.code_list_state,
        layouts.code,
        frame,
        "Code"
    );
    render_help(layouts.help, frame, "Help");
    render_lcd(
        &computer.devices.lcd0,
        layouts.lcd0,
        frame,
        "LCD0 (dvc 0)"
    );
    render_lcd(
        &computer.devices.lcd1,
        layouts.lcd1,
        frame,
        "LCD1 (dvc 1)"
    );
    render_led(
        computer.core.power,
        layouts.power_led,
        frame,
        "Power"
    );
    render_printer(
        &computer.devices.tty,
        layouts.printer,
        frame,
        "TTY (dvc 2)"
    );

    let gp_registers = vec![
        ("gp0", computer.core.register_file.gp0),
        ("gp1", computer.core.register_file.gp1),
        ("gp2", computer.core.register_file.gp2),
        ("gp3", computer.core.register_file.gp3),
        ("gp4", computer.core.register_file.gp4),
        ("gp5", computer.core.register_file.gp5),
        ("gp6", computer.core.register_file.gp6),
        ("gp7", computer.core.register_file.gp7),
    ];
    render_registers(
        gp_registers,
        layouts.registers,
        frame,
        "General Purpose Registers"
    );

    let sp_registers = vec![
        ("ans", computer.core.register_file.ans),
        ("dvc", computer.core.register_file.dvc),
        ("pc",  computer.core.register_file.pc),
        ("sp",  computer.core.register_file.sp),
    ];
    render_registers(
        sp_registers,
        layouts.special_registers,
        frame,
        "Special Purpose Registers"
    );

    render_memory(
        computer,
        ui.memory_selected_row,
        &mut ui.memory_page_rows,
        layouts.memory,
        frame,
        "Memory"
    );
}

/// Coarse classification of a memory address into the kind of thing
/// it currently holds. Used by the memory pane to color regions so
/// students can visually pick out where the program, current
/// instruction, and stack live without having to count bytes. The
/// classification is derived from the live `Computer` state on every
/// frame — there is no persistent state for this.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Region {
    /// The four bytes of the next instruction to execute (`pc..pc+4`).
    /// Wins over every other classification, including Program,
    /// because students care more about "where am I?" than "what
    /// kind of memory is this?".
    Current,
    /// Stack contents: addresses from `sp + 2` (the most recently
    /// pushed value) up to and including `RAM - 2` (the bottom of
    /// memory, where the stack starts). Empty when nothing has been
    /// pushed.
    Stack,
    /// Loaded program bytes (`0..program.size()`).
    Program,
    /// Everything else — scratch / unused memory.
    Unused,
}

fn region_of(addr: usize, computer: &Computer) -> Region {
    let pc = computer.core.register_file.pc as usize;
    if addr >= pc && addr < pc + 4 {
        return Region::Current;
    }
    let sp = computer.core.register_file.sp as usize;
    // The stack lives at addresses (sp + 2)..=(RAM - 2). Empty when
    // sp + 2 > RAM - 2, which is its initial state.
    if addr >= sp + 2 && addr + 1 < RAM {
        return Region::Stack;
    }
    if let Some(p) = &computer.program {
        if addr < p.size() {
            return Region::Program;
        }
    }
    Region::Unused
}

fn region_style(region: Region) -> Style {
    match region {
        // Yellow bg + black fg works on both light and dark
        // terminal backgrounds.
        Region::Current => Style::new().black().on_yellow().bold(),
        // Plain (non-dim) magenta is distinct from program green
        // and high-contrast on white backgrounds.
        Region::Stack   => Style::new().magenta(),
        // Plain green (no .dim() — dimmed green vanishes on white).
        Region::Program => Style::new().green(),
        Region::Unused  => Style::new(),
    }
}

/// Draws the source-code list on the left side of the screen and
/// highlights whichever line maps to the current `pc`. The mapping
/// from `pc` to source line is precomputed at compile time and lives
/// on `Program::source_addrs`.
///
/// The user can scroll this list independently of the highlight by
/// mutating the offset on `state` from the event loop (Up/Down keys);
/// here we only call `state.select(...)` to set the highlighted row.
fn render_code(
    program: &Program,
    pc: u16,
    state: &mut ListState,
    area: Rect,
    frame: &mut Frame,
    title: &str,
) {
    let current_line = program.source_addrs.get(&pc);
    state.select(current_line.copied());
    let items = program.source_lines.clone();
    let list = List::new(items)
        .block(common_block(title))
        .highlight_style(Style::new().italic().red());

    frame.render_stateful_widget(list, area, state);
}

/// Draws the static "Help" cheat sheet block at the bottom-left of
/// the screen. Two columns × two rows of key/description pairs.
fn render_help(
    area: Rect,
    frame: &mut Frame,
    title: &str
) {
    let text_n = vec![
        Line::from(vec![
            Span::styled("n", Style::new().bold()),
            Span::raw(" - execute next instruction")
        ])
    ];
    let text_q = vec![
        Line::from(vec![
            Span::styled("q", Style::new().bold()),
            Span::raw(" - exit this program")
        ]),
    ];
    let text_up = vec![
        Line::from(vec![
            Span::styled("Up/Down", Style::new().bold()),
            Span::raw(" - scroll code window")
        ])
    ];
    let text_pgup = vec![
        Line::from(vec![
            Span::styled("PgUp/PgDown", Style::new().bold()),
            Span::raw(" - scroll mem window")
        ])
    ];
    let rows = [
        Row::new([text_n, text_q]),
        Row::new([text_up, text_pgup])
    ];
    let widths = vec![
        Constraint::Length(28), Constraint::Length(31)
    ];
    let table = Table::new(rows, widths)
        .column_spacing(4)
        .block(common_block(title));
    frame.render_widget(table, area);
}

/// Draws the power indicator: green " ON " when `power == true`,
/// red " OFF " when the CPU has halted.
fn render_led(
    power: bool,
    area: Rect,
    frame: &mut Frame,
    title: &str,
) {
    let content = match power {
        true =>  " ON  ",
        false => " OFF "
    };
    let style = match power {
        true => Style::new().black().on_green(),
        false => Style::new().white().on_red(),
    };
    let text = vec![
        Line::from(vec![
            Span::styled(
                String::from(content),
                style
            )
        ])
    ];
    let paragraph = Paragraph::new(text)
        .block(common_block(title))
        .centered();
    frame.render_widget(paragraph, area);
}

/// Renders an `Lcd` device as a 5-row 7-segment display showing the
/// most recently written value, zero-padded to 5 digits. The font
/// glyphs are stored as ASCII art in `lcd_font.txt` (one 5-row glyph
/// per digit, concatenated). Reads only `lcd.last_written()`; the
/// full write history is the test interface, not the render
/// interface.
fn render_lcd(
    lcd: &Lcd,
    area: Rect,
    frame: &mut Frame,
    title: &str,
) {
    // Parse the font file once per call: each digit is 5 rows of
    // ASCII art, in order 0..=9.
    let font_definition = include_str!("../lcd_font.txt");
    let mut lcd_font: Vec<Vec<&str>> = vec![];
    let mut current_lcd_char: Vec<&str> = vec![];
    for (n, text) in font_definition.lines().enumerate() {
        current_lcd_char.push(text);
        if ((n + 1) % 5) == 0 {
            lcd_font.push(current_lcd_char);
            current_lcd_char = vec![];
        }
    }

    // Build the 5-row display by stitching together the matching row
    // of each digit's glyph, left to right.
    let value = format!("{:0>5}", lcd.last_written().unwrap_or(0));
    let mut content = String::new();
    for row in 0..5 {
        for c in value.chars() {
            let char_id = match c {
                '0' => 0,
                '1' => 1,
                '2' => 2,
                '3' => 3,
                '4' => 4,
                '5' => 5,
                '6' => 6,
                '7' => 7,
                '8' => 8,
                '9' => 9,
                _ => panic!()
            };

            content.push_str(lcd_font[char_id][row]);
        }
        content.push_str("\n");
    }

    let paragraph = Paragraph::new(content)
        .block(common_block(title));
    frame.render_widget(paragraph, area);
}

/// Draws the tty buffer (dvc 2) as a free-form text block. Programs
/// write to it via `copy <reg> out` after `put 2 dvc`. The TUI's
/// step loop also `push_line`s `ExecutionError`s here for now —
/// that will move to a dedicated STATUS pane in a later stage (see
/// the error-handling-reform note).
fn render_printer(
    tty: &Tty,
    area: Rect,
    frame: &mut Frame,
    title: &str
) {
    let paragraph = Paragraph::new(tty.contents().to_string())
        .block(common_block(title));
    frame.render_widget(paragraph, area);
}

/// Draws a row of `(name, value)` register cells with bold name
/// headers above them. Used for both the GP register row and the
/// special-purpose register row — same shape, different inputs.
fn render_registers(
    pairs: Vec<(&str, u16)>,
    area: Rect,
    frame: &mut Frame,
    title: &str
) {
    let cells: Vec<String> = (&pairs).into_iter().map(|(_, val)| {
        format!("{:5}", val)
    }).collect();
    let rows = [Row::new(cells)];
    let widths: Vec<Constraint> = (&pairs).into_iter().map(|_| {
        Constraint::Length(5)
    }).collect();
    let block = common_block(title);
    let header_cells: Vec<String> = (&pairs).into_iter().map(|(name, _)| {
        format!("{:>5}", name)
    }).collect();
    let header = Row::new(header_cells)
        .style(Style::new().bold());
    let table = Table::new(rows, widths)
        .column_spacing(1)
        .header(header)
        .block(block);
    frame.render_widget(table, area);
}

/// Draws the memory pane: an `ADDR` column followed by 8 byte
/// columns. Only the rows that fit in `area` are built — frame cost
/// is constant in `RAM` size, not linear. Originally a stage 1
/// optimization to support a larger RAM; today it's still the right
/// shape even though `RAM` itself is small.
///
/// Each byte is styled by one of:
/// - **Change highlight** (white-on-blue, bold) if the byte just
///   changed in the most recent step. This is the killer feature
///   from stage 5a — students press `n` and immediately see the
///   bytes that just got written. Wins over region coloring.
/// - **Region color** (5b) otherwise: yellow bg for the four bytes
///   of the next instruction (`pc..pc+4`), dim cyan for stack
///   contents, dim green for program bytes, default for unused.
///
/// `selected_row` is the row to highlight as "the one the cursor is
/// on"; the window is scrolled so that row sits in the middle when
/// possible (clamped to the ends). As a side effect, writes the
/// visible row count back through `page_rows_out` so the PgUp/PgDn
/// handlers in `main.rs` can read it on the next key event.
fn render_memory(
    computer: &Computer,
    selected_row: usize,
    page_rows_out: &mut usize,
    area: Rect,
    frame: &mut Frame,
    title: &str,
) {
    let memory = &computer.core.memory;
    let total_rows = memory.len() / MEMORY_BYTES_PER_ROW;
    let header_style = Style::new().bold();
    let row_select_style = Style::new().red().italic();
    let changed_style = Style::new().white().on_blue().bold();

    // Reserve: top border (1) + header (1) + blank (1) + bottom border (1).
    let visible_rows = (area.height as usize).saturating_sub(4);
    // Report back to the key handlers so PgUp/PgDn can jump a page.
    *page_rows_out = visible_rows.max(1);
    if visible_rows == 0 || total_rows == 0 {
        let paragraph = Paragraph::new("").block(common_block(title));
        frame.render_widget(paragraph, area);
        return;
    }

    // Center the selected row in the window when possible, but keep
    // the window inside [0, total_rows - visible_rows].
    let half = visible_rows / 2;
    let max_top = total_rows.saturating_sub(visible_rows);
    let scroll_top = selected_row.saturating_sub(half).min(max_top);

    let mut lines: Vec<Line> = Vec::with_capacity(visible_rows + 2);
    lines.push(Line::from(vec![
        Span::styled("ADDR ", header_style),
        Span::styled("   +0", header_style),
        Span::styled("   +1", header_style),
        Span::styled("   +2", header_style),
        Span::styled("   +3", header_style),
        Span::styled("   +4", header_style),
        Span::styled("   +5", header_style),
        Span::styled("   +6", header_style),
        Span::styled("   +7", header_style),
    ]));
    lines.push(Line::from(""));

    for i in 0..visible_rows {
        let row_idx = scroll_top + i;
        if row_idx >= total_rows { break; }
        let row_start = row_idx * MEMORY_BYTES_PER_ROW;

        let mut spans: Vec<Span> = Vec::with_capacity(MEMORY_BYTES_PER_ROW + 1);
        spans.push(Span::raw(format!("{:>5}", row_start)));
        for offset in 0..MEMORY_BYTES_PER_ROW {
            let addr = row_start + offset;
            let byte = memory[addr];
            // Per-byte style priority: changed > region.
            let style = if computer.byte_changed(addr as u16) {
                changed_style
            } else {
                region_style(region_of(addr, computer))
            };
            spans.push(Span::styled(format!("{:>5}", byte), style));
        }

        // The selected-row style is fg+italic only (no bg), so it
        // composes cleanly with per-byte bg styles: a changed byte on
        // the selected row still shows as white-on-blue.
        let line = if row_idx == selected_row {
            Line::from(spans).style(row_select_style)
        } else {
            Line::from(spans)
        };
        lines.push(line);
    }

    let paragraph = Paragraph::new(lines).block(common_block(title));
    frame.render_widget(paragraph, area);
}

/// Standard bordered block with a `[Title]` label in blue. Every
/// pane wraps its content in one of these so the TUI has a uniform
/// look.
fn common_block(title: &str) -> Block<'_> {
    let title = format!("[{title}]");
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().blue())
}
