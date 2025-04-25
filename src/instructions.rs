use anyhow::Result;
use thiserror::Error;

use crate::registers::RegisterName;

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum InstructionName {
    // This must come first so that "0" instrs halt the machine
    halt = 0,
    add,
    copy,
    jump,
    read,
    mul,
    noop,
    put,
    write,
    sub,
}

const HALT_ID:  u8 = InstructionName::halt  as u8;
const ADD_ID:   u8 = InstructionName::add   as u8;
const COPY_ID:  u8 = InstructionName::copy  as u8;
const JUMP_ID:  u8 = InstructionName::jump  as u8;
const MUL_ID:   u8 = InstructionName::mul   as u8;
const NOOP_ID:  u8 = InstructionName::noop  as u8;
const PUT_ID:   u8 = InstructionName::put   as u8;
const SUB_ID:   u8 = InstructionName::sub   as u8;
const WRITE_ID: u8 = InstructionName::write as u8;
const READ_ID:  u8 = InstructionName::read  as u8;

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum Instruction {
    halt,
    add(RegisterName, RegisterName),
    copy(RegisterName, RegisterName),
    jump(RegisterName, RegisterName),
    mul(RegisterName, RegisterName),
    noop,
    put(u16, RegisterName),
    sub(RegisterName, RegisterName),
    write(RegisterName, RegisterName),
    read(RegisterName, RegisterName),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("'{0}' is not a valid rpu instruction")]
    NoSuchInstruction(String)
}

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("No instruction associated with numerical id {0}")]
    NoSuchInstruction(u8)
}

impl Instruction {
    pub fn try_from_str(s: &str) -> Result<Self> {
        let p: Vec<&str> = s
            .trim_end()
            .split_whitespace()
            .collect();
        match p[0] {
            "halt" => Ok(Instruction::halt),
            "add" => {
                let x = RegisterName::try_from_str(p[1])?;
                let y = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::add(x, y))
            },
            "copy" => {
                let src = RegisterName::try_from_str(p[1])?;
                let dst = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::copy(src, dst))
            },
            "jump" => {
                let addr = RegisterName::try_from_str(p[1])?;
                let cond = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::jump(addr, cond))
            },
            "mul" => {
                let x = RegisterName::try_from_str(p[1])?;
                let y = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::mul(x, y))
            },
            "noop" => Ok(Instruction::noop),
            "put" => {
                let val: u16 = p[1].parse()?;
                let dst = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::put(val, dst))
            },
            "sub" => {
                let x = RegisterName::try_from_str(p[1])?;
                let y = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::sub(x, y))
            },
            "write" => {
                let src = RegisterName::try_from_str(p[1])?;
                let addr = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::write(src, addr))
            },
            "read" => {
                let addr = RegisterName::try_from_str(p[1])?;
                let dst = RegisterName::try_from_str(p[2])?;
                Ok(Instruction::read(addr, dst))
            },
            _ => Err(ParseError::NoSuchInstruction(p[0].to_string()).into())
        }
    }

    pub fn to_u32(&self) -> u32 {
        match self {
            Instruction::halt => {
                u32::from_ne_bytes([HALT_ID,0,0,0])
            },
            Instruction::add(x, y) => {
                u32::from_ne_bytes([ADD_ID,*x as u8,*y as u8,0])
            },
            Instruction::copy(src, dst) => {
                u32::from_ne_bytes([COPY_ID,*src as u8,*dst as u8,0])
            },
            Instruction::jump(addr, cond) => {
                u32::from_ne_bytes([JUMP_ID,*addr as u8,*cond as u8,0])
            },
            Instruction::mul(x, y) => {
                u32::from_ne_bytes([MUL_ID,*x as u8,*y as u8,0])
            },
            Instruction::noop => {
                u32::from_ne_bytes([NOOP_ID,0,0,0])
            }
            Instruction::put(val, dst) => {
                let v = val.to_ne_bytes();
                u32::from_ne_bytes([PUT_ID,v[0],v[1],*dst as u8])
            },
            Instruction::sub(x, y) => {
                u32::from_ne_bytes([SUB_ID,*x as u8,*y as u8,0])
            },
            Instruction::write(src, addr) => {
                u32::from_ne_bytes([WRITE_ID,*src as u8,*addr as u8,0])
            },
            Instruction::read(addr, dst) => {
                u32::from_ne_bytes([READ_ID,*addr as u8,*dst as u8,0])
            },
        }
    }

    pub fn try_from_u32(encoded: u32) -> Result<Self> {
        let bytes = encoded.to_ne_bytes();
        let instr = bytes[0];
        match instr {
            HALT_ID => Ok(Instruction::halt),
            ADD_ID => {
                let x = RegisterName::try_from_u8(bytes[1])?;
                let y = RegisterName::try_from_u8(bytes[2])?;
                Ok(Instruction::add(x, y))
            },
            COPY_ID => {
                let src = RegisterName::try_from_u8(bytes[1])?;
                let dst = RegisterName::try_from_u8(bytes[2])?;
                Ok(Instruction::copy(src, dst))
            },
            JUMP_ID => {
                let addr = RegisterName::try_from_u8(bytes[1])?;
                let cond = RegisterName::try_from_u8(bytes[2])?;
                Ok(Instruction::jump(addr, cond))
            },
            MUL_ID => {
                let x = RegisterName::try_from_u8(bytes[1])?;
                let y = RegisterName::try_from_u8(bytes[2])?;
                Ok(Instruction::mul(x, y))
            },
            NOOP_ID => Ok(Instruction::noop),
            PUT_ID => {
                let val = u16::from_ne_bytes([bytes[1],bytes[2]]);
                let dst = RegisterName::try_from_u8(bytes[3])?;
                Ok(Instruction::put(val, dst))
            },
            SUB_ID => {
                let x = RegisterName::try_from_u8(bytes[1])?;
                let y = RegisterName::try_from_u8(bytes[2])?;
                Ok(Instruction::sub(x, y))
            },
            WRITE_ID => {
                let src = RegisterName::try_from_u8(bytes[1])?;
                let addr = RegisterName::try_from_u8(bytes[2])?;
                Ok(Instruction::write(src, addr))
            },
            READ_ID => {
                let addr = RegisterName::try_from_u8(bytes[1])?;
                let dst = RegisterName::try_from_u8(bytes[2])?;
                Ok(Instruction::read(addr, dst))
            },
            _ => Err(
                    DecodeError::NoSuchInstruction(instr as u8)
                        .into()
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_instructions() {
        let pairs = vec![
            (
                "halt", 
                Instruction::halt
            ),
            (
                "add gp0 gp1",
                Instruction::add(
                    RegisterName::gp0,
                    RegisterName::gp1
                )
            ),
            (
                "copy ans out",
                Instruction::copy(
                    RegisterName::ans,
                    RegisterName::out
                )
            ),
            (
                "jump gp2 gp7",
                Instruction::jump(
                    RegisterName::gp2,
                    RegisterName::gp7
                )
            ),
            (
                "mul gp0 gp1",
                Instruction::mul(
                    RegisterName::gp0,
                    RegisterName::gp1
                )
            ),
            (
                "noop",
                Instruction::noop
            ),
            (
                "put 7 gp0",
                Instruction::put(7, RegisterName::gp0)
            ),
            (
                "sub gp0 gp1",
                Instruction::sub(
                    RegisterName::gp0,
                    RegisterName::gp1
                )
            ),
            (
                "write gp0 gp1",
                Instruction::write(
                    RegisterName::gp0,
                    RegisterName::gp1
                )
            ),
            (
                "read gp0 gp1",
                Instruction::read(
                    RegisterName::gp0,
                    RegisterName::gp1
                )
            ),
        ];
        for (text, expected) in pairs {
            let actual = Instruction::try_from_str(text).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn encode_instructions() {
        let pairs = vec![
            (
                Instruction::halt,
                u32::from_ne_bytes([
                    InstructionName::halt as u8,
                    0,
                    0,
                    0
                ])
            ),
            (
                Instruction::add(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::add as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
            (
                Instruction::copy(
                    RegisterName::gp3,
                    RegisterName::out
                ),
                u32::from_ne_bytes([
                    InstructionName::copy as u8,
                    RegisterName::gp3 as u8,
                    RegisterName::out as u8,
                    0
                ])
            ),
            (
                Instruction::jump(
                    RegisterName::gp7,
                    RegisterName::gp6
                ),
                u32::from_ne_bytes([
                    InstructionName::jump as u8,
                    RegisterName::gp7 as u8,
                    RegisterName::gp6 as u8,
                    0
                ])
            ),
            (
                Instruction::mul(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::mul as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
            (
                Instruction::noop,
                u32::from_ne_bytes([
                    InstructionName::noop as u8,
                    0,
                    0,
                    0
                ])
            ),
            (
                Instruction::put(7, RegisterName::gp0),
                u32::from_ne_bytes([
                    InstructionName::put as u8,
                    7_u16.to_ne_bytes()[0],
                    7_u16.to_ne_bytes()[1],
                    RegisterName::gp0 as u8
                ])
            ),
            (
                Instruction::sub(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::sub as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
            (
                Instruction::write(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::write as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
            (
                Instruction::read(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::read as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
        ];
        for (instr, expected) in pairs {
            let actual = instr.to_u32();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn decode_instructions() {
        let pairs = vec![
            (
                Instruction::halt,
                u32::from_ne_bytes([
                    InstructionName::halt as u8,
                    0,
                    0,
                    0
                ])
            ),
            (
                Instruction::add(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::add as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
            (
                Instruction::copy(
                    RegisterName::gp3,
                    RegisterName::out
                ),
                u32::from_ne_bytes([
                    InstructionName::copy as u8,
                    RegisterName::gp3 as u8,
                    RegisterName::out as u8,
                    0
                ])
            ),
            (
                Instruction::jump(
                    RegisterName::gp7,
                    RegisterName::gp6
                ),
                u32::from_ne_bytes([
                    InstructionName::jump as u8,
                    RegisterName::gp7 as u8,
                    RegisterName::gp6 as u8,
                    0
                ])
            ),
            (
                Instruction::mul(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::mul as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
            (
                Instruction::noop,
                u32::from_ne_bytes([
                    InstructionName::noop as u8,
                    0,
                    0,
                    0
                ])
            ),
            (
                Instruction::put(7, RegisterName::gp0),
                u32::from_ne_bytes([
                    InstructionName::put as u8,
                    7_u16.to_ne_bytes()[0],
                    7_u16.to_ne_bytes()[1],
                    RegisterName::gp0 as u8
                ])
            ),
            (
                Instruction::sub(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::sub as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
            (
                Instruction::write(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::write as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
            (
                Instruction::read(
                    RegisterName::gp2,
                    RegisterName::gp1
                ),
                u32::from_ne_bytes([
                    InstructionName::read as u8,
                    RegisterName::gp2 as u8,
                    RegisterName::gp1 as u8,
                    0
                ])
            ),
        ];
        for (expected, encoded) in pairs {
            let actual = Instruction::try_from_u32(encoded)
                .unwrap();
            assert_eq!(expected, actual);
        }
    }
}
