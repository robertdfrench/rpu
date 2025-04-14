use anyhow::Result;
use thiserror::Error;

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum Register {
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
    NoSuchRegister(String)
}

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("No register associated with numerical id {0}")]
    NoSuchRegister(u8)
}

impl Register {
    pub fn try_from_str(s: &str) -> Result<Self> {
        match s {
            "gr0" => Ok(Register::gr0),
            "gr1" => Ok(Register::gr1),
            "gr2" => Ok(Register::gr2),
            "gr3" => Ok(Register::gr3),
            "gr4" => Ok(Register::gr4),
            "gr5" => Ok(Register::gr5),
            "gr6" => Ok(Register::gr6),
            "gr7" => Ok(Register::gr7),
            "out" => Ok(Register::out),
            "pc"  => Ok(Register::pc),
            "srA" => Ok(Register::srA),
            _ => Err(ParseError::NoSuchRegister(s.to_string()).into())
        }
    }

    pub fn try_from_u8(x: u8) -> Result<Self> {
        match x {
            0 => Ok(Register::gr0),
            1 => Ok(Register::gr1),
            2 => Ok(Register::gr2),
            3 => Ok(Register::gr3),
            4 => Ok(Register::gr4),
            5 => Ok(Register::gr5),
            6 => Ok(Register::gr6),
            7 => Ok(Register::gr7),
            8 => Ok(Register::out),
            9 => Ok(Register::pc),
            10 => Ok(Register::srA),
            _ => Err(DecodeError::NoSuchRegister(x).into())
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            Register::gr0 => 0,
            Register::gr1 => 1,
            Register::gr2 => 2,
            Register::gr3 => 3,
            Register::gr4 => 4,
            Register::gr5 => 5,
            Register::gr6 => 6,
            Register::gr7 => 7,
            Register::out => 8,
            Register::pc => 9,
            Register::srA => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_registers() {
        let pairs = vec![
            ("gr0", Register::gr0),
            ("gr1", Register::gr1),
            ("gr2", Register::gr2),
            ("gr3", Register::gr3),
            ("gr4", Register::gr4),
            ("gr5", Register::gr5),
            ("gr6", Register::gr6),
            ("gr7", Register::gr7),
            ("out", Register::out),
            ("pc", Register::pc),
            ("srA", Register::srA),
        ];
        for (text, expected) in pairs {
            let actual: Register = Register::try_from_str(text).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn decode_registers() {
        let pairs: Vec<(u8, Register)> = vec![
            (0, Register::gr0),
            (1, Register::gr1),
            (2, Register::gr2),
            (3, Register::gr3),
            (4, Register::gr4),
            (5, Register::gr5),
            (6, Register::gr6),
            (7, Register::gr7),
            (8, Register::out),
            (9, Register::pc),
            (10, Register::srA),
        ];
        for (byte, expected) in pairs {
            let actual: Register = Register::try_from_u8(byte).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn encode_registers() {
        let pairs: Vec<(u8, Register)> = vec![
            (0, Register::gr0),
            (1, Register::gr1),
            (2, Register::gr2),
            (3, Register::gr3),
            (4, Register::gr4),
            (5, Register::gr5),
            (6, Register::gr6),
            (7, Register::gr7),
            (8, Register::out),
            (9, Register::pc),
            (10, Register::srA),
        ];
        for (expected, register) in pairs {
            let actual: u8 = register.as_u8();
            assert_eq!(expected, actual);
        }
    }
}
