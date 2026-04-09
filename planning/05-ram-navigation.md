# Stage 5 — RAM change visualization (and a little navigation)

## Why this stage exists at all

The original framing of this stage was "RAM navigation" — cursor,
arrow keys, jump-to-address, follow-PC, etc. That was the right
answer when RAM was 64 KiB and the problem was *"students can't
find their way around 8,192 rows"*.

After the stage 1 pivot to **1 KiB**, navigation isn't really a
problem anymore. The whole memory pane is ~128 rows; a few
PgDn/PgUp presses cover the entire address space. What students
*still* can't see is **what's changing** as the program runs. Step
by step, the LCD updates and the registers update, but the bytes
in RAM are an undifferentiated grid of small numbers — nothing
visually says "this byte just got written" or "this region holds
the program" or "this is the stack."

This stage replaces the original navigation focus with a
**change-visualization** focus. The killer feature is per-step
highlighting of bytes that just changed. Everything else is
supporting cast.

## Prerequisites

By now we have:

- A constant-cost `render_memory` (stage 1).
- 1 KiB of RAM (stage 1).
- A headless `Computer` that owns the program (stage 2).
- `Program.labels` exposed for the optional jump-to-label feature
  (stage 2).
- A render function callable from `TestBackend` tests, with
  `UiState` as a separate struct from `Computer` (stage 3).

Nothing in this stage requires touching `core.rs`.

## Sub-stages

Roughly ordered by pedagogical value. Earlier sub-stages are the
ones that actually answer the user complaint; later sub-stages are
nice-to-have polish that may or may not be worth doing depending
on how things feel after 5a-5c land.

### 5a — Per-step change highlighting (the killer feature)

After every `Computer::step()`, snapshot RAM. On the next render,
any byte that differs from the snapshot gets a flash style (e.g.
white-on-blue, bright background). The flash lasts for one render,
then the snapshot updates.

Students press `n` and *immediately see the byte that just got
written*. That's the entire pedagogical goal of this stage in one
feature.

State to add (on `Computer`, not `UiState` — the snapshot is part
of CPU history, not view state):

```rust
pub struct Computer {
    pub core: Core,
    pub devices: Devices,
    pub program: Option<Program>,
    /// Snapshot of `core.memory` taken at the *start* of the most
    /// recent `step()`. Compared against the live memory at render
    /// time to highlight bytes that just changed.
    last_step_memory: [u8; RAM],
}
```

Update inside `step()`:

```rust
pub fn step(&mut self) -> Result<(), ExecutionError> {
    self.last_step_memory = self.core.memory;
    let mut slice = self.devices.as_slice();
    self.core.execute_single_instruction(&mut slice)?;
    Ok(())
}
```

A small accessor for the renderer:

```rust
impl Computer {
    /// Returns true if the byte at `addr` differs from its value
    /// at the start of the most recent step.
    pub fn byte_changed(&self, addr: u16) -> bool {
        self.core.memory[addr as usize]
            != self.last_step_memory[addr as usize]
    }
}
```

In `render_memory`, before formatting each byte:

```rust
let changed = computer.byte_changed(addr as u16);
let span_style = if changed {
    Style::new().white().on_blue().bold()
} else {
    base_style_for(addr, computer)   // see 5b
};
```

That's the whole feature. Maybe 40 lines including the snapshot
field, the accessor, and the render-side branch.

**Tests** (`tests/render.rs`):

```rust
#[test]
fn changed_bytes_are_highlighted_after_step() {
    let mut computer = Computer::new();
    computer.load_source(
        "put 42 gp0\n\
         put 100 gp1\n\
         write gp0 gp1\n\
         halt\n",
    ).unwrap();
    computer.step().unwrap();   // put 42 gp0   — no memory change
    computer.step().unwrap();   // put 100 gp1  — no memory change
    computer.step().unwrap();   // write gp0 gp1 — memory[100..102] changes

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();
    ui.memory_selected_row = 100 / 8;
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    // Find the cell rendering byte 100; assert its bg is blue.
    let cell = find_byte_cell(terminal.backend().buffer(), 100);
    assert_eq!(cell.style().bg, Some(Color::Blue));
}

#[test]
fn unchanged_bytes_are_not_highlighted_after_step() {
    // ... same setup, then assert byte 50 (untouched) has the
    //     default background.
}
```

### 5b — Region coloring

Tint memory bytes by what region they're in, so students can see
the *shape* of the program even when nothing is changing. No new
state — derived from `Computer` each frame.

| Region   | Source                           | Style suggestion       |
| -------- | -------------------------------- | ---------------------- |
| Program  | `0..program.size()`              | dim green              |
| Current  | `pc..pc+4`                       | bold yellow background |
| Stack    | `(sp + 2)..=(RAM - 2)`           | dim cyan               |
| Unused   | (everything else)                | default / dim          |

Per-step change highlighting (5a) wins over region coloring when
they overlap — a byte that just changed is shown as "changed", not
"program byte".

```rust
fn region(addr: usize, computer: &Computer) -> Region {
    let pc = computer.core.register_file.pc as usize;
    if (pc..pc+4).contains(&addr) { return Region::Current; }
    let sp = computer.core.register_file.sp as usize;
    if addr >= sp + 2 && addr <= RAM - 2 { return Region::Stack; }
    if let Some(p) = &computer.program {
        if addr < p.size() { return Region::Program; }
    }
    Region::Unused
}
```

This is the feature that makes "where is the stack?" visually
obvious.

### 5c — Dirty-bit heatmap (ever-touched memory)

A `[bool; RAM]` of "has this byte ever been written since
power-on". Touched bytes render in a brighter shade than untouched
bytes. Lets students see the *cumulative footprint* of execution,
which is interesting for programs that use scratch memory.

Lives on `Computer`. Updated by an instrumented memory write helper
that wraps the existing direct `memory[i] = ...` lines in `core.rs`
(the `write` and `push` instructions, plus `load_program`).

This is a lower-priority addition — it's a nice-to-have on top of
the change highlight in 5a, not a substitute for it. Skip it
entirely if the per-step highlight already feels sufficient.

### 5d — Watch list / pinned addresses (optional)

A small sidebar pane in the right column showing only addresses
the user has pinned, regardless of where the memory pane is
scrolled. Lets students keep an eye on specific scratch variables
without losing the main view.

```rust
pub struct UiState {
    // ...
    pub watched: Vec<u16>,
}
```

Key like `w` toggles "pin the byte at the cursor" (requires the
optional 5e cursor below). Or the watch list could be hardcoded
per-program via a future `# watch 100` directive in source — but
that's source-format work, defer.

Skip this entirely if no example program produces interesting
scratch state worth pinning.

### 5e — Cursor (only if needed by 5d, or for completeness)

Add a `memory_cursor: u16` to `UiState` and arrow-key navigation
inside the memory pane (toggled in/out of "memory focus" with a
key like `m`, so arrows still scroll the code window normally).
The cursor renders as a fifth region in the classifier from 5b
and wins ties.

Only worth doing if 5d (watch list) wants a way to pick addresses
interactively. Otherwise skip — at 1 KiB the page-jump from stage
1 is already enough.

### 5f — Decoded instruction sidebar (optional polish)

When some address is "in focus" (whether via cursor or just at the
PC), decode those 4 bytes via `Instruction::try_from_u32` and show
the disassembled form in a small pane. Pure read-only feature.

```rust
let bytes = u32::from_ne_bytes([
    computer.core.memory[addr],
    computer.core.memory[addr+1],
    computer.core.memory[addr+2],
    computer.core.memory[addr+3],
]);
let text = match Instruction::try_from_u32(bytes) {
    Ok(instr) => format!("{addr:>4}: {instr:?}"),
    Err(_)    => format!("{addr:>4}: (not a valid instruction)"),
};
```

Cute but the source pane already shows the source line for the
current PC, which is more readable than `Instruction::Debug`. Only
worth building if you want a way to disassemble *non-PC* addresses
(e.g. inspecting program bytes at a different location).

## Things I'm explicitly NOT proposing

These were in the original stage 5 plan; they don't earn their
keep at 1 KiB:

- **Jump-to-address prompt** (the `g` modal). At 1 KiB you can
  PgDn to anywhere in 5-6 keypresses. The modal would be more
  ceremony than it saves.
- **Auto-follow modes for PC and SP.** With change-highlighting
  (5a) the right thing is already visually obvious — you don't
  need the view to chase the cursor.
- **Hex/dec toggle and ASCII gutter.** Would have been nice at
  64 KiB. At 1 KiB the decimal display is already legible. Defer
  forever, or add when a student actually asks for it.

These are documented here so future-me doesn't get tempted to
revive them without rethinking the underlying problem.

## Test plan

For each shipped sub-stage, at least one render test in
`tests/render.rs`:

- [ ] **5a:** byte that just changed has the highlight style;
      byte that didn't change has the base style. Tests both
      branches with the same render call.
- [ ] **5a:** highlight clears after a second `step()` if the
      byte didn't change again.
- [ ] **5b:** byte at PC has the "current" style; byte in the
      stack region has the "stack" style; byte in unused memory
      has the dim style.
- [ ] **5c (if shipped):** dirty bit set after writing; cleared
      after... actually, dirty bits are cumulative — they're never
      cleared during a run. Test that an untouched byte renders
      dim and a written byte renders bright.

Plus a CPU-level test in `tests/cpu.rs`:

- [ ] `Computer::byte_changed(addr)` reflects only the most recent
      step, not the cumulative diff.

## Done when

- [ ] When a student presses `n`, they can immediately see which
      bytes changed. This is the headline; if it's not true at the
      end of this stage, the stage didn't ship.
- [ ] The program region, stack region, and current PC are
      visually distinct (5b).
- [ ] Whatever sub-stages you skipped are explicitly documented as
      skipped, not silently dropped.

## Notes for future me

- 5a is the *only* must-have. 5b is strongly recommended. 5c-5f
  are optional and you should not feel bad about shipping the
  stage without them.
- Don't conflate the per-step diff (5a) with the cumulative dirty
  bits (5c). They answer different questions: 5a is "what just
  happened", 5c is "what has ever happened". Different colors,
  different state.
- The `last_step_memory: [u8; RAM]` snapshot is 1 KiB per
  `Computer`. Negligible. Don't optimize it.
- If you find yourself wanting the user to be able to see the
  entire LCD history (not just `last_written()`) in the TUI,
  *that's not what `Lcd::history` is for*. It's a test affordance.
  If the TUI needs scrollback, design that separately.
