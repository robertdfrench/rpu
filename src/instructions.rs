use anyhow::Result;
use thiserror::Error;

use crate::registers::Register;

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum Instruction {
    put(u16, Register),
    add(Register, Register),
    cp(Register, Register),
    hcf
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("'{0}' is not a valid cpu instruction")]
    NoSuchInstruction(String)
}

impl Instruction {
    fn try_from_str(s: &str) -> Result<Self> {
        let p: Vec<&str> = s.trim_end().split(' ').collect();
        match p[0] {
            "hcf" => Ok(Instruction::hcf),
            "put" => {
                let val: u16 = p[1].parse()?;
                let dst = Register::try_from_str(p[2])?;
                Ok(Instruction::put(val, dst))
            },
            "add" => {
                let src = Register::try_from_str(p[1])?;
                let dst = Register::try_from_str(p[2])?;
                Ok(Instruction::add(src, dst))
            },
            "cp" => {
                let src = Register::try_from_str(p[1])?;
                let dst = Register::try_from_str(p[2])?;
                Ok(Instruction::cp(src, dst))
            },
            _ => Err(ParseError::NoSuchInstruction(p[0].to_string()).into())
        }
    }

    fn to_u32(&self) -> u32 {
        match self {
            Instruction::hcf => 0,
            Instruction::put(val, dst) => {
                let dst = dst.as_u8() as u32;
                let val = *val as u32;
                1 + val*(2^8) + dst*(2^24)
            },
            Instruction::add(src, dst) => {
                let src = src.as_u8() as u32;
                let dst = dst.as_u8() as u32;
                2 + src*(2^8) + dst*(2^16)
            },
            Instruction::cp(src, dst) => {
                let src = src.as_u8() as u32;
                let dst = dst.as_u8() as u32;
                3 + src*(2^8) + dst*(2^16)
            }
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
            ("put 7 gr0", Instruction::put(7, Register::gr0)),
            ("add gr0 gr1", Instruction::add(Register::gr0, Register::gr1)),
            ("cp srA out", Instruction::cp(Register::srA, Register::out)),
        ];
        for (text, expected) in pairs {
            let actual = Instruction::try_from_str(text).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn encode_instructions() {
        let gr0: u8 = Register::gr0.as_u8();
        let gr1: u8 = Register::gr1.as_u8();
        let gr2: u8 = Register::gr2.as_u8();
        let gr3: u8 = Register::gr3.as_u8();
        let out: u8 = Register::out.as_u8();

        let pairs = vec![
            (Instruction::hcf, 0 as u32),
            (
                Instruction::put(7, Register::gr0),
                (1 + 7*(2^8) + gr0*(2^24)) as u32
            ),
            (
                Instruction::add(Register::gr2, Register::gr1),
                (2 + gr2*(2^8) + gr1*(2^16)) as u32
            ),
            (
                Instruction::cp(Register::gr3, Register::out),
                (3 + gr3*(2^8) + out*(2^16)) as u32
            )
        ];
        for (instr, expected) in pairs {
            let actual = instr.to_u32();
            assert_eq!(actual, expected);
        }
    }
}
