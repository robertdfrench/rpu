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
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
    let err = c.run_to_halt().unwrap_err();
    assert!(
        matches!(err, ExecutionError::StepLimitExceeded),
        "expected StepLimitExceeded, got {err:?}",
    );
}

/// Writing to dvc 2 (the tty) should append the bytes as UTF-16
/// code units to the tty buffer. dvc 2 is the only way to reach the
/// tty — there is no longer a silent fallback for unknown dvc
/// values; see `writing_to_unknown_dvc_errors` below.
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
    )
    .unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.tty.contents(), "Hi");
}

/// Selecting a `dvc` value that doesn't map to any device in the
/// table is a clean execution error, not a panic and not a silent
/// fallback. This is the behavior change introduced by stage 4.
#[test]
fn writing_to_unknown_dvc_errors() {
    let mut c = Computer::new();
    c.load_source(
        "put 99 dvc\n\
         put 5 gp0\n\
         copy gp0 out\n\
         halt\n",
    )
    .unwrap();
    let err = c.run_to_halt().unwrap_err();
    assert!(
        matches!(err, ExecutionError::NoSuchDevice(99)),
        "expected NoSuchDevice(99), got {err:?}",
    );
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

// ============================================================
// Exercise 12: read_memory — write 42 to addr 200, read back
// ============================================================
#[test]
fn exercise_12_read_memory() {
    let src = std::fs::read_to_string("examples/12.read_memory.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(42));
}

// ============================================================
// Exercise 13: multiply — 6*7=42 on LCD0, 12*12=144 on LCD1
// ============================================================
#[test]
fn exercise_13_multiply() {
    let src = std::fs::read_to_string("examples/13.multiply.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(42));
    assert_eq!(c.devices.lcd1.last_written(), Some(144));
}

// ============================================================
// Exercise 14: copy_chain — 42 survives through 4 copies
// ============================================================
#[test]
fn exercise_14_copy_chain() {
    let src = std::fs::read_to_string("examples/14.copy_chain.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(42));
    assert_eq!(c.devices.lcd1.last_written(), Some(42));
}

// ============================================================
// Exercise 15: swap — gp0 and gp1 trade places via arithmetic
// ============================================================
#[test]
fn exercise_15_swap() {
    let src = std::fs::read_to_string("examples/15.swap.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    // LCD0 shows before=10 then after=20; LCD1 shows before=20 then after=10
    assert_eq!(c.devices.lcd0.history(), vec![10, 20]);
    assert_eq!(c.devices.lcd1.history(), vec![20, 10]);
}

// ============================================================
// Exercise 16: memory_array_sum — sum [10,20,30,40,50] = 150
// ============================================================
#[test]
fn exercise_16_memory_array_sum() {
    let src = std::fs::read_to_string("examples/16.memory_array_sum.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(150));
}

// ============================================================
// Exercise 17: memory_copy — copy 5 values from addr 200 to 300
// ============================================================
#[test]
fn exercise_17_memory_copy() {
    let src = std::fs::read_to_string("examples/17.memory_copy.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(10));
}

// ============================================================
// Exercise 18: string_to_tty — write "Hi!" to TTY
// ============================================================
#[test]
fn exercise_18_string_to_tty() {
    let src = std::fs::read_to_string("examples/18.string_to_tty.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.tty.contents(), "Hi!");
}

// ============================================================
// Exercise 19: power — 2^10 = 1024
// ============================================================
#[test]
fn exercise_19_power() {
    let src = std::fs::read_to_string("examples/19.power.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(1024));
}

// ============================================================
// Exercise 20: division — 100 / 5 = 20
// ============================================================
#[test]
fn exercise_20_division() {
    let src = std::fs::read_to_string("examples/20.division.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(20));
}

// ============================================================
// Exercise 21: factorial — 7! = 5040
// ============================================================
#[test]
fn exercise_21_factorial() {
    let src = std::fs::read_to_string("examples/21.factorial.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(5040));
}

// ============================================================
// Exercise 22: count_up — 1,2,3,...,10 on LCD0
// ============================================================
#[test]
fn exercise_22_count_up() {
    let src = std::fs::read_to_string("examples/22.count_up.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(
        c.devices.lcd0.history(),
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    );
}

// ============================================================
// Exercise 23: mult_table — 3*1..3*5 in memory, verify 3*4=12
// ============================================================
#[test]
fn exercise_23_mult_table() {
    let src = std::fs::read_to_string("examples/23.mult_table.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(12));
}

// ============================================================
// Exercise 24: stack_swap — push/pop swap gp0,gp1
// ============================================================
#[test]
fn exercise_24_stack_swap() {
    let src = std::fs::read_to_string("examples/24.stack_swap.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    // LCD0: before=100, after=200; LCD1: before=200, after=100
    assert_eq!(c.devices.lcd0.history(), vec![100, 200]);
    assert_eq!(c.devices.lcd1.history(), vec![200, 100]);
}

// ============================================================
// Exercise 25: add_subroutine — ADD_NUMS(3,4)=7, ADD_NUMS(10,20)=30
// ============================================================
#[test]
fn exercise_25_add_subroutine() {
    let src = std::fs::read_to_string("examples/25.add_subroutine.rpu").unwrap();
    let mut c = Computer::new();
    c.load_source(&src).unwrap();
    c.run_to_halt().unwrap();
    assert_eq!(c.devices.lcd0.last_written(), Some(7));
    assert_eq!(c.devices.lcd1.last_written(), Some(30));
}
