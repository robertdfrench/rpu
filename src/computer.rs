//! Headless top-level type that owns the CPU, devices, and loaded
//! program. This is the API that tests and the TUI both build on:
//! tests construct a `Computer`, load a source string, run it to
//! halt, and inspect device history; the TUI does the same but
//! drives stepping from key events instead of `run_to_halt`.

use crate::core::{Core, BootError, ExecutionError};
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
///
/// Stage 4 will replace the hardcoded `match dvc` in `core.rs::copy`
/// with proper indexed lookup; this struct is already shaped for
/// that.
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

    /// View of the device table indexed by `dvc` value:
    /// `[0]=lcd0, [1]=lcd1, [2]=tty`. Stage 4 will start treating
    /// this as the source of truth for dispatch.
    pub fn as_slice(&mut self) -> [&mut dyn Device; 3] {
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
}

impl Computer {
    pub fn new() -> Self {
        Self {
            core: Core::new(),
            devices: Devices::new(),
            program: None,
        }
    }

    /// Compile a source string, load it into RAM, and remember the
    /// `Program` so the TUI can render its source lines and resolve
    /// labels.
    pub fn load_source(&mut self, src: &str) -> Result<(), BootError> {
        let program = Program::try_compile(src)?;
        self.core.load_program(&program)?;
        self.program = Some(program);
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.core.power
    }

    /// Execute one instruction. Mirrors `Core::execute_single_instruction`
    /// but routes the device slice from `Devices::as_slice()` so
    /// callers don't have to assemble it themselves.
    pub fn step(&mut self) -> Result<(), ExecutionError> {
        let mut slice = self.devices.as_slice();
        self.core.execute_single_instruction(&mut slice)?;
        Ok(())
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
