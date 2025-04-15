use anyhow::Result;
use thiserror::Error;

use crate::registers::RegisterName;

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum Instruction {
    put(u16, RegisterName),
    add(RegisterName, RegisterName),
    cp(RegisterName, RegisterName),
    hcf
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("'{0}' is not a valid cpu instruction")]
    NoSuchInstruction(String)
}

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("No instruction associated with numerical id {0}")]
    NoSuchInstruction(u8)
}

impl Instruction {
    pub fn try_from_str(s: &str) -> Result<Self> {
        let p: Vec<&str> = s.trim_end().split(' ').collect();
        match p[0] {
            "hcf" => Ok(Instruction::hcf),
            "put" => {
                let val: u16 = p[1].parse()?;
                let dst = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::put(val, dst))
            },
            "add" => {
                let src = RegisterName::try_from_str(p[1])?;
                let dst = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::add(src, dst))
            },
            "cp" => {
                let src = RegisterName::try_from_str(p[1])?;
                let dst = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::cp(src, dst))
            },
            _ => Err(ParseError::NoSuchInstruction(p[0].to_string()).into())
        }
    }

    pub fn to_u32(&self) -> u32 {
        match self {
            Instruction::hcf => 0,
            Instruction::put(val, dst) => {
                let v = val.to_ne_bytes();
                u32::from_ne_bytes([1,v[0],v[1],dst.as_u8()])
            },
            Instruction::add(src, dst) => {
                u32::from_ne_bytes([2,src.as_u8(),dst.as_u8(),0])
            },
            Instruction::cp(src, dst) => {
                u32::from_ne_bytes([3,src.as_u8(),dst.as_u8(),0])
            }
        }
    }

    pub fn try_from_u32(encoded: u32) -> Result<Self> {
        let bytes = encoded.to_ne_bytes();
        let instr = bytes[0];
        match instr {
            0 => Ok(Instruction::hcf),
            1 => {
                let val = u16::from_ne_bytes([bytes[1],bytes[2]]);
                let dst = RegisterName::try_from_u8(bytes[3])?;
                Ok(Instruction::put(val, dst))
            },
            2 => {
                let src = RegisterName::try_from_u8(bytes[1])?;
                let dst = RegisterName::try_from_u8(bytes[2])?;
                Ok(Instruction::add(src, dst))
            },
            3 => {
                let src = RegisterName::try_from_u8(bytes[1])?;
                let dst = RegisterName::try_from_u8(bytes[2])?;
                Ok(Instruction::cp(src, dst))
            },
            _ => Err(DecodeError::NoSuchInstruction(instr as u8).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_instructions() {
        let pairs = vec![
            ("hcf", Instruction::hcf),
            ("put 7 gp0", Instruction::put(7, RegisterName::gp0)),
            ("add gp0 gp1", Instruction::add(RegisterName::gp0, RegisterName::gp1)),
            ("cp ans out", Instruction::cp(RegisterName::ans, RegisterName::out)),
        ];
        for (text, expected) in pairs {
            let actual = Instruction::try_from_str(text).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn encode_instructions() {
        let gp0 = RegisterName::gp0.as_u8();
        let gp1 = RegisterName::gp1.as_u8();
        let gp2 = RegisterName::gp2.as_u8();
        let gp3 = RegisterName::gp3.as_u8();
        let out = RegisterName::out.as_u8();

        let pairs = vec![
            (Instruction::hcf, 0 as u32),
            (
                Instruction::put(7, RegisterName::gp0),
                u32::from_ne_bytes([
                    1,
                    7_u16.to_ne_bytes()[0],
                    7_u16.to_ne_bytes()[1],
                    gp0
                ])
            ),
            (
                Instruction::add(RegisterName::gp2, RegisterName::gp1),
                u32::from_ne_bytes([2,gp2,gp1,0])
            ),
            (
                Instruction::cp(RegisterName::gp3, RegisterName::out),
                u32::from_ne_bytes([3,gp3,out,0])
            )
        ];
        for (instr, expected) in pairs {
            let actual = instr.to_u32();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn decode_instructions() {
        let gp0 = RegisterName::gp0.as_u8();
        let gp1 = RegisterName::gp1.as_u8();
        let gp2 = RegisterName::gp2.as_u8();
        let gp3 = RegisterName::gp3.as_u8();
        let out = RegisterName::out.as_u8();

        let pairs = vec![
            (Instruction::hcf, 0 as u32),
            (
                Instruction::put(7, RegisterName::gp0),
                u32::from_ne_bytes([
                    1,
                    7_u16.to_ne_bytes()[0],
                    7_u16.to_ne_bytes()[1],
                    gp0
                ])
            ),
            (
                Instruction::add(RegisterName::gp2, RegisterName::gp1),
                u32::from_ne_bytes([2,gp2,gp1,0])
            ),
            (
                Instruction::cp(RegisterName::gp3, RegisterName::out),
                u32::from_ne_bytes([3,gp3,out,0])
            )
        ];
        for (expected, encoded) in pairs {
            let actual = Instruction::try_from_u32(encoded).unwrap();
            assert_eq!(expected, actual);
        }
    }
}
