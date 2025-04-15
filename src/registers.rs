use anyhow::Result;
use thiserror::Error;

#[derive(Debug)]
#[allow(non_snake_case)]
pub struct RegisterFile {
    gp0: u16,
    gp1: u16,
    gp2: u16,
    gp3: u16,
    gp4: u16,
    gp5: u16,
    gp6: u16,
    gp7: u16,
    pc:  u16,
    ans: u16
}

impl RegisterFile {
    pub fn new() -> Self {
        Self {
            gp0: 0,
            gp1: 0,
            gp2: 0,
            gp3: 0,
            gp4: 0,
            gp5: 0,
            gp6: 0,
            gp7: 0,
            pc:  0,
            ans: 0,
        }
    }

    pub fn write(&mut self, name: RegisterName, val: u16) {
        match name {
            RegisterName::gp0 => { self.gp0 = val },
            RegisterName::gp1 => { self.gp1 = val },
            RegisterName::gp2 => { self.gp2 = val },
            RegisterName::gp3 => { self.gp3 = val },
            RegisterName::gp4 => { self.gp4 = val },
            RegisterName::gp5 => { self.gp5 = val },
            RegisterName::gp6 => { self.gp6 = val },
            RegisterName::gp7 => { self.gp7 = val },
            RegisterName::out => panic!("out is a pseudo-register"),
            RegisterName::pc  => { self.pc = val},
            RegisterName::ans => { self.ans = val },
        }
    }

    pub fn read(&mut self, name: RegisterName) -> u16 {
        match name {
            RegisterName::gp0 => self.gp0,
            RegisterName::gp1 => self.gp1,
            RegisterName::gp2 => self.gp2,
            RegisterName::gp3 => self.gp3,
            RegisterName::gp4 => self.gp4,
            RegisterName::gp5 => self.gp5,
            RegisterName::gp6 => self.gp6,
            RegisterName::gp7 => self.gp7,
            RegisterName::out => panic!("out is a pseudo-register"),
            RegisterName::pc  => self.pc,
            RegisterName::ans => self.ans,
        }
    }

}


#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RegisterName {
    gp0,
    gp1,
    gp2,
    gp3,
    gp4,
    gp5,
    gp6,
    gp7,
    out,
    pc,
    ans,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("There is no register called '{0}'")]
    NoSuchRegisterName(String)
}

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("No register associated with numerical id {0}")]
    NoSuchRegisterName(u8)
}

impl RegisterName {
    pub fn try_from_str(s: &str) -> Result<Self> {
        match s {
            "gp0" => Ok(RegisterName::gp0),
            "gp1" => Ok(RegisterName::gp1),
            "gp2" => Ok(RegisterName::gp2),
            "gp3" => Ok(RegisterName::gp3),
            "gp4" => Ok(RegisterName::gp4),
            "gp5" => Ok(RegisterName::gp5),
            "gp6" => Ok(RegisterName::gp6),
            "gp7" => Ok(RegisterName::gp7),
            "out" => Ok(RegisterName::out),
            "pc"  => Ok(RegisterName::pc),
            "ans" => Ok(RegisterName::ans),
            _ => Err(ParseError::NoSuchRegisterName(s.to_string()).into())
        }
    }

    pub fn try_from_u8(x: u8) -> Result<Self> {
        match x {
            0 => Ok(RegisterName::gp0),
            1 => Ok(RegisterName::gp1),
            2 => Ok(RegisterName::gp2),
            3 => Ok(RegisterName::gp3),
            4 => Ok(RegisterName::gp4),
            5 => Ok(RegisterName::gp5),
            6 => Ok(RegisterName::gp6),
            7 => Ok(RegisterName::gp7),
            8 => Ok(RegisterName::out),
            9 => Ok(RegisterName::pc),
            10 => Ok(RegisterName::ans),
            _ => Err(DecodeError::NoSuchRegisterName(x).into())
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            RegisterName::gp0 => 0,
            RegisterName::gp1 => 1,
            RegisterName::gp2 => 2,
            RegisterName::gp3 => 3,
            RegisterName::gp4 => 4,
            RegisterName::gp5 => 5,
            RegisterName::gp6 => 6,
            RegisterName::gp7 => 7,
            RegisterName::out => 8,
            RegisterName::pc => 9,
            RegisterName::ans => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_registers() {
        let pairs = vec![
            ("gp0", RegisterName::gp0),
            ("gp1", RegisterName::gp1),
            ("gp2", RegisterName::gp2),
            ("gp3", RegisterName::gp3),
            ("gp4", RegisterName::gp4),
            ("gp5", RegisterName::gp5),
            ("gp6", RegisterName::gp6),
            ("gp7", RegisterName::gp7),
            ("out", RegisterName::out),
            ("pc", RegisterName::pc),
            ("ans", RegisterName::ans),
        ];
        for (text, expected) in pairs {
            let actual: RegisterName = RegisterName::try_from_str(text).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn decode_registers() {
        let pairs: Vec<(u8, RegisterName)> = vec![
            (0, RegisterName::gp0),
            (1, RegisterName::gp1),
            (2, RegisterName::gp2),
            (3, RegisterName::gp3),
            (4, RegisterName::gp4),
            (5, RegisterName::gp5),
            (6, RegisterName::gp6),
            (7, RegisterName::gp7),
            (8, RegisterName::out),
            (9, RegisterName::pc),
            (10, RegisterName::ans),
        ];
        for (byte, expected) in pairs {
            let actual: RegisterName = RegisterName::try_from_u8(byte).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn encode_registers() {
        let pairs: Vec<(u8, RegisterName)> = vec![
            (0, RegisterName::gp0),
            (1, RegisterName::gp1),
            (2, RegisterName::gp2),
            (3, RegisterName::gp3),
            (4, RegisterName::gp4),
            (5, RegisterName::gp5),
            (6, RegisterName::gp6),
            (7, RegisterName::gp7),
            (8, RegisterName::out),
            (9, RegisterName::pc),
            (10, RegisterName::ans),
        ];
        for (expected, register) in pairs {
            let actual: u8 = register.as_u8();
            assert_eq!(expected, actual);
        }
    }
}
