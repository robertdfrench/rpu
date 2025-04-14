use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccessError {
    #[error("You tried to write to register '{0:?}', but it is read-only")]
    CannotWrite(RegisterName),

    #[error("You tried to read from register '{0:?}', but it is write-only")]
    CannotRead(RegisterName)
}

#[derive(Debug)]
pub struct Register {
    pub name: RegisterName,
    pub readable: bool,
    pub writeable: bool,
    pub value: u16
}

impl Register {
    pub fn new_ro(name: RegisterName) -> Self {
        Self {
            name,
            readable: true,
            writeable: false,
            value: 0
        }
    }

    pub fn new_wo(name: RegisterName) -> Self {
        Self {
            name,
            readable: false,
            writeable: true,
            value: 0
        }
    }

    pub fn new_rw(name: RegisterName) -> Self {
        Self {
            name,
            readable: true,
            writeable: true,
            value: 0
        }
    }

    pub fn try_write(&mut self, value: u16) -> Result<()> {
        if !self.writeable {
            Err(AccessError::CannotWrite(self.name).into())
        } else {
            self.value = value;
            Ok(())
        }
    }

    pub fn try_read(&self) -> Result<u16> {
        if !self.readable {
            Err(AccessError::CannotRead(self.name).into())
        } else {
            Ok(self.value)
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
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

    #[test]
    #[should_panic]
    fn enforce_register_readonly() {
        let mut gr0 = Register::new_ro(RegisterName::gr0);
        gr0.try_write(7).unwrap();
    }

    #[test]
    fn read_from_ro_register() {
        let mut gr0 = Register::new_ro(RegisterName::gr0);
        gr0.value = 5;
        assert_eq!(gr0.try_read().unwrap(), 5);
    }

    #[test]
    #[should_panic]
    fn enforce_register_writeonly() {
        let gr0 = Register::new_wo(RegisterName::gr0);
        gr0.try_read().unwrap();
    }

    #[test]
    fn write_to_wo_register() {
        let mut gr0 = Register::new_wo(RegisterName::gr0);
        gr0.try_write(7).unwrap();
        assert_eq!(gr0.value, 7);
    }

    #[test]
    fn rw_register() {
        let mut gr0 = Register::new_rw(RegisterName::gr0);
        gr0.try_write(7).unwrap();
        assert_eq!(gr0.try_read().unwrap(), 7);
    }
}
