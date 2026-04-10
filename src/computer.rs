//! Headless top-level type that owns the CPU, devices, and loaded
//! program. This is the API that tests and the TUI both build on:
//! tests construct a `Computer`, load a source string, run it to
//! halt, and inspect device history; the TUI does the same but
//! drives stepping from key events instead of `run_to_halt`.

use crate::core::{Core, BootError, ExecutionError, RAM};
use crate::devices::{Device, Lcd, Tty};
use crate::programs::Program;

/// Maximum number of instructions `Computer::run_to_halt` will
/// execute before giving up. Generous enough for any realistic
/// teaching program; small enough that an infinite loop fails fast
/// in `cargo test`.
pub const STEP_LIMIT: usize = 1_000_000;

/// The full set of devices the CPU can talk to. Named fields so
/// tests and the TUI can grab specific devices with their concrete
/// types (no `dyn Device` downcasting). The CPU sees them through
/// the `as_slice()` view, indexed by `dvc`.
pub struct Devices {
    pub lcd0: Lcd,
    pub lcd1: Lcd,
    pub tty: Tty,
}

impl Devices {
    pub fn new() -> Self {
        Self {
            lcd0: Lcd::new(),
            lcd1: Lcd::new(),
            tty: Tty::new(),
        }
    }

    /// View of the device table indexed by `dvc` value. The order is
    /// load-bearing — it defines what each `dvc` value means at
    /// runtime — and is locked in by `device_table_ordering_is_stable`
    /// in this module's tests. When you add a device here, also add
    /// a matching `DVC_*` constant in `devices.rs` and an assertion
    /// to that test.
    pub fn as_slice(&mut self) -> [&mut dyn Device; 3] {
        // [0]=lcd0, [1]=lcd1, [2]=tty
        [&mut self.lcd0, &mut self.lcd1, &mut self.tty]
    }
}

impl Default for Devices {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Computer {
    pub core: Core,
    pub devices: Devices,
    pub program: Option<Program>,

    /// Snapshot of `core.memory` taken at the *start* of the most
    /// recent `step()`. Compared against the live memory at render
    /// time so the TUI can flash bytes that just changed. Initialized
    /// to match `core.memory` at boot, and re-synced inside
    /// `load_source` so freshly loaded program bytes don't all show
    /// up as "just changed" on the first frame.
    last_step_memory: [u8; RAM],
}

impl Computer {
    pub fn new() -> Self {
        Self {
            core: Core::new(),
            devices: Devices::new(),
            program: None,
            // Both buffers start as all zeros, so byte_changed returns
            // false for every address until something actually changes.
            last_step_memory: [0; RAM],
        }
    }

    /// Compile a source string, load it into RAM, and remember the
    /// `Program` so the TUI can render its source lines and resolve
    /// labels. After loading, the change-detection snapshot is
    /// re-synced to the freshly loaded memory — otherwise the very
    /// first frame would highlight every program byte as "just
    /// changed", which is the wrong story.
    pub fn load_source(&mut self, src: &str) -> Result<(), BootError> {
        self.load_source_named("<source>", src)
    }

    pub fn load_source_named(&mut self, name: &str, src: &str)
        -> Result<(), BootError>
    {
        let program = Program::try_compile_named(name, src)?;
        self.core.load_program(&program)?;
        self.program = Some(program);
        self.last_step_memory = self.core.memory;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.core.power
    }

    /// Execute one instruction. Mirrors `Core::execute_single_instruction`
    /// but routes the device slice from `Devices::as_slice()` so
    /// callers don't have to assemble it themselves.
    ///
    /// As a side effect, snapshots `core.memory` *before* executing
    /// so that `byte_changed(addr)` can report which bytes this step
    /// touched. The snapshot is taken even if the instruction errors;
    /// that's fine because no further steps will run after an error
    /// anyway (and the snapshot only matters for what the next render
    /// shows).
    pub fn step(&mut self) -> Result<(), ExecutionError> {
        self.last_step_memory = self.core.memory;
        let mut slice = self.devices.as_slice();
        self.core.execute_single_instruction(&mut slice)?;
        Ok(())
    }

    /// Returns true if the byte at `addr` differs from its value at
    /// the start of the most recent step. Used by the memory pane
    /// renderer to flash recently-touched bytes. Always returns
    /// false before any step has run, and always returns false for
    /// addresses past the end of RAM (treated as "not changed"
    /// rather than panicking).
    pub fn byte_changed(&self, addr: u16) -> bool {
        let i = addr as usize;
        if i >= RAM { return false; }
        self.core.memory[i] != self.last_step_memory[i]
    }

    /// Step until the CPU halts. Bails out with
    /// `ExecutionError::StepLimitExceeded` after `STEP_LIMIT` steps
    /// so a runaway program can't hang the test suite.
    pub fn run_to_halt(&mut self) -> Result<(), ExecutionError> {
        for _ in 0..STEP_LIMIT {
            if !self.is_running() {
                return Ok(());
            }
            self.step()?;
        }
        Err(ExecutionError::StepLimitExceeded)
    }
}

impl Default for Computer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::{DVC_LCD0, DVC_LCD1, DVC_TTY};

    /// Locks in the order of devices in `Devices::as_slice()`. The
    /// order is what gives each `dvc` literal its meaning at runtime,
    /// so a future "I'll just reorder the array" mistake would
    /// silently rebind every program's device IDs. This test makes
    /// that mistake loud.
    ///
    /// Strategy: write a unique sentinel through each slice index,
    /// then read it back through the *typed* field on `Devices`. If
    /// `[0]` lands in `lcd0`, `[1]` in `lcd1`, and `[2]` in `tty`,
    /// the order is correct. When a new device is added, this test
    /// gets one more sentinel + one more typed assertion.
    #[test]
    fn device_table_ordering_is_stable() {
        let mut devices = Devices::new();

        assert_eq!(devices.as_slice().len(), 3);

        // Each write borrows a fresh slice — `as_slice` returns
        // mutable references, so we can't hold the whole slice and
        // also touch typed fields afterward.
        devices.as_slice()[DVC_LCD0 as usize].write(101).unwrap();
        devices.as_slice()[DVC_LCD1 as usize].write(102).unwrap();
        devices.as_slice()[DVC_TTY  as usize].write(b'g' as u16).unwrap();

        assert_eq!(devices.lcd0.last_written(), Some(101));
        assert_eq!(devices.lcd1.last_written(), Some(102));
        assert_eq!(devices.tty.contents(), "g");
    }
}
