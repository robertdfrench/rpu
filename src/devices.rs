use std::collections::VecDeque;

/// Named device IDs. Source assembly programs still write the literal
/// numbers (per the no-sugar decision in the planning doc), but Rust
/// code — tests, the TUI, the device table itself — should reach for
/// these constants instead of magic numbers. Future device stages
/// (input, screen, disk) will add new entries here.
pub const DVC_LCD0: u16 = 0;
pub const DVC_LCD1: u16 = 1;
pub const DVC_TTY:  u16 = 2;

#[derive(Debug, PartialEq)]
pub enum Error {
    Write(String),
    Read(String),
}

pub trait Device {
    fn write(&mut self, value: u16) -> Result<(), Error>;
    fn read(&mut self) -> Result<Option<u16>, Error>;
}

pub struct Buffer(pub Vec<u16>);

impl Device for Buffer {
    fn write(&mut self, value: u16) -> Result<(), Error> {
        self.0.push(value);
        Ok(())
    }

    fn read(&mut self) -> Result<Option<u16>, Error> {
        Ok(self.0.pop())
    }
}

/// How many recent values an `Lcd` keeps. Older entries fall off
/// the front. 1024 is roomy enough that no realistic teaching
/// program will fill it in a single run, and small enough that
/// `history()`'s clone is ~2 KiB worst case.
pub const LCD_HISTORY_CAP: usize = 1024;

/// A simple display device that records every value written to it.
/// The TUI renders only the most recent value (via `last_written()`)
/// using a 7-segment font; tests can inspect the full sequence with
/// `history()`.
#[derive(Default)]
pub struct Lcd {
    history: VecDeque<u16>,
}

impl Lcd {
    pub fn new() -> Self {
        Self { history: VecDeque::new() }
    }

    /// The most recent value written, or `None` if nothing has been
    /// written since power-on.
    pub fn last_written(&self) -> Option<u16> {
        self.history.back().copied()
    }

    /// Returns the recorded outputs in oldest-to-newest order, capped
    /// at the most recent `LCD_HISTORY_CAP` writes. Cloned for
    /// ergonomic test asserts; the renderer uses `last_written()`
    /// instead and never calls this.
    pub fn history(&self) -> Vec<u16> {
        self.history.iter().copied().collect()
    }
}

impl Device for Lcd {
    fn write(&mut self, value: u16) -> Result<(), Error> {
        if self.history.len() == LCD_HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(value);
        Ok(())
    }

    fn read(&mut self) -> Result<Option<u16>, Error> {
        Err(Error::Read(String::from("The LCD isn't an input")))
    }
}

/// A teletype-style output device. Each `u16` written is interpreted
/// as a UTF-16 code unit and appended to a string buffer. Used for
/// the error console pane in the TUI and for any program that wants
/// to print text instead of numbers.
#[derive(Default)]
pub struct Tty {
    buffer: String,
}

impl Tty {
    pub fn new() -> Self {
        Self { buffer: String::new() }
    }

    pub fn contents(&self) -> &str {
        &self.buffer
    }

    /// Append a line of text to the buffer. Used by the TUI's error
    /// console to surface CPU errors next to whatever the program
    /// has already printed.
    pub fn push_line(&mut self, line: &str) {
        self.buffer.push_str(line);
        self.buffer.push('\n');
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_demo() {
        let mut b = Buffer(vec![]);
        b.write(5).unwrap();
        assert_eq!(b.read().unwrap(), Some(5));
    }

    #[test]
    fn lcd_records_history_in_order() {
        let mut lcd = Lcd::new();
        lcd.write(5).unwrap();
        lcd.write(7).unwrap();
        lcd.write(12).unwrap();
        assert_eq!(lcd.history(), vec![5, 7, 12]);
        assert_eq!(lcd.last_written(), Some(12));
    }

    #[test]
    fn lcd_history_caps_at_limit() {
        let mut lcd = Lcd::new();
        for i in 0..(LCD_HISTORY_CAP + 5) {
            lcd.write(i as u16).unwrap();
        }
        let history = lcd.history();
        assert_eq!(history.len(), LCD_HISTORY_CAP);
        // Oldest 5 entries fell off the front.
        assert_eq!(history[0], 5);
        assert_eq!(*history.last().unwrap(), (LCD_HISTORY_CAP + 4) as u16);
    }

    #[test]
    fn tty_appends_utf16_chars() {
        let mut tty = Tty::new();
        tty.write(b'H' as u16).unwrap();
        tty.write(b'i' as u16).unwrap();
        assert_eq!(tty.contents(), "Hi");
    }
}
