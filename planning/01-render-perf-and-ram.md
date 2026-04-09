# Stage 1 — Fix `render_memory` perf and grow RAM

## Why this is first

The original reason RAM was shrunk to 256 bytes was that the TUI got
sluggish at larger sizes. The bottleneck isn't ratatui's `Table` widget,
it's how we feed it: `render_memory()` in `src/main.rs` builds **every**
row in RAM on **every** frame, even though only a handful are visible.

```rust
// src/main.rs ~ render_memory()
for (addr, byte) in memory.iter().enumerate() {
    current_row.push(format!("{:5}", byte));   // String alloc per byte
    if (addr + 1) % 8 == 0 {
        rows.push(Row::new(current_row).style(style));   // Row alloc per 8
        ...
    }
}
let table = Table::new(rows, widths)...
```

At 256 bytes that's 32 rows × 9 `String`s ≈ 288 allocations per frame.
At 64 KiB it would be ~73,728 allocations per frame. Linear in RAM
size — that's the bottleneck.

Fix this once and growing RAM becomes a one-line change.

## What to change

### 1. Replace the `Table` with a windowed `Paragraph`

`Paragraph` is a better fit here because:

- We can build only the visible lines.
- Color spans (region overlay in stage 5) become trivial.
- Same primitive will be useful for the pixel screen later.

The shape:

```rust
fn render_memory(
    memory: &[u8],
    scroll: &mut MemoryScroll,    // see below
    area: Rect,
    frame: &mut Frame,
    title: &str,
) {
    // How many byte-rows fit in the visible area, accounting for the
    // border (2 rows) and the header (1 row).
    let visible_rows = area.height.saturating_sub(3) as usize;
    let bytes_per_row = 8;

    // Clamp the scroll offset so we never scroll past the end.
    let max_offset = memory.len().saturating_sub(visible_rows * bytes_per_row);
    scroll.offset = scroll.offset.min(max_offset);
    let start = scroll.offset - (scroll.offset % bytes_per_row);

    // Header line.
    let mut lines: Vec<Line> = Vec::with_capacity(visible_rows + 1);
    lines.push(Line::from(Span::styled(
        " ADDR    +0   +1   +2   +3   +4   +5   +6   +7",
        Style::new().bold(),
    )));

    // Only build the rows that will actually be drawn.
    for row_idx in 0..visible_rows {
        let row_start = start + row_idx * bytes_per_row;
        if row_start >= memory.len() { break; }

        let row_end = (row_start + bytes_per_row).min(memory.len());
        let mut spans: Vec<Span> = Vec::with_capacity(bytes_per_row + 1);
        spans.push(Span::raw(format!("{:>5} ", row_start)));
        for byte in &memory[row_start..row_end] {
            spans.push(Span::raw(format!(" {:>4}", byte)));
        }
        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines)
        .block(common_block(title));
    frame.render_widget(paragraph, area);
}
```

`MemoryScroll` is a small replacement for the current `TableState`:

```rust
pub struct MemoryScroll {
    pub offset: usize,   // byte index of the topmost visible row
}
```

PgUp / PgDn handlers in `main.rs` mutate `offset` directly (in
multiples of `bytes_per_row`). Arrow keys come in stage 5.

### 2. Bump `RAM`

`src/core.rs:10`:

```rust
pub const RAM: usize = 65_536;   // u16::MAX as usize + 1
```

Also fix the lying doc comment on `Core::memory` (`src/core.rs:50`)
which currently says "16K of RAM".

### 3. Audit `u16` arithmetic for boundary overflow

With `RAM == 65_536`, addresses span the full `u16` range. A few spots
to look at:

- `core.rs::push` — `sp - 2` when `sp == 0` already errors with
  `StackOverflow`. Good.
- `core.rs::pop` — `sp + 2` when `sp == RAM - 2` now equals `65_534 +
  2 == 65_536`, which **overflows `u16`**. Currently masked because
  `RAM - 2 == 254` fits. Fix: do the bounds check before the add, or
  use `checked_add`.
- `core.rs::write` and `read` — both index `memory[(addr + 1) as
  usize]`. If `addr == u16::MAX`, this overflows. Either reject odd
  boundary addresses or do the addition in `usize`.
- `RegisterFile::sp` initial value — currently `RAM - 2` as a literal;
  make sure it's computed, not hardcoded.

These were all latent bugs even at 256 bytes; they just couldn't be
hit because the values never got close to the boundary. Worth a
comment in each fix explaining what changed.

## Test plan

This stage lands before the test scaffolding stage, so tests are
limited to what we already have. Manual checks:

- [ ] `cargo run --example` (or however you currently run it) with
      `examples/01.print_5.s`. Memory pane shows the program bytes at
      the top, scrolls smoothly with PgDn through 64 KiB.
- [ ] Stepping through `08.99_bottles_of_beer.s` feels as responsive
      as it does today at 256 bytes.
- [ ] `cargo test` still passes. The existing `test_memory` and
      stack tests should be unaffected.
- [ ] Briefly run with a debug print of frame time before/after the
      `render_memory` change to confirm the win (optional, but
      satisfying).

In stage 3 we'll add a render test that asserts the memory pane only
draws `visible_rows` rows regardless of `RAM`, which is the real
guarantee we want.

## Done when

- [ ] `render_memory` builds at most `area.height` rows per frame.
- [ ] `RAM == 65_536`.
- [ ] All `u16` boundary arithmetic in `core.rs` is overflow-safe.
- [ ] Existing tests pass.
- [ ] Manual scroll through 64 KiB feels smooth.

## Notes for future me

- Don't add the cursor / region coloring / decoded sidebar here. Those
  are stage 5 and they'll be much easier to add to a `Paragraph`-based
  renderer than a `Table`-based one — which is exactly the point of
  doing this refactor first.
- If `Paragraph` turns out to be the wrong choice for some reason
  (e.g. you want per-cell selection later), the windowed approach also
  works with `Table`: just slice the rows you build to the visible
  range. The perf fix is "build less", not "use Paragraph
  specifically".
