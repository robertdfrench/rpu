mod registers;
mod instructions;
pub mod programs;
pub mod core;
pub mod devices;
pub mod computer;

// Top-level re-exports so integration tests and main.rs can write
// `rpu::Computer` instead of `rpu::computer::Computer`.
pub use computer::{Computer, Devices, STEP_LIMIT};
pub use core::{ExecutionError, BootError, RAM};
pub use devices::{Device, Lcd, Tty, Buffer, LCD_HISTORY_CAP};
pub use programs::Program;
