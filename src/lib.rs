mod registers;
mod instructions;
pub mod programs;
pub mod core;
pub mod devices;
pub mod computer;
pub mod tui;

// Top-level re-exports so integration tests and main.rs can write
// `rpu::Computer` instead of `rpu::computer::Computer`.
pub use computer::{Computer, Devices, STEP_LIMIT};
pub use core::{ExecutionError, BootError, RAM};
pub use devices::{Device, Lcd, Tty, Buffer, LCD_HISTORY_CAP, DVC_LCD0, DVC_LCD1, DVC_TTY};
pub use programs::Program;
pub use tui::{render, UiState};
