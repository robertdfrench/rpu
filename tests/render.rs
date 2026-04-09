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
