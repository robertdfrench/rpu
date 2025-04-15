use anyhow::Result;
use thiserror::Error;

#[derive(Debug)]
#[allow(non_snake_case)]
pub struct RegisterFile {
    gr0: u16,
    gr1: u16,
    gr2: u16,
    gr3: u16,
    gr4: u16,
    gr5: u16,
    gr6: u16,
    gr7: u16,
    pc:  u16,
    srA: u16
}

impl RegisterFile {
    pub fn new() -> Self {
        Self {
            gr0: 0,
            gr1: 0,
            gr2: 0,
            gr3: 0,
            gr4: 0,
            gr5: 0,
            gr6: 0,
            gr7: 0,
            pc:  0,
            srA: 0,
        }
    }

    pub fn write(&mut self, name: RegisterName, val: u16) {
        match name {
            RegisterName::gr0 => { self.gr0 = val },
            RegisterName::gr1 => { self.gr1 = val },
            RegisterName::gr2 => { self.gr2 = val },
            RegisterName::gr3 => { self.gr3 = val },
            RegisterName::gr4 => { self.gr4 = val },
            RegisterName::gr5 => { self.gr5 = val },
            RegisterName::gr6 => { self.gr6 = val },
            RegisterName::gr7 => { self.gr7 = val },
            RegisterName::out => panic!("out is a pseudo-register"),
            RegisterName::pc  => { self.pc = val},
            RegisterName::srA => { self.srA = val },
        }
    }

    pub fn read(&mut self, name: RegisterName) -> u16 {
        match name {
            RegisterName::gr0 => self.gr0,
            RegisterName::gr1 => self.gr1,
            RegisterName::gr2 => self.gr2,
            RegisterName::gr3 => self.gr3,
            RegisterName::gr4 => self.gr4,
            RegisterName::gr5 => self.gr5,
            RegisterName::gr6 => self.gr6,
            RegisterName::gr7 => self.gr7,
            RegisterName::out => panic!("out is a pseudo-register"),
            RegisterName::pc  => self.pc,
            RegisterName::srA => self.srA,
        }
    }

}


#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RegisterName {
    gr0,
    gr1,
    gr2,
    gr3,
    gr4,
    gr5,
    gr6,
    gr7,
    out,
    pc,
    srA,
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
            "gr0" => Ok(RegisterName::gr0),
            "gr1" => Ok(RegisterName::gr1),
            "gr2" => Ok(RegisterName::gr2),
            "gr3" => Ok(RegisterName::gr3),
            "gr4" => Ok(RegisterName::gr4),
            "gr5" => Ok(RegisterName::gr5),
            "gr6" => Ok(RegisterName::gr6),
            "gr7" => Ok(RegisterName::gr7),
            "out" => Ok(RegisterName::out),
            "pc"  => Ok(RegisterName::pc),
            "srA" => Ok(RegisterName::srA),
            _ => Err(ParseError::NoSuchRegisterName(s.to_string()).into())
        }
    }

    pub fn try_from_u8(x: u8) -> Result<Self> {
        match x {
            0 => Ok(RegisterName::gr0),
            1 => Ok(RegisterName::gr1),
            2 => Ok(RegisterName::gr2),
            3 => Ok(RegisterName::gr3),
            4 => Ok(RegisterName::gr4),
            5 => Ok(RegisterName::gr5),
            6 => Ok(RegisterName::gr6),
            7 => Ok(RegisterName::gr7),
            8 => Ok(RegisterName::out),
            9 => Ok(RegisterName::pc),
            10 => Ok(RegisterName::srA),
            _ => Err(DecodeError::NoSuchRegisterName(x).into())
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            RegisterName::gr0 => 0,
            RegisterName::gr1 => 1,
            RegisterName::gr2 => 2,
            RegisterName::gr3 => 3,
            RegisterName::gr4 => 4,
            RegisterName::gr5 => 5,
            RegisterName::gr6 => 6,
            RegisterName::gr7 => 7,
            RegisterName::out => 8,
            RegisterName::pc => 9,
            RegisterName::srA => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_registers() {
        let pairs = vec![
            ("gr0", RegisterName::gr0),
            ("gr1", RegisterName::gr1),
            ("gr2", RegisterName::gr2),
            ("gr3", RegisterName::gr3),
            ("gr4", RegisterName::gr4),
            ("gr5", RegisterName::gr5),
            ("gr6", RegisterName::gr6),
            ("gr7", RegisterName::gr7),
            ("out", RegisterName::out),
            ("pc", RegisterName::pc),
            ("srA", RegisterName::srA),
        ];
        for (text, expected) in pairs {
            let actual: RegisterName = RegisterName::try_from_str(text).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn decode_registers() {
        let pairs: Vec<(u8, RegisterName)> = vec![
            (0, RegisterName::gr0),
            (1, RegisterName::gr1),
            (2, RegisterName::gr2),
            (3, RegisterName::gr3),
            (4, RegisterName::gr4),
            (5, RegisterName::gr5),
            (6, RegisterName::gr6),
            (7, RegisterName::gr7),
            (8, RegisterName::out),
            (9, RegisterName::pc),
            (10, RegisterName::srA),
        ];
        for (byte, expected) in pairs {
            let actual: RegisterName = RegisterName::try_from_u8(byte).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn encode_registers() {
        let pairs: Vec<(u8, RegisterName)> = vec![
            (0, RegisterName::gr0),
            (1, RegisterName::gr1),
            (2, RegisterName::gr2),
            (3, RegisterName::gr3),
            (4, RegisterName::gr4),
            (5, RegisterName::gr5),
            (6, RegisterName::gr6),
            (7, RegisterName::gr7),
            (8, RegisterName::out),
            (9, RegisterName::pc),
            (10, RegisterName::srA),
        ];
        for (expected, register) in pairs {
            let actual: u8 = register.as_u8();
            assert_eq!(expected, actual);
        }
    }
}
