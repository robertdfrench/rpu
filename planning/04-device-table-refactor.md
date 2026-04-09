# Stage 4 — Device table refactor

## Why this is here (and why it's now smaller)

Stage 2 already gave us `Computer`, `Devices`, and `Tty` as a real
`Device`. What stage 2 *didn't* do is clean up the dispatch in
`core.rs::copy()`:

```rust
RegisterName::out => {
    match self.register_file.dvc {
        0 => { devices[0].write(val).unwrap(); },
        1 => { devices[1].write(val).unwrap(); },
        _ => { devices[2].write(val).unwrap(); },   // tty fallback
    }
    Ok(())
}
```

Three specific problems:

1. **Adding a new device means editing this match.** Stages 6, 7,
   and 8 each add a new device. We don't want to keep growing this.
2. **`.unwrap()` on device writes** is hiding errors that should be
   surfaced as `ExecutionError`.
3. **The `_ => devices[2]` fallback** silently routes "any unknown
   dvc" to tty. After this stage, an unknown dvc is a clean
   execution error.

This stage is small but it unblocks every later device stage.
Doing it now (before adding inputs and screens) means stages 6/7/8
each become "push another device into `Devices`" instead of
"refactor `copy` again".

## What to change

### 1. Indexed dispatch in `core.rs::copy`

```rust
fn copy(
    &mut self,
    src: RegisterName,
    dst: RegisterName,
    devices: &mut [&mut dyn Device],
) -> Result<(), ExecutionError> {
    let val = match src {
        RegisterName::out => return Err(ExecutionError::CannotCpFrom(dst)),
        _ => self.register_file.read(src)?,
    };

    match dst {
        RegisterName::pc  => Err(ExecutionError::CannotCpTo(dst)),
        RegisterName::ans => Err(ExecutionError::CannotCpTo(dst)),
        RegisterName::out => {
            let dvc = self.register_file.dvc as usize;
            let device = devices
                .get_mut(dvc)
                .ok_or(ExecutionError::NoSuchDevice(dvc as u16))?;
            device.write(val).map_err(ExecutionError::Device)?;
            Ok(())
        }
        _ => {
            self.register_file.write(dst, val)?;
            Ok(())
        }
    }
}
```

The hardcoded `match dvc { 0 => ..., 1 => ..., _ => ... }` is gone.
Adding a device is now a one-line change in `Devices::as_slice()`.

### 2. New `ExecutionError` variants

```rust
pub enum ExecutionError {
    // ... existing variants ...
    NoSuchDevice(u16),
    Device(crate::devices::Error),
}
```

These replace the `.unwrap()` calls. A program that selects a
nonexistent `dvc` now gets a clean execution error visible in the
tty / error console pane, instead of either a panic or a silent
fallback to tty.

This is a **behavior change**: programs that previously did
`put 99 dvc; copy gp0 out` and silently saw their output go to the
tty pane now get `ExecutionError::NoSuchDevice(99)`. If any existing
example relies on the old behavior, fix it or document it. (None
should — the tty fallback was undocumented and surprising.)

### 3. Pin the device ordering down with a unit test

The order in `Devices::as_slice()` is load-bearing — it defines what
each `dvc` value means. A test that locks it in:

```rust
// in src/computer.rs or src/devices.rs tests
#[test]
fn device_table_ordering_is_stable() {
    let mut devices = Devices::new();
    let slice = devices.as_slice();
    assert_eq!(slice.len(), 3);
    // We can't easily downcast through dyn Device, so instead
    // verify by writing a sentinel and reading back via the typed
    // accessor.
    drop(slice);
    devices.as_slice()[0].write(101).unwrap();
    devices.as_slice()[1].write(102).unwrap();
    devices.as_slice()[2].write(103).unwrap();
    assert_eq!(devices.lcd0.last_written(), Some(101));
    assert_eq!(devices.lcd1.last_written(), Some(102));
    assert_eq!(devices.tty.contents(), "g");   // u16(103) → 'g'
}
```

(Adjust the assertion details to match your `Tty::write`
implementation. The point is: a future "I'll just reorder the
array" mistake breaks this test loudly.)

When stage 6/7/8 add devices, this test gets one more line each.

### 4. Constants for device IDs

Add named constants in `src/devices.rs` so Rust code never has to
write magic numbers:

```rust
pub const DVC_LCD0: u16 = 0;
pub const DVC_LCD1: u16 = 1;
pub const DVC_TTY:  u16 = 2;
// Stages 6/7/8 will add: DVC_KEYBOARD, DVC_SCREEN, DVC_DISK
```

Source assembly programs still write the literal numbers (per the
no-sugar decision in `README.md` non-goals). The constants are for
Rust code: tests, the TUI, and the device table itself.

The current device table:

| dvc | Constant     | Device |
| --- | ------------ | ------ |
| 0   | `DVC_LCD0`   | LCD0   |
| 1   | `DVC_LCD1`   | LCD1   |
| 2   | `DVC_TTY`    | TTY    |

After this stage you can `assert_eq!(c.core.register_file.dvc,
DVC_TTY)` instead of `... 2`. Stages 6/7/8 grow the table.

## Test plan

### Headless (`tests/cpu.rs`)

The existing `writing_to_tty_appends_chars` test (added in stage 2)
uses an explicit `put 2 dvc` so it exercises the *intended* tty
index, not the fallback. After this stage it should continue to
pass with no changes — the dispatch shape changed but the index
didn't.

New test for the behavior change:

```rust
#[test]
fn writing_to_unknown_dvc_errors() {
    let mut c = Computer::new();
    c.load_source("put 99 dvc\nput 5 gp0\ncopy gp0 out\nhalt\n").unwrap();
    let err = c.run_to_halt().unwrap_err();
    assert!(matches!(err, ExecutionError::NoSuchDevice(99)));
}
```

Plus the `device_table_ordering_is_stable` unit test from above
(probably belongs in `src/computer.rs`'s `tests` module since
that's where `Devices` lives).

### Render (`tests/render.rs`)

- [ ] The smoke test from stage 3 still passes — the dispatch
      change shouldn't affect what gets rendered.
- [ ] Add a render test: load a tty-printing program, run to halt,
      assert the printer pane in the rendered buffer contains the
      expected text.

## Done when

- [ ] `core.rs::copy` does indexed device dispatch with no
      hardcoded `match dvc { 0 => ..., 1 => ... }`.
- [ ] `ExecutionError::NoSuchDevice` and `ExecutionError::Device`
      exist and replace the `.unwrap()` calls.
- [ ] `DVC_LCD0`, `DVC_LCD1`, `DVC_TTY` constants exist.
- [ ] `device_table_ordering_is_stable` test exists and passes.
- [ ] Headless tests for unknown-dvc and tty-write behaviors exist.
- [ ] Adding a new device requires only: add a field to `Devices`,
      add it to `as_slice()`, add a constant, add a line to the
      ordering test. **No edits to `core.rs::copy`.**

## Notes for future me

- A comment above the array literal in `Devices::as_slice()` listing
  the indices (`// [0]=lcd0, [1]=lcd1, [2]=tty`) helps a lot. The
  ordering test catches mistakes; the comment prevents them.
- Don't take the bait of making `Devices` itself implement
  `Device`-ish dispatch (`fn write(&mut self, dvc: u16, val: u16)`).
  The `&mut [&mut dyn Device]` shape is what `core.rs` wants and it
  composes with future devices for free.
- The behavior change (unknown dvc → error instead of tty) is the
  kind of thing that should be called out in commit messages and
  any user-facing changelog. It's the right call but it's a break.
- After this stage, the `match dst` block in `copy` is the right
  shape forever — every future change is in `Devices`, not `Core`.
  Take a moment to appreciate it.
