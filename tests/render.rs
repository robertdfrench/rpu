//! TUI render tests. These prove the renderer can be exercised
//! against an in-memory `TestBackend` with no real terminal — the
//! load-bearing detail that lets every future visual feature get a
//! cheap regression test.
//!
//! Two tests for now: one that runs a program and asserts that the
//! result lands in LCD0, and one that scrolls the memory pane to the
//! end of RAM. Both deliberately assert on substrings rather than on
//! cell positions, so cosmetic shifts (a column added, a border moved)
//! don't break them — only changes to *what* is displayed.

use ratatui::style::Color;
use ratatui::{backend::TestBackend, Terminal};
use rpu::tui::MEMORY_BYTES_PER_ROW;
use rpu::{render, Computer, UiState, RAM};

/// Flatten a `TestBackend` buffer into a single string with one row
/// per line. Cheap and good enough for substring asserts. Inlined
/// here for the moment; extract into `tests/common/mod.rs` once a
/// third test wants it.
fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Counts the number of buffer cells whose background color matches
/// `bg`. Used by the change-highlight and region-color tests to
/// verify that exactly the expected number of cells got the expected
/// style.
fn cells_with_bg(buffer: &ratatui::buffer::Buffer, bg: Color) -> usize {
    let mut count = 0;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer[(x, y)].style().bg == Some(bg) {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn lcd0_shows_result_after_running_add_program() {
    // Run examples/02.add_5_7.rpu to completion. The program puts 5
    // and 7 into gp0/gp1, adds them, and copies the answer to LCD0.
    let source = std::fs::read_to_string("examples/02.add_5_7.rpu").unwrap();
    let mut computer = Computer::new();
    computer.load_source(&source).unwrap();
    computer.run_to_halt().unwrap();

    // 120x30 is large enough to fit every pane comfortably.
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let rendered = buffer_to_string(terminal.backend().buffer());

    // LCD0 should show "00012" — five 7-segment ASCII-art glyphs in
    // a 5-row block, built by `render_lcd` from `lcd_font.txt`. Each
    // glyph is 6 chars wide (5 visible + trailing space). The LCD
    // pane is just narrow enough to clip the very last trailing
    // space, and any further whitespace gets padded out by the buffer
    // to fill the pane, so the *meaningful* content of each row is
    // the leading non-space portion. Asserting the trimmed leading
    // content of all 5 rows pins down that `render_lcd` drew exactly
    // "00012".
    let expected_lcd_rows = [
        "╭───╮ ╭───╮ ╭───╮  ─╮    ───╮",
        "│  /│ │  /│ │  /│   │       │",
        "│ / │ │ / │ │ / │   │   ╭───╯",
        "│/  │ │/  │ │/  │   │   │",
        "╰───╯ ╰───╯ ╰───╯  ─┴─  ╰───",
    ];
    for (i, expected) in expected_lcd_rows.iter().enumerate() {
        assert!(
            rendered.contains(expected),
            "LCD0 row {i} mismatch.\n\
             expected substring: {expected:?}\n\
             full rendered output:\n{rendered}",
        );
    }

    // And the power LED should read OFF since the program halted.
    assert!(
        rendered.contains("OFF"),
        "expected power LED to read OFF after halt in:\n{rendered}",
    );
}

#[test]
fn printer_pane_shows_tty_output_after_running() {
    // Print "Hi" to the tty (dvc 2): 'H' = 72, 'i' = 105.
    let mut computer = Computer::new();
    computer.load_source(
        "put 2 dvc\n\
         put 72 gp0\n\
         copy gp0 out\n\
         put 105 gp0\n\
         copy gp0 out\n\
         halt\n",
    ).unwrap();
    computer.run_to_halt().unwrap();

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    // The printer pane is rendered as plain text, so "Hi" should
    // appear verbatim in the buffer.
    let rendered = buffer_to_string(terminal.backend().buffer());
    assert!(
        rendered.contains("Hi"),
        "expected 'Hi' in rendered printer pane:\n{rendered}",
    );
}

/// Stage 5a: bytes that just changed in the most recent step are
/// rendered with a blue background. Each byte cell is 5 chars wide,
/// so two changed bytes should produce exactly 10 blue cells in the
/// rendered buffer.
#[test]
fn changed_bytes_are_highlighted_after_step() {
    let mut computer = Computer::new();
    computer.load_source(
        "put 257 gp0\n\
         put 100 gp1\n\
         write gp0 gp1\n\
         halt\n",
    ).unwrap();
    computer.step().unwrap(); // put 257 gp0
    computer.step().unwrap(); // put 100 gp1
    computer.step().unwrap(); // write gp0 gp1 — memory[100..102] := [1, 1]

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();
    // Make sure row 12 (addresses 96..104) is in the visible window.
    ui.memory_selected_row = 100 / MEMORY_BYTES_PER_ROW;
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let blue_cells = cells_with_bg(terminal.backend().buffer(), Color::Blue);
    assert_eq!(
        blue_cells, 10,
        "expected 2 changed bytes × 5 cells each = 10 blue cells",
    );
}

/// Stage 5a: the change highlight is per-step, not cumulative. After
/// running an additional step that doesn't touch memory, the previous
/// frame's blue cells should be gone.
#[test]
fn highlight_clears_on_next_step() {
    let mut computer = Computer::new();
    computer.load_source(
        "put 257 gp0\n\
         put 100 gp1\n\
         write gp0 gp1\n\
         halt\n",
    ).unwrap();
    computer.step().unwrap(); // put 257 gp0
    computer.step().unwrap(); // put 100 gp1
    computer.step().unwrap(); // write gp0 gp1 — memory changes
    computer.step().unwrap(); // halt — no memory change

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();
    ui.memory_selected_row = 100 / MEMORY_BYTES_PER_ROW;
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let blue_cells = cells_with_bg(terminal.backend().buffer(), Color::Blue);
    assert_eq!(
        blue_cells, 0,
        "highlight should clear after a non-mutating step",
    );
}

/// Stage 5b: the four bytes at PC are rendered with a yellow
/// background ("current instruction" region). With just `halt`
/// loaded the program is exactly 4 bytes long, PC is at 0, so
/// addresses 0..4 should all be yellow — that's 4 bytes × 5 cells
/// each = 20 yellow cells.
#[test]
fn current_pc_region_is_yellow_highlighted() {
    let mut computer = Computer::new();
    computer.load_source("halt\n").unwrap();
    // No step()s — we want PC at 0 with halt about to fire.

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let yellow_cells = cells_with_bg(terminal.backend().buffer(), Color::Yellow);
    assert_eq!(
        yellow_cells, 20,
        "expected 4 PC bytes × 5 cells each = 20 yellow cells",
    );
}

#[test]
fn memory_pane_scrolls_to_the_end_of_ram() {
    // Any program will do; we just need `program: Some(_)` so render
    // doesn't panic.
    let mut computer = Computer::new();
    computer.load_source("halt\n").unwrap();

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();

    // Jump the selection to the last row of RAM. The renderer should
    // clamp the scroll window so this row is visible — meaning the
    // address of the last row appears in the rendered buffer.
    let total_rows = RAM / MEMORY_BYTES_PER_ROW;
    ui.memory_selected_row = total_rows - 1;
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let rendered = buffer_to_string(terminal.backend().buffer());
    let last_row_addr = RAM - MEMORY_BYTES_PER_ROW;
    assert!(
        rendered.contains(&last_row_addr.to_string()),
        "expected last row address {last_row_addr} in:\n{rendered}",
    );
}
