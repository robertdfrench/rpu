# Stage 5 — RAM navigation features

## Why this is here

This is the stage that actually solves the original "I can't navigate
the RAM" complaint. By now we have:

- A perf-fixed memory pane that builds only the visible window
  (stage 1).
- 1 KiB of RAM — small enough that scrolling is a non-issue and
  students can see the whole memory in a few page-downs (stage 1).
- A headless `Computer` (stage 2) that gives `program: Option<Program>`
  a natural home.
- A render function callable from tests (stage 3), so we can pin
  visual behavior down with `TestBackend`.

Everything in this stage is additive on the `Paragraph`-based memory
renderer from stage 1. None of it requires further refactoring of
`core.rs`.

## Sub-stages

The features below are roughly ordered by bang-for-buck. Each
sub-stage is independently shippable; pick a stopping point that
matches what you actually need.

### 4a — Region coloring

Tint memory bytes by what they are, no new state required:

| Region   | Source                              | Color suggestion       |
| -------- | ----------------------------------- | ---------------------- |
| Program  | `0..program.size()`                 | dim green              |
| Current  | `pc..pc+4`                          | bold yellow background |
| Stack    | `(sp + 2)..=(RAM - 2)`              | dim cyan               |
| Unused   | (everything else)                   | default / dim          |

For this to work, `Computer` needs to remember the loaded program's
size after `load_program`. Add `program: Option<Program>` (per stage
2's note) and use `program.as_ref().map(|p| p.size())`.

In `render_memory`, classify each byte before formatting and apply
a `Style` to its `Span`:

```rust
fn classify(addr: usize, computer: &Computer) -> Region {
    let pc = computer.core.register_file.pc as usize;
    if (pc..pc+4).contains(&addr) {
        Region::Current
    } else if let Some(p) = &computer.program {
        if addr < p.size() { return Region::Program; }
        // ... etc
    }
    // ...
}

fn style_for(region: Region) -> Style {
    match region {
        Region::Current => Style::new().yellow().bold().on_black(),
        Region::Program => Style::new().green().dim(),
        Region::Stack   => Style::new().cyan().dim(),
        Region::Unused  => Style::new().dim(),
    }
}
```

That's the whole feature. It's the highest-leverage thing in the
stage and it costs you maybe 30 lines.

**Test (using stage 3 scaffolding):**

```rust
#[test]
fn current_instruction_is_highlighted() {
    let mut computer = Computer::new();
    computer.load_source("put 7 gp0\nhalt\n").unwrap();
    // PC is at 0; bytes 0..4 should be the "current" region.

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut ui = UiState::default();
    terminal.draw(|f| render(f, &computer, &mut ui)).unwrap();

    let buffer = terminal.backend().buffer();
    // Find a cell that contains the byte at address 0 and check it
    // has the "current" highlight style. (Helper: find_cell_at_addr.)
    let cell = find_byte_cell(buffer, 0);
    assert_eq!(cell.style().fg, Some(Color::Yellow));
}
```

### 4b — Cursor + arrow-key navigation

Add a separate cursor inside the memory pane, distinct from
PgUp/PgDn scrolling:

```rust
pub struct UiState {
    pub mem_scroll: MemoryScroll,
    pub mem_cursor: u16,        // byte index, 0..RAM
    pub mem_focus: bool,        // true when arrow keys move cursor
}
```

When `mem_focus` is on, arrow keys move the cursor and auto-scroll
the pane to keep it visible. A key like `m` toggles focus into the
memory pane; `Esc` exits. Without focus, arrow keys do whatever
they currently do (navigate code, presumably).

The cursor is rendered as a fifth region in the classifier above
(`Region::Cursor`), drawn on top so it wins ties with `Current`.

### 4c — Decoded instruction sidebar

When the cursor is on a 4-aligned address, decode those 4 bytes and
show what they'd execute as in a tiny pane. Reuses
`Instruction::try_from_u32`.

```rust
fn render_decoded(frame: &mut Frame, computer: &Computer, ui: &UiState, area: Rect) {
    let addr = ui.mem_cursor as usize;
    let bytes = if addr + 4 <= computer.core.memory.len() {
        let mut b = [0u8; 4];
        b.copy_from_slice(&computer.core.memory[addr..addr+4]);
        Some(u32::from_ne_bytes(b))
    } else { None };

    let text = match bytes.and_then(|w| Instruction::try_from_u32(w).ok()) {
        Some(instr) => format!("{addr:>5}: {instr:?}"),
        None        => format!("{addr:>5}: (not a valid instruction)"),
    };
    frame.render_widget(
        Paragraph::new(text).block(common_block("Decoded")),
        area,
    );
}
```

The pane lives where you have room — probably squeezing the help
panel down by a line or two, or adding a row to the right column
under the special registers. Layout decision; not blocking.

### 4d — Jump-to-address prompt

Press `g`, get a small input modal at the bottom of the screen,
type an address (decimal, `0xNN` hex, or `.LABEL`), enter to jump
the cursor there.

```rust
pub enum UiMode {
    Normal,
    GotoPrompt { input: String },
}
```

In `render`, if `ui.mode` is `GotoPrompt`, draw a one-line
`Paragraph` over the bottom of the screen showing `:goto > {input}`.
Key handling switches based on mode.

Address parsing:

```rust
fn parse_address(s: &str, program: Option<&Program>) -> Option<u16> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("0x") {
        u16::from_str_radix(rest, 16).ok()
    } else if let Some(label) = s.strip_prefix('.') {
        program?.label_address(label)
    } else {
        s.parse::<u16>().ok()
    }
}
```

For label support, `Program` needs to retain its symbol table after
compile — currently it might throw it away. Worth checking
`programs.rs` and adding `pub labels: HashMap<String, u16>` if
missing. The cost is negligible (a small map per program) and it
unlocks both this feature and the decoded sidebar showing label
names.

### 4e — Auto-follow modes

Toggle keys `F p` (follow PC) and `F s` (follow SP). When active,
`mem_scroll.offset` is recomputed each frame to keep the followed
register's address visible (and centered, ideally).

```rust
pub enum FollowMode { Off, Pc, Sp }

// In render_memory, before clamping the offset:
match ui.follow {
    FollowMode::Pc => ui.mem_scroll.offset = center_on(computer.core.register_file.pc, visible_rows),
    FollowMode::Sp => ui.mem_scroll.offset = center_on(computer.core.register_file.sp, visible_rows),
    FollowMode::Off => {}
}
```

This is the feature you'll use most while debugging your own programs.

### 4f — Hex/dec toggle and ASCII gutter

Press `x` to toggle hex/dec for the byte values; always show an
ASCII gutter on the right side (`. ` for non-printable). Polish; do
last.

```
 ADDR    +0   +1   +2   +3   +4   +5   +6   +7   ASCII
     0   0a   04   00   00   0a   05   00   01   ........
     8   0a   06   00   02   01   00   00   00   ........
```

## Test plan

For each sub-stage, add a render test against `TestBackend`:

- [ ] **4a:** styled cell at PC has yellow fg.
- [ ] **4a:** styled cells in stack region have cyan fg.
- [ ] **4b:** cursor cell is visually distinct; arrow key advances
      `ui.mem_cursor` by 1.
- [ ] **4b:** moving cursor past the visible window auto-scrolls.
- [ ] **4c:** decoded sidebar shows the right `Instruction` for the
      cursor's aligned address.
- [ ] **4d:** parsing tests for `parse_address` (decimal, hex,
      label, garbage).
- [ ] **4d:** integration test: press `g`, type `0x10`, enter,
      `ui.mem_cursor == 0x10`.
- [ ] **4e:** with `FollowMode::Pc`, scrolling is forced to show PC.
- [ ] **4f:** hex toggle changes "0a" ↔ "10" in rendered output.

## Done when

- [ ] You can load any example, scan through all 1 KiB, and never
      lose track of where the program / stack / current instruction
      are.
- [ ] You can jump to any address by name or number.
- [ ] When you don't want to think about it, follow-PC mode just
      keeps the right thing on screen.
- [ ] Each sub-stage has at least one render test pinning its
      behavior down.

## Notes for future me

- The cursor and the scroll offset are *separate concerns*. Don't
  conflate them: `mem_scroll` is "what's visible", `mem_cursor` is
  "what's selected". Auto-scroll is a derived behavior that adjusts
  scroll to keep cursor visible.
- It's tempting to make every sub-stage configurable (color schemes,
  bytes-per-row, etc). Resist. Hardcode reasonable defaults; add
  config only when a real need shows up.
- `Region` classification will be reused by the screen-device
  framebuffer view in stage 7 if you generalize it. Don't generalize
  preemptively, but if you find yourself writing the same `match` a
  second time, that's the signal.
