//! Integration tests for the headless `Computer` API.
//!
//! Everything in this file goes through `rpu::Computer` and asserts
//! on observable state — register values, device history, error
//! variants. No `TestBackend`, no `Frame`, no terminal. These tests
//! run cleanly without a tty (`cargo test < /dev/null`).

use rpu::{Computer, ExecutionError};

/// Smallest realistic end-to-end test: load a tiny program, run it
/// to halt, assert on what the LCD saw.
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
    assert_eq!(c.devices.lcd0.history(), vec![12]);
}

/// Sequence test: countdown should print 5, 4, 3, 2, 1, 0 in order.
/// (The program's comment says "5,4,3,2,1 blastoff!" but in fact it
/// writes the result of `1 - 1 = 0` to the LCD before checking the
/// exit condition. The 0 is real and the test reflects what the
/// program actually does, not what the comment hopes for.)
///
/// This is the test that motivates having a history field at all —
/// `last_written()` alone could only see the final 0, with no way
/// to verify the descending sequence that came before it.
#[test]
fn countdown_example_prints_5_through_0() {
    let src = std::fs::read_to_string("examples/07.countdown.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();

    assert_eq!(c.devices.lcd0.history(), vec![5, 4, 3, 2, 1, 0]);
}

/// A second example: fibonacci. The program initializes with
/// `gp0=1, gp1=0` and on each iteration writes the *running
/// previous* to LCD0 and the running current to LCD1. So LCD0
/// starts with `[1, 0, 1, 1, 2, 3, 5, ...]` (not the standard
/// fibonacci sequence; the leading `1, 0` is the init bleeding
/// through), and LCD1 follows the more conventional `[0, 1, 1, 2,
/// 3, 5, ...]`.
///
/// The program runs forever until the running sum overflows u16,
/// which surfaces as `ExecutionError::Overflow`.
#[test]
fn fibonacci_example_runs_to_overflow() {
    let src = std::fs::read_to_string("examples/09.fibonacci.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();

    let err = c.run_to_halt().unwrap_err();
    assert!(
        matches!(err, ExecutionError::Overflow(_, _)),
        "expected Overflow, got {err:?}",
    );

    let lcd0 = c.devices.lcd0.history();
    let lcd1 = c.devices.lcd1.history();
    // Sanity check: both LCDs saw plenty of values before overflow.
    assert!(lcd0.len() >= 10, "lcd0 only saw {} values", lcd0.len());
    assert!(lcd1.len() >= 10, "lcd1 only saw {} values", lcd1.len());
    // The early sequence locks in what the program is actually
    // doing — if anyone "fixes" the init values later, this test
    // catches the behavioral change.
    assert_eq!(&lcd0[..5], &[1, 0, 1, 1, 2]);
    assert_eq!(&lcd1[..5], &[0, 1, 1, 2, 3]);
}

/// Stack push/pop round trip exercised end-to-end through the
/// assembler and executor (rather than calling `Core::push`/`pop`
/// directly the way the unit tests in core.rs do).
#[test]
fn push_then_pop_round_trips_through_program() {
    let mut c = Computer::new();
    c.load_source(
        "put 42 gp0\n\
         push gp0\n\
         put 0 gp0\n\
         pop gp1\n\
         copy gp1 out\n\
         halt\n",
    ).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(42));
}

/// Compilation errors surface as `BootError`, not panics.
#[test]
fn malformed_program_returns_boot_error() {
    let mut c = Computer::new();
    let result = c.load_source("this is not valid assembly\n");
    assert!(result.is_err(), "expected BootError, got {result:?}");
}

/// Programs with no halt and no terminating error must not hang
/// the test suite. The step limit catches them.
#[test]
fn runaway_loop_hits_step_limit() {
    let mut c = Computer::new();
    c.load_source(
        "put 0 gp0\n\
         jump gp0 zero\n",
    ).unwrap();
    let err = c.run_to_halt().unwrap_err();
    assert!(
        matches!(err, ExecutionError::StepLimitExceeded),
        "expected StepLimitExceeded, got {err:?}",
    );
}

/// Writing to dvc 2 (the tty fallback) should append the bytes as
/// UTF-16 code units to the tty buffer. After stage 4 lands, dvc 2
/// will be the *explicit* tty index instead of "fallback for any
/// unknown dvc"; until then this also exercises the fallback.
#[test]
fn writing_to_tty_appends_chars() {
    let mut c = Computer::new();
    c.load_source(
        "put 2 dvc\n\
         put 72 gp0\n\
         copy gp0 out\n\
         put 105 gp0\n\
         copy gp0 out\n\
         halt\n",
    ).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.tty.contents(), "Hi");
}

/// `Computer::new()` plus `load_source` plus `run_to_halt` works
/// without any tty / terminal / io setup at all. This test exists
/// mostly as documentation: if it ever stops compiling because some
/// new dependency dragged in a TUI requirement, we have a problem.
#[test]
fn computer_runs_without_a_terminal() {
    let mut c = Computer::new();
    c.load_source("halt\n").unwrap();
    c.run_to_halt().unwrap();
    assert!(!c.is_running());
}
