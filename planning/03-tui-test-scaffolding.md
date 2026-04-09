# Stage 3 — TUI render test scaffolding

## Why this is here (and why it's now smaller)

Stage 2 (the headless `Computer` API) means **most** future tests
don't need to touch the TUI at all. CPU behavior, device protocols,
input handling, and disk persistence are all unit tests against
`Computer`.

But there's still a category of behavior that's *only* visible
through rendering: layout, region coloring (stage 5), the screen
device's pixel output (stage 7), input prompt mode (stage 6). For
those, we need a way to exercise `render()` without a real terminal.

Ratatui ships exactly the tool for this:
`ratatui::backend::TestBackend` is a backend that writes into an
in-memory `Buffer`. You construct a `Terminal<TestBackend>`, call
`terminal.draw(...)`, and inspect the resulting buffer cell-by-cell
or as a string.

The cost of admission is one small refactor: pull `render` out of
the live `main` loop so it takes a `Frame` and a `&Computer`, with
no live terminal in scope.

## What to change

### 1. Extract `render` so it doesn't need a real terminal

Today (rough shape — confirm against actual code):

```rust
// src/main.rs
fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut computer = Computer::new();   // exists after stage 2
    // ... load source ...
    loop {
        terminal.draw(|frame| {
            // ... 200 lines of layout + render_* calls all inline ...
        })?;
        // ... handle key events ...
    }
}
```

Goal:

```rust
// src/tui.rs (new file) or top of main.rs
pub fn render(frame: &mut Frame, computer: &Computer, ui: &mut UiState) {
    let layouts = Layouts::compute(frame.area());
    render_code(frame, computer, layouts.code);
    render_memory(frame, &computer.core.memory, &mut ui.mem_scroll, layouts.memory);
    render_registers(frame, &computer.core.register_file, layouts.gp_regs, layouts.sp_regs);
    render_lcd(frame, &computer.devices.lcd0, layouts.lcd0);
    render_lcd(frame, &computer.devices.lcd1, layouts.lcd1);
    render_power(frame, computer.core.power, layouts.power);
    render_printer(frame, &computer.devices.tty, layouts.printer);
    render_help(frame, layouts.help);
}

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut computer = Computer::new();
    let mut ui = UiState::default();
    // ... load source ...
    loop {
        terminal.draw(|f| render(f, &computer, &mut ui))?;
        // ... handle key events ...
    }
}
```

Two things to notice:

- **`render` takes `&Computer`, not `&mut Computer`.** Rendering is
  pure(ish): it should never mutate CPU state. The only mutable
  argument is `UiState`.
- **`UiState` is new.** It absorbs the `TableState` (or
  `MemoryScroll` from stage 1) and any other UI-only state currently
  living as locals in `main()`. Pulling these into a struct means
  tests can construct one and call `render` without faking a key
  event loop.

```rust
#[derive(Default)]
pub struct UiState {
    pub mem_scroll: MemoryScroll,   // from stage 1
    // Stage 5 will add: mem_cursor, mem_focus, follow_mode
    // Stage 6 will add: mode (Normal / InputPrompt / GotoPrompt)
}
```

The `Layouts` struct already exists per the exploration report —
keep it, just make sure it's pure
(`Layouts::compute(area: Rect) -> Self`).

**Should `render` move into its own file?** Yes, eventually —
`src/tui.rs` for the render functions, leaving `main.rs` as just the
event loop. Worth doing in this stage if `main.rs` is already too
long to navigate. It is.

### 2. Add the first `TestBackend` smoke test

New file `tests/render.rs`:

```rust
use ratatui::{backend::TestBackend, Terminal};
use rpu::{render, Computer, UiState};

#[test]
fn lcd0_shows_result_after_running_add_program() {
    let source = std::fs::read_to_string("examples/02.add_5_7.s").unwrap();
    let mut computer = Computer::new();
    computer.load_source(&source).unwrap();
    computer.run_to_halt().unwrap();

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let buffer = terminal.backend().buffer();
    let rendered = buffer_to_string(buffer);

    assert!(
        rendered.contains("00012"),
        "expected '00012' (5+7) somewhere in rendered output:\n{rendered}",
    );
}

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
```

This is deliberately loose: it asserts on a substring, not on cell
positions. Adding a border or shifting a panel by a column doesn't
break it; only changing what's *displayed* does. Tighter tests come
later when there's something specific worth pinning down (e.g. "the
PC highlight is on the row containing the current instruction").

`buffer_to_string` will be reused enough that it should probably
live in a `tests/common/mod.rs` helper module. Fine to inline for
now and extract on the second use.

### 3. A second smoke test: the memory pane only renders visible rows

This is the test that pins down stage 1's perf fix. The memory pane
should only render `area.height` rows worth of memory data,
regardless of `RAM` size — *not* `RAM / 8` rows.

You can't directly assert "only N rows were built" through the
buffer (the buffer just shows what's drawn). But you *can* assert
that scrolling renders fast and that the visible content matches
expectations. A loose version:

```rust
#[test]
fn memory_pane_scrolls_to_the_end() {
    let mut computer = Computer::new();
    computer.load_source("halt\n").unwrap();   // smallest valid program

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();

    // Scroll to the last row of RAM.
    ui.mem_selected_row = (rpu::core::RAM / 8) - 1;
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let rendered = buffer_to_string(terminal.backend().buffer());
    // The address column should show the last row's start address.
    let last_row_addr = rpu::core::RAM - 8;
    assert!(rendered.contains(&last_row_addr.to_string()));
}
```

The real perf assertion is informal: if this test takes more than a
few milliseconds, something is wrong. (You can wrap the `draw` call
in a timer and `assert!(elapsed < Duration::from_millis(50))` if you
want the test to fail loudly on regression. It's a bit flaky on
loaded CI machines — use sparingly.)

## Test plan

- [ ] `cargo test` passes.
- [ ] `cargo test --test render` runs and passes.
- [ ] `cargo test --test cpu` (from stage 2) still passes.
- [ ] Manually run the TUI — behavior unchanged after the
      `render`/`UiState` extraction.
- [ ] Deliberately break something visible (e.g. comment out the LCD
      render call) and confirm the smoke test catches it.

## Done when

- [ ] `render` is callable with just `&Computer` + `&mut UiState` +
      `&mut Frame`. No live `Terminal` required.
- [ ] `UiState` exists and absorbs all UI-only state currently in
      `main()` locals.
- [ ] `tests/render.rs` exists with at least two `TestBackend`
      smoke tests (one for output, one for memory navigation).
- [ ] `render` and `UiState` are exported from `src/lib.rs`.
- [ ] (Bonus) `main.rs` is now short enough to comfortably read in
      one screen, with all render logic moved to `src/tui.rs`.

## Notes for future me

- Resist the urge to write a render test for *every* pane in this
  stage. Two smoke tests are enough to prove the scaffolding works;
  more focused tests should be added alongside the features they
  cover (region coloring tests in stage 5, screen tests in stage 7).
- Snapshot tests with `insta` are nice but add a dep and a review
  workflow (`cargo insta review`). Don't reach for them until you've
  written 3+ render tests by hand and felt the boilerplate.
- The headless tests from stage 2 cover *what the program did*. The
  render tests in this stage cover *how that's shown to the user*.
  These are separate concerns; don't blur them by writing render
  tests that try to also verify CPU behavior. If you find yourself
  doing that, write the CPU assertion as a `tests/cpu.rs` test
  instead.
- When stage 5 adds the cursor/region/follow features, *those*
  tests will be tighter (cell-level color assertions). The looseness
  here is appropriate for the smoke-test role.
