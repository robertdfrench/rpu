# Stage 3 — TUI render test scaffolding

## Why this is here (and why it's now smaller)

Stage 2 (the headless `Computer` API) means **most** future tests
don't need to touch the TUI at all. CPU behavior, device protocols,
input handling, and disk persistence are all unit tests against
`Computer` in `tests/cpu.rs`.

But there's still a category of behavior that's *only* visible
through rendering: layout, region coloring (stage 5), the screen
device's pixel output (stage 7), input prompt mode (stage 6). For
those, we need a way to exercise `render()` without a real terminal.

Ratatui ships exactly the tool: `ratatui::backend::TestBackend` is
a backend that writes into an in-memory `Buffer`. You construct a
`Terminal<TestBackend>`, call `terminal.draw(...)`, and inspect the
resulting buffer cell-by-cell or as a string.

## What's blocking us today

After stage 2, `src/main.rs` looks like this (boiled down):

```rust
struct App {
    computer: Computer,
    code_list_state: ListState,
    memory_selected_row: usize,
    memory_page_rows: usize,
}

fn render(app: &mut App, frame: &mut Frame) { ... }
```

Two things make this untestable from `tests/`:

1. **`render` and `App` live in `main.rs`, which is the binary, not
   the library.** Integration tests in `tests/` can only call into
   the library (they `use rpu::...`), so they can't see anything in
   `main.rs`.
2. **`render` takes `&mut App`, which couples the headless CPU
   (`Computer`) to the TUI view state in one struct.** Tests want
   to drive the CPU through `Computer` and the renderer through
   *just* the view state, separately.

Both problems get fixed in this stage with one refactor.

## What to change

### 1. Move the render code into the library

Create `src/tui.rs` and move `render`, `Layouts`, all the
`render_*` helpers, and the new `UiState` (see below) into it. The
library now has a `tui` module that depends on ratatui — that's
fine, ratatui is already a binary dependency, this just promotes it
to a library dependency. The headless `Computer` API still doesn't
import ratatui, so users (and tests) that only want the CPU pay
nothing.

`Cargo.toml` doesn't need to change — `ratatui` is already a normal
dependency.

`src/lib.rs` adds:

```rust
pub mod tui;
pub use tui::{render, UiState};
```

### 2. Split `App` into `Computer` + `UiState`

`UiState` holds the TUI view state currently sitting on `App`:

```rust
// src/tui.rs
#[derive(Default)]
pub struct UiState {
    pub code_list_state: ListState,
    pub memory_selected_row: usize,
    pub memory_page_rows: usize,
    // Stage 5 may add: mem_cursor (optional), watched (optional)
    // Stage 6 will add: input_mode (Normal / InputPrompt)
}
```

`render` takes them separately:

```rust
pub fn render(
    frame: &mut Frame,
    computer: &Computer,
    ui: &mut UiState,
) {
    let layouts = Layouts::compute(frame.area());
    render_code(frame, computer, &mut ui.code_list_state, layouts.code);
    render_memory(
        frame,
        &computer.core.memory,
        ui.memory_selected_row,
        &mut ui.memory_page_rows,
        layouts.memory,
    );
    render_lcd(frame, &computer.devices.lcd0, layouts.lcd0, "LCD0 (dvc 0)");
    render_lcd(frame, &computer.devices.lcd1, layouts.lcd1, "LCD1 (dvc 1)");
    render_printer(frame, &computer.devices.tty, layouts.printer);
    // ... etc
}
```

Notice `render` takes `&Computer`, not `&mut Computer`. Rendering
must never mutate CPU state — that property is what makes it safe
to call from tests at arbitrary points.

### 3. Slim `App` (or delete it)

After the move, the only thing left in `main.rs` is the event loop.
The `App` struct can either:

- **Stay as a thin wrapper** (`App { computer: Computer, ui: UiState }`)
  to keep the event loop tidy, or
- **Disappear entirely**, with `main()` holding `computer` and `ui`
  as locals.

Either is fine. The wrapper costs four lines and reads slightly
better in the event loop, so I'd lean toward keeping it as a
shrunken `App`. The important thing is that the *fields* now live
in `UiState` (in the library) where tests can construct them.

The event loop becomes:

```rust
fn run(mut terminal: DefaultTerminal, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| rpu::render(f, &app.computer, &mut app.ui))?;
        match event::read()? {
            Event::Key(ke) => match ke.code {
                KeyCode::PageDown => {
                    let total_rows = rpu::core::RAM / 8;
                    let last = total_rows.saturating_sub(1);
                    let jump = app.ui.memory_page_rows.max(1);
                    app.ui.memory_selected_row =
                        (app.ui.memory_selected_row + jump).min(last);
                }
                // ... etc
                KeyCode::Char('n') => match app.computer.step() {
                    Ok(()) => {}
                    Err(e) => app.computer.devices.tty
                        .push_line(&format!("{:?}", e)),
                },
                _ => {}
            },
            _ => {}
        }
    }
}
```

`Layouts` moves into `tui.rs` alongside `render` and stops being
publicly visible — it's an implementation detail.

### 4. First `TestBackend` smoke test

New file `tests/render.rs`:

```rust
use ratatui::{backend::TestBackend, Terminal};
use rpu::{render, Computer, UiState};

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
    let source = std::fs::read_to_string("examples/02.add_5_7.rpu").unwrap();
    let mut computer = Computer::new();
    computer.load_source(&source).unwrap();
    computer.run_to_halt().unwrap();

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let rendered = buffer_to_string(terminal.backend().buffer());
    assert!(
        rendered.contains("00012"),
        "expected '00012' (5+7) somewhere in rendered output:\n{rendered}",
    );
}
```

This is deliberately loose: it asserts on a substring, not on cell
positions. Adding a border or shifting a panel by a column doesn't
break it; only changing what's *displayed* does. Tighter tests come
later when there's something specific worth pinning down (e.g.
"the PC highlight is on the row containing the current
instruction").

`buffer_to_string` will be reused enough that it should probably
live in a `tests/common/mod.rs` helper module. Fine to inline for
now and extract on the second use.

### 5. Second smoke test: memory pane scrolls to the end of RAM

This is the test that pins down stage 1's perf fix and the
windowed renderer:

```rust
#[test]
fn memory_pane_scrolls_to_the_end_of_ram() {
    let mut computer = Computer::new();
    computer.load_source("halt\n").unwrap();

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();

    // Jump the selection to the last row of RAM.
    ui.memory_selected_row = (rpu::core::RAM / 8) - 1;
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let rendered = buffer_to_string(terminal.backend().buffer());
    let last_row_addr = rpu::core::RAM - 8;
    assert!(
        rendered.contains(&last_row_addr.to_string()),
        "expected last row address {last_row_addr} in:\n{rendered}",
    );
}
```

The real perf assertion is informal: at 1 KiB this test runs in
microseconds. If it ever takes more than a few milliseconds,
something is badly wrong.

## Test plan

- [ ] `cargo test` passes (all of unit + `tests/cpu.rs` + the new
      `tests/render.rs`).
- [ ] Manually run the TUI — behavior unchanged after the
      `render` / `UiState` extraction.
- [ ] Deliberately break something visible (e.g. comment out the LCD
      render call) and confirm the smoke test catches it.

## Done when

- [ ] `src/tui.rs` exists with `render`, `Layouts`, all the
      `render_*` helpers, and `UiState`.
- [ ] `render` is callable with just `&Computer` + `&mut UiState` +
      `&mut Frame`. No live `Terminal` required.
- [ ] `UiState` is exported from `src/lib.rs`.
- [ ] `src/main.rs` is short enough to read on one screen — only
      the event loop, key handling, and CLI parsing remain.
- [ ] `tests/render.rs` exists with at least two `TestBackend`
      smoke tests.

## Notes for future me

- ratatui being in the library API is *fine*. The headless
  `Computer` doesn't import it, so users / tests / future tools
  that only want the CPU still pay nothing for it.
- Resist the urge to write a render test for *every* pane in this
  stage. Two smoke tests are enough to prove the scaffolding works;
  more focused tests should be added alongside the features they
  cover (change-highlight tests in stage 5, screen tests in stage
  7).
- Snapshot tests with `insta` are nice but add a dep and a review
  workflow (`cargo insta review`). Don't reach for them until
  you've written 3+ render tests by hand and felt the boilerplate.
- The headless tests from stage 2 cover *what the program did*. The
  render tests in this stage cover *how that's shown to the user*.
  These are separate concerns; don't blur them by writing render
  tests that try to also verify CPU behavior. If you find yourself
  doing that, write the CPU assertion as a `tests/cpu.rs` test
  instead.
- The `App` wrapper in `main.rs` is intentional — it lets the event
  loop borrow `&app.computer` and `&mut app.ui` separately without
  fighting the borrow checker. Don't flatten `App` into locals
  unless that becomes a problem; the current shape is small and
  readable.
