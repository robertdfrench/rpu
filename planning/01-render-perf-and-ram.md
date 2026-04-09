# Stage 1 — Fix `render_memory` perf and right-size RAM

**Status: shipped.** Commit `24097e2` (initial work) plus follow-ups:
page-jump for PgUp/PgDn, and the 64 KiB → 1 KiB pivot.

## What the problem was

The original reason RAM was shrunk to 256 bytes was that the TUI got
sluggish at larger sizes. The bottleneck wasn't ratatui's `Table`
widget — it was how we fed it: `render_memory()` in `src/main.rs` built
**every** row in RAM on **every** frame, even though only a handful
were visible.

```rust
// old src/main.rs ~ render_memory()
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
At 64 KiB it would have been ~73,728 allocations per frame. Linear
in RAM size — that was the bottleneck.

## What shipped

### 1. Windowed `Paragraph` for `render_memory`

`render_memory` now builds at most `area.height` rows per frame,
regardless of RAM size. Switched from `Table` to `Paragraph` because
it gives us per-line `Span` control (cheap region coloring later in
stage 5) and the same primitive will be reusable for the pixel screen
in stage 7.

The memory pane state went from `TableState` (with its built-in
selection model) to a single `memory_selected_row: usize` on
`Computer`. The renderer centers the selection in the visible window
and clamps to bounds.

### 2. Page-jump for PgUp/PgDn

PgUp/PgDn used to move 1 row at a time. They now jump by the
visible page size, computed by `render_memory` each frame and
written back to `Computer.memory_page_rows` for the next key event
to read.

### 3. RAM size: 1 KiB (not 64 KiB)

Originally aimed for 64 KiB so that every `u16` register value would
be a valid address. After implementing it, even with the perf fix,
**8,192 rows is too many to ever navigate through** — at ~26 visible
rows per page, top-to-bottom is 300+ keypresses. Fast frames don't
fix "too much stuff to scroll".

Pivoted to **1 KiB** (`RAM = 1024`). Reasoning:

- The largest current example (`08.99_bottles_of_beer.s`) is well
  under 300 bytes; 1 KiB has comfortable headroom.
- A few page-downs covers the whole memory pane.
- Programs that walk past the end now get a clean
  `AddressOutOfBounds` error (see #4 below) instead of a panic, so
  the boundary work wasn't wasted.
- If a real example demands more later, it's one constant in
  `core.rs` to bump.

### 4. `u16` boundary arithmetic in `core.rs`

The bounds-check work was done while RAM was still 64 KiB and is
left in place because it's still correct (and cheap) at any RAM
size:

- New `ExecutionError::AddressOutOfBounds(u16)` carries the
  offending base address.
- `read` and `write` instructions now compute the high byte index
  in `usize` and bounds-check, instead of doing `addr + 1` in `u16`
  (which would have panicked in debug at `addr == 0xFFFF` if RAM
  ever grew that large).
- `execute_single_instruction` bounds-checks the 4-byte fetch.
- The `pc += 4` increment is done in `usize`; if the next `pc` would
  land at or past `RAM`, the CPU auto-halts cleanly.

### 5. Cosmetic

`Core::memory`'s doc comment used to claim "16K of RAM" (it was
256 bytes). Fixed.

## What's still latent

- **`sp` user-mischief.** A program that does `put 1 sp` followed
  by `push` will compute `sp - 2` as a u16 underflow (already
  caught by the `sp == 0` check) — but `put u16::MAX sp` followed
  by `pop` will compute `sp + 2` and overflow. Pre-existing at any
  RAM size; the natural execution path can't hit it. Not worth
  fixing in isolation; revisit if/when we add an instruction-level
  invariant pass.

## Test plan (manual, executed)

- [x] `cargo test` — 21/21 passing.
- [x] `cargo build` — succeeds with the 1 pre-existing
      `common_block` lifetime warning (unchanged).
- [x] User confirmed page-jump works and 1 KiB feels right.

## Notes for future me

- The windowed-rendering approach is the right shape regardless of
  RAM size. Don't undo it if you grow RAM later.
- The `AddressOutOfBounds` variant is the natural place to extend
  for any future bounds-check failures (jump-to-out-of-bounds,
  etc.).
- Stage 5 (RAM navigation) should make change-visualization the
  centerpiece — see the brainstorm in the conversation log around
  the 1 KiB pivot. The actually-useful features at 1 KiB are
  per-step change highlighting, dirty-bit heatmaps, and a watch
  list of pinned addresses, *not* fancier scrolling.
