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

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("No instruction associated with numerical id {0}")]
    NoSuchInstruction(u8)
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
        let b0 = 2_u32.pow(0);
        let b1 = 2_u32.pow(8);
        let b2 = 2_u32.pow(16);
        let b3 = 2_u32.pow(24);

        match self {
            Instruction::hcf => 0,
            Instruction::put(val, dst) => {
                let dst = dst.as_u8() as u32;
                let val = *val as u32;
                1 + val*b1 + dst*b3
            },
            Instruction::add(src, dst) => {
                let src = src.as_u8() as u32;
                let dst = dst.as_u8() as u32;
                2 + src*b1 + dst*b2
            },
            Instruction::cp(src, dst) => {
                let src = src.as_u8() as u32;
                let dst = dst.as_u8() as u32;
                3 + src*b1 + dst*b2
            }
        }
    }

    fn try_from_u32(encoded: u32) -> Result<Self> {
        let b0 = 2_u32.pow(0);
        let b1 = 2_u32.pow(8);
        let b2 = 2_u32.pow(16);
        let b3 = 2_u32.pow(24);

        let instr: u32 = encoded % b1;
        let encoded: u32 = (encoded - instr)/b1;
        match instr {
            0 => Ok(Instruction::hcf),
            1 => {
                let val: u32 = encoded % b2;
                let dst: u32 = (encoded - val)/b2;
                let val: u16 = val as u16;
                let dst = Register::try_from_u8(dst as u8)?;
                Ok(Instruction::put(val, dst))
            },
            2 => {
                let src: u32 = encoded % b1;
                let dst: u32 = (encoded - src)/b1;
                let src = Register::try_from_u8(src as u8)?;
                let dst = Register::try_from_u8(dst as u8)?;
                Ok(Instruction::add(src, dst))
            },
            3 => {
                let src: u32 = encoded % b1;
                let dst: u32 = (encoded - src)/b1;
                let src = Register::try_from_u8(src as u8)?;
                let dst = Register::try_from_u8(dst as u8)?;
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
    fn math() {
        assert_eq!(2_u32.pow(0), 1);
        assert_eq!(2_u32.pow(8), 256);
        assert_eq!(2_u32.pow(16), 65536);
    }

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
        let gr0 = Register::gr0.as_u8() as u32;
        let gr1 = Register::gr1.as_u8() as u32;
        let gr2 = Register::gr2.as_u8() as u32;
        let gr3 = Register::gr3.as_u8() as u32;
        let out = Register::out.as_u8() as u32;

        let b0 = 2_u32.pow(0);
        let b1 = 2_u32.pow(8);
        let b2 = 2_u32.pow(16);
        let b3 = 2_u32.pow(24);

        let pairs = vec![
            (Instruction::hcf, 0 as u32),
            (
                Instruction::put(7, Register::gr0),
                (1 + 7*b1 + gr0*b3) as u32
            ),
            (
                Instruction::add(Register::gr2, Register::gr1),
                (2 + gr2*b1 + gr1*b2) as u32
            ),
            (
                Instruction::cp(Register::gr3, Register::out),
                (3 + gr3*b1 + out*b2) as u32
            )
        ];
        for (instr, expected) in pairs {
            let actual = instr.to_u32();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn decode_instructions() {
        let gr0 = Register::gr0.as_u8() as u32;
        let gr1 = Register::gr1.as_u8() as u32;
        let gr2 = Register::gr2.as_u8() as u32;
        let gr3 = Register::gr3.as_u8() as u32;
        let out = Register::out.as_u8() as u32;

        let b0 = 2_u32.pow(0);
        let b1 = 2_u32.pow(8);
        let b2 = 2_u32.pow(16);
        let b3 = 2_u32.pow(24);

        let pairs = vec![
            (Instruction::hcf, 0 as u32),
            (
                Instruction::put(7, Register::gr0),
                (1 + 7*b1 + gr0*b3) as u32
            ),
            (
                Instruction::add(Register::gr2, Register::gr1),
                (2 + gr2*b1 + gr1*b2) as u32
            ),
            (
                Instruction::cp(Register::gr3, Register::out),
                (3 + gr3*b1 + out*b2) as u32
            )
        ];
        for (expected, encoded) in pairs {
            let actual = Instruction::try_from_u32(encoded).unwrap();
            assert_eq!(expected, actual);
        }
    }
}
