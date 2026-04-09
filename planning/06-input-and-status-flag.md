# Stage 6 — Input device and `rdy` status flag

## Why this is here

Today, nothing in the ISA pulls a value *from* a device. The
`Device` trait already has a `read()` method that returns
`Option<u16>` (`src/devices.rs:9`), but no opcode calls it. This
stage finally wires it up and adds the first input device.

The blocker was: what should `copy in <reg>` do when the input
buffer is empty? We can't return `0`, because `0` is a perfectly
valid byte the user might type. The answer is a one-bit status flag
register, `rdy`, set by the CPU after each input read.

This stage depends on the device table refactor (stage 4): the
`in` pseudo-register dispatches through the same indexed device
table that `out` does.

## Design decisions

### The `in` pseudo-register, mirroring `out`

Source syntax:

```
put 3 dvc       # select device 3 (the keyboard)
copy in gp0     # try to read one value from devices[3]
```

`in` is a new `RegisterName` variant. It's only legal as the
**source** of a `copy`, never as a destination (mirror of how `out`
is only legal as a destination). Any other use is a compile-time
error in `programs.rs` and a runtime `ExecutionError` if it slips
through.

Why a pseudo-register and not a new opcode? Two reasons:

1. **Symmetry.** `out` is the destination side; `in` is the source
   side. They're the same shape and can share most of the dispatch
   code in `core.rs::copy()`.
2. **Opcodes are precious.** We have a byte to spend on opcodes and
   want to keep the ISA at ~12 instructions for teaching. A new
   `RegisterName` variant is free.

### The `rdy` flag

A new register on the register file:

```rust
pub struct RegisterFile {
    // ... existing fields ...
    pub rdy: u16,   // 0 or 1, set after each `copy in <reg>`
}
```

Behavior of `copy in <reg>`:

1. Look up `devices[dvc]`.
2. Call `devices[dvc].read()`.
3. If `Some(val)`: write `val` into `<reg>`, set `rdy = 1`.
4. If `None`: leave `<reg>` unchanged, set `rdy = 0`.

`rdy` is readable like any other register, so it composes with
`jump`:

```
.WAIT
  copy in gp0       # try to read; sets rdy = 1 on success
  jump WAIT rdy     # jump-if-zero on rdy → loop while empty
  # gp0 now holds a real input byte
```

(Recall `jump ADDR COND` jumps if `COND == 0`, so this is a
"jump while not ready" loop. Comment the example clearly the first
time you write it; the inversion is the kind of thing students
trip on.)

`rdy` is **not** writable by user programs (like `ans`). Trying to
`put 1 rdy` or `copy gp0 rdy` is an error. The CPU is the only
writer.

## What to change

### 1. `src/registers.rs`

- Add `RegisterName::in_` (Rust keyword, so the trailing
  underscore — or use `input` if you'd rather not have it). Update
  the parser, encoder, decoder, and the name table.
- Add `RegisterName::rdy`.
- Add `pub rdy: u16` to `RegisterFile`. Initial value: `0`.
- In `RegisterFile::write`, reject writes to `in_` and `rdy` with
  an `AccessError`.
- In `RegisterFile::read`, reject reads of `in_` (it's a
  pseudo-register handled in `core.rs::copy`). `rdy` reads
  normally.

### 2. `src/core.rs`

In `copy`, add a branch for `src == RegisterName::in_`:

```rust
fn copy(
    &mut self,
    src: RegisterName,
    dst: RegisterName,
    devices: &mut [&mut dyn Device],
) -> Result<(), ExecutionError> {
    let val = match src {
        RegisterName::out => return Err(ExecutionError::CannotCpFrom(src)),
        RegisterName::in_ => {
            // Read from the currently selected device.
            let dvc = self.register_file.dvc as usize;
            let device = devices.get_mut(dvc)
                .ok_or(ExecutionError::NoSuchDevice(dvc as u16))?;
            match device.read().map_err(ExecutionError::Device)? {
                Some(v) => {
                    self.register_file.rdy = 1;
                    v
                }
                None => {
                    self.register_file.rdy = 0;
                    // Destination is left unchanged. Return early so
                    // we don't fall through to the dst write.
                    return Ok(());
                }
            }
        }
        _ => self.register_file.read(src)?,
    };
    // ... existing dst dispatch (out / pc-reject / ans-reject / reg) ...
}
```

Two subtleties:

- **On `None`, return early.** Don't fall through to the
  destination write — the spec says the destination is unchanged.
- **`rdy` is set on every `copy in`**, success or not. Programs
  that don't care about `rdy` simply don't read it.

### 3. `src/programs.rs`

- Recognize `in` as a register name (or `in_` / `input`, depending
  what you call it in Rust).
- Reject programs that use `in` as a destination or `rdy` /
  `out` / `pc` / `ans` as `put` destinations. Most of these are
  probably already rejected; double-check.

### 4. `src/devices.rs` — `LineInput` device

A simple line-input device that the TUI feeds into:

```rust
use std::collections::VecDeque;

#[derive(Default)]
pub struct LineInput {
    /// Bytes that have been submitted but not yet read by the CPU.
    pub queue: VecDeque<u8>,
}

impl LineInput {
    pub fn submit_line(&mut self, line: &str) {
        self.queue.extend(line.bytes());
        self.queue.push_back(b'\n');
    }
}

impl Device for LineInput {
    fn write(&mut self, _: u16) -> Result<(), Error> {
        // Optional: ring a bell, or just no-op. Programs shouldn't
        // write to a keyboard.
        Ok(())
    }

    fn read(&mut self) -> Result<Option<u16>, Error> {
        Ok(self.queue.pop_front().map(u16::from))
    }
}
```

Why `VecDeque<u8>` and not `String`? Because the CPU consumes one
byte per `read`, and we want O(1) front-pops. `String` doesn't
support that cleanly.

### 5. TUI integration

Add `LineInput` to `Devices` (stage 4) at index 3:

| dvc | Device     |
| --- | ---------- |
| 0   | LCD0       |
| 1   | LCD1       |
| 2   | TTY        |
| 3   | LineInput  |

In the TUI, add a small input pane at the bottom of the screen:

```
┌[Input]──────────────────────────────────────────────────┐
│ > hello world_                                          │
└─────────────────────────────────────────────────────────┘
```

A new field on `UiState`:

```rust
pub enum UiMode {
    Normal,
    InputPrompt { buffer: String },
    GotoPrompt  { input: String },   // from stage 5
}
```

Toggle into `InputPrompt` with a key (suggest `i`). In input mode,
keystrokes go into `buffer`; Enter calls
`computer.devices.line_input.submit_line(&buffer)` and exits the
mode; Esc cancels.

While in input mode, the normal CPU keybinds (`n` for step, `q`
for quit) shouldn't fire. Mode-aware key handling is the only
complication and it's easy.

### 6. Example program

`examples/12.echo.s`:

```
# Echo: read characters from the keyboard, write each one to LCD0
# until newline (ASCII 10).
#
# Setup: nothing to put — dvc starts at 0 (LCD0). We'll switch to
# the keyboard when we want to read.

.LOOP
  put 3 dvc        # select keyboard
  copy in gp0      # try to read a byte; sets rdy
  jump LOOP rdy    # if rdy == 0 (no input yet), spin

  put 10 gp1       # newline?
  sub gp0 gp1
  jump DONE ans    # if gp0 - 10 == 0, we're done

  put 0 dvc        # select LCD0
  copy gp0 out     # echo the byte

  put 0 gp2        # unconditional jump (cond=0)
  jump LOOP gp2

.DONE
  halt
```

A second example, `13.adder.s`, that reads two single-digit numbers
and prints the sum, would be a great smoke test for the whole
keyboard → CPU → LCD path.

## Test plan

### Unit tests in `core.rs`

- [ ] `copy in gp0` with a non-empty `LineInput` writes the byte
      and sets `rdy = 1`.
- [ ] `copy in gp0` with an empty `LineInput` leaves `gp0`
      unchanged and sets `rdy = 0`.
- [ ] `copy in gp0` with `dvc` pointing at a non-existent device
      returns `ExecutionError::NoSuchDevice`.
- [ ] `put 1 rdy` and `copy gp0 rdy` both error.
- [ ] `copy in pc` (or any other dst that's normally illegal) errors
      the same way it would for `copy gp0 pc`.

### Integration test

- [ ] Load `examples/12.echo.s`. Construct a `LineInput`,
      `submit_line("hi")`. Step until halt. Assert the LCD0 device's
      buffer contains `'h', 'i'`.

### Render test

- [ ] In `InputPrompt` mode, the input pane appears and shows the
      typed buffer.
- [ ] Pressing Enter consumes the buffer (it disappears from the
      pane).

## Done when

- [ ] `RegisterName::in_` and `RegisterName::rdy` exist; parser,
      decoder, and `RegisterFile` know about them.
- [ ] `core.rs::copy` reads from `devices[dvc]` when `src == in_`,
      sets `rdy` accordingly, and leaves the destination unchanged
      on `None`.
- [ ] `LineInput` device exists and is at `dvc = 3`.
- [ ] TUI has an input prompt mode wired into `LineInput`.
- [ ] `examples/12.echo.s` runs correctly.
- [ ] All the tests above pass.

## Notes for future me

- A live keyboard device (key events from crossterm directly,
  rather than line-buffered) is a natural follow-up. It could live
  at `dvc = 4` and share `LineInput`'s `VecDeque`-based shape. The
  hard part is *just* the TUI mode — the device itself is trivial.
  Worth doing for the screen-pixel demo (move a dot with arrow
  keys) — see stage 7.
- If you find yourself wanting more than a 1-bit ready flag (e.g. a
  full status register with bits for ready / error / EOF), the
  natural place is to widen `rdy` into a `status` register and
  document each bit. Don't preemptively widen.
- The `match dst` block in `copy` is getting long. After this stage
  is in, look at it — there might be a worthwhile extraction
  (`fn read_source(...)` and `fn write_dest(...)`) but only if it
  reads more clearly afterward.
