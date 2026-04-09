# Stage 7 — Pixel screen device

## Why this is here

The goal: a box in the TUI where the running program can plot
"pixels" (rendered as ASCII block characters), driven by writes to
a new device. Visually it should look like a tiny low-res display.

This stage depends on stages 4 (device table) and ideally 6 (so you
can wire keyboard input into a "draw with arrows" demo). It does
**not** depend on the RAM navigation stage.

## Design decisions

### Resolution and rendering

Use **half-block characters** (`▀`, `▄`, `█`, ` `) so two pixel rows
fit in one terminal row. This gets you square-ish pixels in most
terminals.

Suggested default: **32 wide × 16 tall** = 512 pixels. That fits
comfortably in the existing middle column and is enough for fun
demos (Pong-ish, snake, bouncing balls). Each terminal row shows
two pixel rows, so the screen takes 8 terminal rows + 2 for the
border = 10 lines total.

Storage: `Vec<bool>` of length `WIDTH * HEIGHT`, indexed
row-major. 1 bit per pixel; we don't need color or grayscale for
the first version.

```rust
pub const SCREEN_W: usize = 32;
pub const SCREEN_H: usize = 16;

pub struct Screen {
    pub framebuffer: Vec<bool>,   // SCREEN_W * SCREEN_H
    cursor_x: u16,
    cursor_y: u16,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            framebuffer: vec![false; SCREEN_W * SCREEN_H],
            cursor_x: 0,
            cursor_y: 0,
        }
    }
}
```

### The protocol

The device receives `u16` values via `Device::write`. Since the
program needs to do more than one thing per pixel (set X, set Y,
plot, clear), we need a small command protocol. Two reasonable
options:

**Option A — stateful cursor (recommended).** High byte is the
opcode, low byte is the argument:

| Opcode | Name      | Arg     | Effect                                    |
| ------ | --------- | ------- | ----------------------------------------- |
| `0x00` | `SET_X`   | x       | `cursor_x = arg`                          |
| `0x01` | `SET_Y`   | y       | `cursor_y = arg`                          |
| `0x02` | `PLOT`    | (any)   | set pixel at `(cursor_x, cursor_y)` to 1  |
| `0x03` | `UNPLOT`  | (any)   | set pixel at `(cursor_x, cursor_y)` to 0  |
| `0x04` | `CLEAR`   | (any)   | clear the entire framebuffer              |
| `0x05` | `FLIP`    | (any)   | toggle the pixel at the cursor            |

Source-side use:

```
# Plot a pixel at (5, 3)
put 0x0005 gp0     # SET_X 5
copy gp0 out
put 0x0103 gp0     # SET_Y 3
copy gp0 out
put 0x0200 gp0     # PLOT
copy gp0 out
```

That's verbose, which is the point — the user said they want to be
explicit about every instruction. Each pixel-set is six instructions.

**Option B — packed (rejected).** A single `u16` carries
`(x: 6 bits, y: 5 bits, on: 1 bit)`. Faster but obscures what's
happening, and the bit-packing is tricky for students.

**Recommendation: Option A.**

### Bounds checking

If a `PLOT` happens with `cursor_x >= SCREEN_W` or `cursor_y >=
SCREEN_H`, the device should silently ignore it (don't crash, don't
wrap). Document this. Wrapping leads to confusing aliasing bugs in
student programs.

### Reading from the screen

`Device::read` returns `None`. The screen is write-only (for now).
A future enhancement could let programs query whether a pixel is
set (useful for collision detection in games), but that's a bigger
protocol expansion. Skip until needed.

## What to change

### 1. New file `src/devices/screen.rs`

Or a new section in `src/devices.rs` if you'd rather keep one file.

```rust
use super::{Device, Error};

pub const SCREEN_W: usize = 32;
pub const SCREEN_H: usize = 16;

#[repr(u8)]
enum Op {
    SetX   = 0x00,
    SetY   = 0x01,
    Plot   = 0x02,
    Unplot = 0x03,
    Clear  = 0x04,
    Flip   = 0x05,
}

pub struct Screen {
    pub framebuffer: Vec<bool>,
    cursor_x: u16,
    cursor_y: u16,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            framebuffer: vec![false; SCREEN_W * SCREEN_H],
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    pub fn pixel(&self, x: usize, y: usize) -> bool {
        self.framebuffer[y * SCREEN_W + x]
    }

    fn set_pixel(&mut self, value: bool) {
        let (x, y) = (self.cursor_x as usize, self.cursor_y as usize);
        if x < SCREEN_W && y < SCREEN_H {
            self.framebuffer[y * SCREEN_W + x] = value;
        }
    }
}

impl Device for Screen {
    fn write(&mut self, value: u16) -> Result<(), Error> {
        let op  = (value >> 8) as u8;
        let arg = (value & 0xff) as u16;
        match op {
            0x00 => self.cursor_x = arg,
            0x01 => self.cursor_y = arg,
            0x02 => self.set_pixel(true),
            0x03 => self.set_pixel(false),
            0x04 => { for px in &mut self.framebuffer { *px = false; } }
            0x05 => {
                let (x, y) = (self.cursor_x as usize, self.cursor_y as usize);
                if x < SCREEN_W && y < SCREEN_H {
                    let idx = y * SCREEN_W + x;
                    self.framebuffer[idx] = !self.framebuffer[idx];
                }
            }
            _ => return Err(Error::Write(format!("screen: unknown op {op:#x}"))),
        }
        Ok(())
    }

    fn read(&mut self) -> Result<Option<u16>, Error> {
        Ok(None)
    }
}
```

### 2. Add to the device table

`Devices` (from stage 4) gets:

```rust
pub struct Devices {
    pub lcd0: Lcd,
    pub lcd1: Lcd,
    pub tty: Tty,
    pub line_input: LineInput,
    pub screen: Screen,
}

impl Devices {
    pub fn as_slice(&mut self) -> [&mut dyn Device; 5] {
        [
            &mut self.lcd0,        // 0
            &mut self.lcd1,        // 1
            &mut self.tty,         // 2
            &mut self.line_input,  // 3
            &mut self.screen,      // 4
        ]
    }
}
```

### 3. TUI render function

```rust
fn render_screen(frame: &mut Frame, screen: &Screen, area: Rect) {
    use ratatui::text::{Line, Span};

    // Each terminal row shows two pixel rows; pair them up.
    let mut lines: Vec<Line> = Vec::with_capacity(SCREEN_H / 2);
    for ty in 0..(SCREEN_H / 2) {
        let mut row = String::with_capacity(SCREEN_W);
        for x in 0..SCREEN_W {
            let top    = screen.pixel(x, ty * 2);
            let bottom = screen.pixel(x, ty * 2 + 1);
            row.push(match (top, bottom) {
                (false, false) => ' ',
                (true,  false) => '▀',
                (false, true ) => '▄',
                (true,  true ) => '█',
            });
        }
        lines.push(Line::from(Span::raw(row)));
    }

    let paragraph = Paragraph::new(lines)
        .block(common_block("Screen"));
    frame.render_widget(paragraph, area);
}
```

Slot it into the layout — probably middle column, between the LCDs
and the printer pane. The screen is 8 terminal rows + 2 border = 10
lines; trim something else if needed.

### 4. Examples

- `examples/14.bouncing_dot.rpu` — plot a single pixel and bounce it
  off the walls. Doesn't need input. Great smoke test.
- `examples/15.draw_with_arrows.rpu` — read keys from `LineInput`
  (or a future live keyboard device), interpret them as
  up/down/left/right, plot the cursor's new position. Requires
  stage 6.

## Test plan

Render tests are very satisfying for this stage because the screen
output is *visible in the buffer*.

### Unit tests on `Screen`

- [ ] Fresh screen has all-`false` framebuffer.
- [ ] `SET_X 5; SET_Y 3; PLOT` sets exactly pixel `(5, 3)`.
- [ ] `PLOT` with cursor out of bounds is a no-op (no crash, no
      wraparound).
- [ ] `CLEAR` zeros the framebuffer.
- [ ] `FLIP` toggles.
- [ ] Unknown opcode returns `Error::Write`.

### Integration test

- [ ] Compile and run a tiny inline program that plots a 2×2
      square. Inspect `computer.devices.screen.framebuffer` and
      assert exactly four pixels are set at the right indices.

### Render test (`TestBackend`)

- [ ] Load the same 2×2 square program, render, and pull out the
      screen pane region of the buffer. Assert it contains a `█`
      at the expected terminal cell. (Use `Buffer::with_lines` for
      a really clean expected layout.)

```rust
let expected = Buffer::with_lines(vec![
    "┌[Screen]──────────────────────────┐",
    "│ ██                               │",
    "│ ██                               │",
    "│                                  │",
    // ... (rest of the rows blank)
]);
```

## Done when

- [ ] `Screen` device exists and implements `Device`.
- [ ] Lives at `dvc = 4` in the device table.
- [ ] TUI shows the screen pane and updates it as the program runs.
- [ ] `examples/14.bouncing_dot.rpu` runs and visibly bounces a dot.
- [ ] All unit/render tests above pass.

## Notes for future me

- 32×16 is small but responsive. Don't bump it without a reason —
  bigger screens eat the printer/LCD layout space.
- If you want color later: change `framebuffer` from `Vec<bool>` to
  `Vec<u8>` with a small palette, and add a `SET_COLOR` opcode. The
  half-block render becomes a `Span::styled` with `fg`/`bg` colors.
  Don't do this preemptively.
- A "live keyboard" device (`dvc = 5`?) is the natural pairing for
  this stage. It's basically a `LineInput` but pushes one byte per
  keypress instead of waiting for Enter. Two arms in the TUI key
  handler. Worth doing once you have a screen to drive with it.
- The cursor protocol's verbosity (six instructions per pixel) is
  intentional given the no-sugar decision, but it does make
  programs long. If a program needs to plot a lot of pixels, the
  natural pattern is a subroutine — which we don't have a `call`
  instruction for. Just use `jump` with a return address in a
  register, the way the existing examples do.
