# Stage 2 — Headless `Computer` API

## Why this is the headline

The CPU logic is *already* layered correctly: `core.rs`,
`registers.rs`, `instructions.rs`, `programs.rs`, and `devices.rs`
don't import ratatui or crossterm. The existing tests in `core.rs`
prove it — they construct a `Core`, run instructions, and assert on
state without ever touching a terminal.

What's missing is a **clean seam to grab**. Today, testing the CPU
end-to-end means hand-rolling a `Vec<&mut dyn Device>`, looping
`execute_single_instruction` until power drops, and reaching into
`core.tty` (a `String` that's pretending to be a CPU field).
`core.rs::test_memory` (lines ~423-453) is eight lines of setup
before the assertions even start.

This stage introduces a top-level `Computer` struct that owns the
CPU, the devices, and the loaded program — and the test surface that
goes with it. After this stage:

```rust
let mut c = Computer::new();
c.load_source("put 5 gp0\nput 7 gp1\nadd gp0 gp1\ncopy ans out\nhalt\n")?;
c.run_to_halt()?;
assert_eq!(c.devices.lcd0.last_written(), Some(12));
```

That's the whole test. No `Frame`, no `TestBackend`, no `UiState`. It
runs in a single-threaded `cargo test` with no terminal attached.

This is the load-bearing stage for everything that comes after,
because **almost no future tests need to touch the TUI**. Render
tests (stage 3) cover only the things that are actually visual. CPU
behavior, device protocols, input handling, and disk persistence are
all unit tests against `Computer`.

## What to change

### 1. Move `tty` off `Core`, make it a real device

`Core::tty: String` and `Core::write_tty()` go away. A new `Tty`
device takes their place:

```rust
// src/devices.rs
#[derive(Default)]
pub struct Tty {
    pub buffer: String,
}

impl Device for Tty {
    fn write(&mut self, value: u16) -> Result<(), Error> {
        let s = String::from_utf16_lossy(&[value]);
        self.buffer.push_str(&s);
        Ok(())
    }

    fn read(&mut self) -> Result<Option<u16>, Error> {
        Ok(None)
    }
}
```

The dispatch in `core.rs::copy()` keeps its existing shape for now —
`match dvc { 0 => devices[0].write(...), 1 => devices[1].write(...),
_ => devices[2].write(...) }` — where index 2 is the tty. The
*cleanup* of that match (indexed lookup, `NoSuchDevice` error,
plug-in friendliness) is stage 4. Doing both in one stage would mush
together two different motivations: stage 2 is "make the seam", stage
4 is "make the seam pluggable".

This is a small but real semantic change: tty is now device #2, not
"the fallback". Document this in any example that prints to tty
(currently none do, per the exploration report — tty mostly carries
error console output).

### 2. The `Devices` struct

```rust
// src/computer.rs (new file) or in src/lib.rs
pub struct Devices {
    pub lcd0: Lcd,
    pub lcd1: Lcd,
    pub tty: Tty,
}

impl Devices {
    pub fn new() -> Self {
        Self {
            lcd0: Lcd::default(),
            lcd1: Lcd::default(),
            tty: Tty::default(),
        }
    }
}
```

Named fields, not a `Vec<Box<dyn Device>>`. The reason: tests and the
TUI both need *typed* access to specific devices (assert on
`lcd0.last_written()`, render the LCD font from `&Lcd`). Downcasting
through `dyn Device` would be a step backwards.

The CPU still wants slice access for indexed dispatch. A small helper
provides it:

```rust
impl Devices {
    pub fn as_slice(&mut self) -> [&mut dyn Device; 3] {
        // Order matters: index = dvc value.
        [&mut self.lcd0, &mut self.lcd1, &mut self.tty]
    }
}
```

(Stage 4 will add a unit test that pins this ordering down so future
device additions don't accidentally shuffle indices.)

### 3. The `Computer` struct

```rust
// src/computer.rs
pub struct Computer {
    pub core: Core,
    pub devices: Devices,
    pub program: Option<Program>,
}

impl Computer {
    pub fn new() -> Self {
        Self {
            core: Core::new(),
            devices: Devices::new(),
            program: None,
        }
    }

    pub fn load_source(&mut self, src: &str) -> Result<(), BootError> {
        let program = Program::try_compile(src)?;
        self.core.load_program(&program)?;
        self.program = Some(program);
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), ExecutionError> {
        let mut devices = self.devices.as_slice();
        self.core.execute_single_instruction(&mut devices)?;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.core.power
    }

    pub fn run_to_halt(&mut self) -> Result<(), ExecutionError> {
        // Safety net: a runaway program shouldn't hang the test
        // suite forever. 1M steps is generous for a 1 KiB CPU.
        const MAX_STEPS: usize = 1_000_000;
        for _ in 0..MAX_STEPS {
            if !self.is_running() { return Ok(()); }
            self.step()?;
        }
        Err(ExecutionError::StepLimitExceeded)
    }
}
```

Two things worth noticing:

- **`run_to_halt` has a step limit.** A program with an infinite
  loop and no `halt` would otherwise hang `cargo test`. 1M steps is
  small enough to fail fast (~seconds) and large enough that no
  legitimate test program will hit it. Add `StepLimitExceeded` to
  `ExecutionError`.
- **`program: Option<Program>` is on `Computer`, not `Core`.** This
  is what stages 3 (render) and 5 (RAM nav) need to colorize program
  bytes and resolve labels. Keep the symbol table on `Program`
  itself — see "open question" below.

### 4. Device history accessors

For tests to assert on what programs *did*, each device needs a way
to report its history. Add minimal accessors:

```rust
impl Lcd {
    pub fn last_written(&self) -> Option<u16> { self.last }
    pub fn history(&self) -> &[u16] { &self.history }
}

impl Tty {
    pub fn contents(&self) -> &str { &self.buffer }
}
```

`Lcd` probably already tracks `last` for rendering the 7-segment
display. Add a `history: Vec<u16>` field that gets pushed in
`write()`. It grows unbounded — fine for a 1 KiB-RAM CPU; tests run
short programs.

(Buffer device from `devices.rs` already does this — it's literally
just `Vec<u16>`. The `Lcd` accessors are the new code.)

### 5. Rewrite the existing `core.rs` tests to use `Computer`

This is half the value of the stage. Today's `test_loading` in
`core.rs` (lines 397-421) asserts on raw byte layout:

```rust
assert_eq!(core.memory[0], InstructionName::put as u8);
assert_eq!(core.memory[1], 7);
assert_eq!(core.memory[2], 0);
assert_eq!(core.memory[3], RegisterName::gp0 as u8);
```

Any change to instruction encoding breaks this. Replace with a
behavioral test against `Computer`:

```rust
#[test]
fn put_then_copy_to_lcd_outputs_value() {
    let mut c = Computer::new();
    c.load_source("put 7 gp0\ncopy gp0 out\nhalt\n").unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(7));
}
```

Same idea for `test_memory` (lines 423-453) — it currently runs four
instructions and asserts on byte locations. Rewrite as a `Computer`
test that asserts on register state after `run_to_halt`.

The stack tests (`test_stack_underflow`, `test_stack_overflow`,
`test_stack`) work directly on `Core`, which is fine — they're
testing the CPU primitive, not end-to-end behavior. Leave them
alone, but consider that they could be rewritten as tiny `Computer`
programs if you want everything to flow through the same surface.

### 6. New `tests/cpu.rs` integration test file

Move the behavioral tests out of `core.rs`'s unit tests and into a
proper integration test. This forces them to use only the public API
(which is the whole point — if a test needs private access to
`Core`, it's a unit test, not an integration test).

```rust
// tests/cpu.rs
use rpu::Computer;

#[test]
fn add_outputs_sum_to_lcd0() {
    let mut c = Computer::new();
    c.load_source(
        "put 5 gp0\n\
         put 7 gp1\n\
         add gp0 gp1\n\
         copy ans out\n\
         halt\n",
    ).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(12));
}

#[test]
fn fibonacci_example_runs_to_halt() {
    let src = std::fs::read_to_string("examples/09.fibonacci.s").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    // Whatever the expected last LCD values are.
}

#[test]
fn runaway_loop_hits_step_limit() {
    let mut c = Computer::new();
    c.load_source("put 0 gp0\njump gp0 zero\n").unwrap();
    let err = c.run_to_halt().unwrap_err();
    assert!(matches!(err, ExecutionError::StepLimitExceeded));
}
```

These are the tests you actually want a hundred of. Aim to have one
integration test per example file in `examples/` by the end of this
stage — they're cheap to write and they catch regressions in any
layer (assembler, encoder, decoder, executor, devices) for free.

### 7. Export the API from `lib.rs`

```rust
// src/lib.rs
mod computer;

pub use computer::{Computer, Devices};
pub use core::Core;
pub use devices::{Device, Lcd, Tty, Buffer};
pub use programs::Program;
// ... etc
```

This is what makes `tests/cpu.rs` work — integration tests only see
`pub` items.

## Open questions

- **Should `Lcd::history` be capped?** I'd say no for now (tests run
  short programs), but if a TUI demo runs for hours and you start
  noticing memory growth, cap it at the last N values.
- **Should `Program` retain its symbol table?** This stage doesn't
  require it, but stages 5 (jump-to-label in the RAM navigator) and
  3 (label-aware decoded sidebar) both want it. Cheap to add now —
  one `pub labels: HashMap<String, u16>` field on `Program`. Doing
  it in this stage means it's available when those stages need it.
- **Where does `Computer` live?** New file `src/computer.rs` is
  cleanest. Adding it to `lib.rs` directly works too but `lib.rs`
  starts to feel like a junk drawer.

## Test plan

- [ ] All existing tests still pass (after rewriting the byte-layout
      ones).
- [ ] `tests/cpu.rs` exists with at least 5 integration tests:
      - simple add-and-output
      - one example file from `examples/`
      - stack push/pop round trip
      - `BootError` on a malformed program
      - `StepLimitExceeded` on an infinite loop
- [ ] `Computer::new()` + `load_source` + `run_to_halt` works without
      any TUI/terminal/io setup at all (i.e. `cargo test` with no
      tty attached works fine — try `cargo test < /dev/null`).
- [ ] `Tty` is no longer accessible as `core.tty`; it's
      `computer.devices.tty.contents()`.

## Done when

- [ ] `Computer`, `Devices`, `Tty` all exist and are exported from
      `lib.rs`.
- [ ] `Core` no longer has `tty: String` or `write_tty()`.
- [ ] `Computer::run_to_halt()` exists and has a step limit.
- [ ] `ExecutionError::StepLimitExceeded` exists.
- [ ] `Lcd` has `last_written()` and `history()` accessors. `Tty`
      has `contents()`.
- [ ] `tests/cpu.rs` exists with the integration tests above.
- [ ] Brittle byte-layout assertions in `core.rs` tests are replaced
      with behavioral assertions through `Computer`.
- [ ] At least one existing example file (`02.add_5_7.s` or
      `09.fibonacci.s`) has an integration test that runs it to
      halt and asserts on output.

## Notes for future me

- The whole point of this stage is the seam. **Resist the urge to
  also clean up the dispatch match** in `core.rs::copy` — that's
  stage 4 and conflating them muddies both diffs.
- `as_slice()` is a `[&mut dyn Device; 3]` array, not a slice
  literal. The fixed length is intentional for now; once stage 4
  lands and the dispatch becomes index-based, the array length
  grows by one per device added. A `Vec` would also work but the
  array gives you a compile-time length check on the device count.
- `tests/cpu.rs` is where you'll spend most of your future testing
  effort. Treat it as a first-class file: keep it organized, give
  tests meaningful names, group them by what they exercise. It will
  grow.
- **Most important habit shift:** going forward, when you add a
  feature, write the headless test in `tests/cpu.rs` *first*. The
  TUI render test (stage 3 onward) is a bonus, not the primary
  proof. The CPU is the thing that has to be correct.
