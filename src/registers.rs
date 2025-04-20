use anyhow::Result;
use thiserror::Error;

#[derive(Debug)]
#[allow(non_snake_case)]
pub struct RegisterFile {
    pub gp0: u16,
    pub gp1: u16,
    pub gp2: u16,
    pub gp3: u16,
    pub gp4: u16,
    pub gp5: u16,
    pub gp6: u16,
    pub gp7: u16,

    pub ans: u16,
    pub pc:  u16,
    pub dvc: u16,
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

            ans: 0,
            dvc: 0,
            pc:  0,
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

            RegisterName::ans => { self.ans = val },
            RegisterName::dvc => { self.dvc = val },
            RegisterName::inp => panic!("inp is a pseudo-register"),
            RegisterName::out => panic!("out is a pseudo-register"),
            RegisterName::pc  => { self.pc = val},
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

            RegisterName::ans => self.ans,
            RegisterName::dvc => self.dvc,
            RegisterName::inp => panic!("inp is a pseudo-register"),
            RegisterName::out => panic!("out is a pseudo-register"),
            RegisterName::pc  => self.pc,
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
    ans,
    dvc,
    inp,
    out,
    pc,
}

const GP0_ID: u8 = RegisterName::gp0 as u8;
const GP1_ID: u8 = RegisterName::gp1 as u8;
const GP2_ID: u8 = RegisterName::gp2 as u8;
const GP3_ID: u8 = RegisterName::gp3 as u8;
const GP4_ID: u8 = RegisterName::gp4 as u8;
const GP5_ID: u8 = RegisterName::gp5 as u8;
const GP6_ID: u8 = RegisterName::gp6 as u8;
const GP7_ID: u8 = RegisterName::gp7 as u8;
const ANS_ID: u8 = RegisterName::ans as u8;
const DVC_ID: u8 = RegisterName::dvc as u8;
const INP_ID: u8 = RegisterName::inp as u8;
const OUT_ID: u8 = RegisterName::out as u8;
const PC_ID: u8 = RegisterName::pc as u8;

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

            "ans" => Ok(RegisterName::ans),
            "dvc" => Ok(RegisterName::dvc),
            "inp" => Ok(RegisterName::inp),
            "out" => Ok(RegisterName::out),
            "pc"  => Ok(RegisterName::pc),
            _ => Err(ParseError::NoSuchRegisterName(s.to_string()).into())
        }
    }

    pub fn try_from_u8(x: u8) -> Result<Self> {
        match x {
            GP0_ID => Ok(RegisterName::gp0),
            GP1_ID => Ok(RegisterName::gp1),
            GP2_ID => Ok(RegisterName::gp2),
            GP3_ID => Ok(RegisterName::gp3),
            GP4_ID => Ok(RegisterName::gp4),
            GP5_ID => Ok(RegisterName::gp5),
            GP6_ID => Ok(RegisterName::gp6),
            GP7_ID => Ok(RegisterName::gp7),

            ANS_ID => Ok(RegisterName::ans),
            DVC_ID => Ok(RegisterName::dvc),
            INP_ID => Ok(RegisterName::inp),
            OUT_ID => Ok(RegisterName::out),
            PC_ID => Ok(RegisterName::pc),
            _ => Err(DecodeError::NoSuchRegisterName(x).into())
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
            ("ans", RegisterName::ans),
            ("dvc", RegisterName::dvc),
            ("inp", RegisterName::inp),
            ("out", RegisterName::out),
            ("pc", RegisterName::pc),
        ];
        for (text, expected) in pairs {
            let actual: RegisterName = RegisterName::try_from_str(text).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn decode_registers() {
        let pairs: Vec<(u8, RegisterName)> = vec![
            (GP0_ID, RegisterName::gp0),
            (GP1_ID, RegisterName::gp1),
            (GP2_ID, RegisterName::gp2),
            (GP3_ID, RegisterName::gp3),
            (GP4_ID, RegisterName::gp4),
            (GP5_ID, RegisterName::gp5),
            (GP6_ID, RegisterName::gp6),
            (GP7_ID, RegisterName::gp7),
            (ANS_ID, RegisterName::ans),
            (DVC_ID, RegisterName::dvc),
            (INP_ID, RegisterName::inp),
            (OUT_ID, RegisterName::out),
            (PC_ID, RegisterName::pc),
        ];
        for (byte, expected) in pairs {
            let actual: RegisterName = RegisterName::try_from_u8(byte).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn encode_registers() {
        let pairs: Vec<(u8, RegisterName)> = vec![
            (GP0_ID, RegisterName::gp0),
            (GP1_ID, RegisterName::gp1),
            (GP2_ID, RegisterName::gp2),
            (GP3_ID, RegisterName::gp3),
            (GP4_ID, RegisterName::gp4),
            (GP5_ID, RegisterName::gp5),
            (GP6_ID, RegisterName::gp6),
            (GP7_ID, RegisterName::gp7),
            (ANS_ID, RegisterName::ans),
            (DVC_ID, RegisterName::dvc),
            (INP_ID, RegisterName::inp),
            (OUT_ID, RegisterName::out),
            (PC_ID, RegisterName::pc),
        ];
        for (expected, register) in pairs {
            let actual: u8 = register as u8;
            assert_eq!(expected, actual);
        }
    }
}
