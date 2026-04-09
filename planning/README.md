# RPU improvement plan

A staged plan for cleaning up `rpu` and adding new capabilities. Each
stage is independently shippable and (mostly) independently testable.
Stages are ordered so that earlier work makes later work easier; doing
them out of order is possible but will cost you.

## Goals (in the user's own words)

1. **Make RAM navigable.** The thing that got me hung up last time. I
   want to grow RAM back to something useful and still be able to find
   my way around it.
2. **Pixel screen output device.** A box in the TUI where the program
   can plot ASCII-block "pixels".
3. **Input.** Either keyboard events or line-by-line input from a TUI
   prompt.
4. **Disk (stretch).** Persist data to a real file on the host
   filesystem, read/write a byte (or chunk) at a time.
5. **Better, less brittle tests.** Two layers: (a) headless tests
   that exercise the CPU directly without any TUI, and (b) render
   tests for the things that are actually visual, using ratatui's
   `TestBackend`.

## Stages

Tackle these in order unless noted.

| #   | Stage                                                            | Why it comes here                                                                                                               |
| --- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| 1   | [Fix `render_memory` perf & grow RAM](01-render-perf-and-ram.md) | The original blocker. One-file change, immediate quality-of-life win, unblocks everything else that touches RAM.                |
| 2   | [Headless `Computer` API](02-headless-computer.md)               | Top-level struct that owns Core + Devices + Program. Makes the CPU testable without any TUI at all. **The most leveraged stage.** |
| 3   | [TUI render test scaffolding](03-tui-test-scaffolding.md)        | Extract `render()` so it's callable with `TestBackend`. Now narrow: only for things that are actually visual.                    |
| 4   | [Device table refactor](04-device-table-refactor.md)             | Replace the hardcoded `match dvc { 0 => ..., 1 => ..., _ => tty }` with indexed dispatch. Unblocks new devices.                 |
| 5   | [RAM navigation features](05-ram-navigation.md)                  | Region coloring, cursor, jump-to-address, auto-follow PC/SP, decoded-instruction sidebar.                                       |
| 6   | [Input device + `rdy` flag](06-input-and-status-flag.md)         | Adds the `in` pseudo-register and a one-bit ready flag. First device that uses `Device::read()`.                                |
| 7   | [Pixel screen device](07-pixel-screen.md)                        | New `Screen` device with stateful cursor protocol. Renders with half-block characters.                                          |
| 8   | [Disk device (stretch)](08-disk.md)                              | Stream-style file device behind a CLI flag. Last because nothing depends on it.                                                 |

## Open decisions

These come up across multiple stages. Decide before starting the
relevant stage; I've left a recommendation in each stage doc.

- **Final RAM size.** Recommendation: 64 KiB (`u16::MAX as usize + 1`),
  so every 16-bit register value is a valid address.
- **What `Device::read()` returning `None` means.** Recommendation: a
  new `rdy` flag register set after each `copy in <reg>`, so polling
  loops can branch on `jump WAIT rdy`.
- **Snapshot tests vs. hand-asserted cells.** Recommendation: start
  hand-asserted (no new dep), reach for `insta` only if you find
  yourself writing the same boilerplate three times.

## Non-goals

Things that came up in brainstorming and were explicitly rejected, so
they don't drift back in by accident:

- **Assembler sugar for device selection** (e.g. `out lcd0 gp0`
  desugaring to `put 0 dvc / copy gp0 out`). Rejected: a teaching tool
  should be explicit about every instruction. The two-instruction form
  stays.

## How to use these docs

Each stage file is self-contained: context, what to change, code
sketches where useful, a test plan, and a "done when" checklist. Edit
them as you go — if reality diverges from the plan, the plan is the
thing that's wrong.
