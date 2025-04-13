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
                let dst: Register = p[2].parse()?;
                Ok(Instruction::put(val, dst))
            },
            "add" => {
                let src: Register = p[1].parse()?;
                let dst: Register = p[2].parse()?;
                Ok(Instruction::add(src, dst))
            },
            _ => Err(ParseError::NoSuchInstruction(p[0].to_string()).into())
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
        ];
        for (text, expected) in pairs {
            let actual = Instruction::try_from_str(text).unwrap();
            assert_eq!(actual, expected);
        }
    }
}
