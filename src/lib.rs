mod registers;
mod instructions;
mod programs;

use std::io::Write;
use instructions::Instruction;
use registers::RegisterName;
use registers::RegisterFile;
use programs::Program;

use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Program is too damn big: {0} bytes")]
    ProgramTooBig(usize),

    #[error("Cannot 'put' into Register {0:?}")]
    CannotPut(RegisterName),

    #[error("Cannot 'add' from Register {0:?}")]
    CannotAdd(RegisterName),

    #[error("Cannot 'cp' from Register {0:?}")]
    CannotCpFrom(RegisterName),

    #[error("Cannot 'cp' to Register {0:?}")]
    CannotCpTo(RegisterName),
}

pub struct ProcessingUnit<'tty, W: Write> {
    register_file: RegisterFile,
    // Register File
    memory: [u8; 65_536],
    tty: &'tty mut W
}

impl<'tty, W: Write> ProcessingUnit<'tty, W> {
    pub fn new(tty: &'tty mut W) -> Self {
        let register_file = RegisterFile::new();
        let memory = [0; 65_536];

        Self { register_file, memory, tty }
    }

    fn write_tty(&mut self, byte: u16) -> Result<()> {
        match writeln!(self.tty, "{byte}") {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into())
        }
    }

    fn load_program(&mut self, program: Program) -> Result<()> {
        if program.size() >= 65_536 {
            return Err(ExecutionError::ProgramTooBig(program.size()).into());
        }
        for (i, byte) in program.bytes().enumerate() {
            self.memory[i] = byte;
        }
        Ok(())
    }

    pub fn load_source(&mut self, source: &str) -> Result<()> {
        let program = Program::try_compile(source)?;
        self.load_program(program)?;
        Ok(())
    }

    fn put(&mut self, val: u16, dst: RegisterName) -> Result<()> {
        match dst {
            RegisterName::pc => Err(ExecutionError::CannotPut(dst).into()),
            RegisterName::ans => Err(ExecutionError::CannotPut(dst).into()), 
            RegisterName::out => Err(ExecutionError::CannotPut(dst).into()),
            _ => {
                self.register_file.write(dst, val);
                Ok(())
            }
        }
    }

    fn add(&mut self, x: RegisterName, y: RegisterName) -> Result<()> {
        let x: u16 = match x {
            RegisterName::out => {
                return Err(ExecutionError::CannotAdd(x).into());
            },
            _ => self.register_file.read(x)
        };

        let y: u16 = match y {
            RegisterName::out => {
                return Err(ExecutionError::CannotAdd(y).into());
            },
            _ => self.register_file.read(y)
        };

        self.register_file.write(RegisterName::ans, x+y);

        Ok(())
    }

    fn cp(&mut self, src: RegisterName, dst: RegisterName) -> Result<()> {
        let val = match src {
            RegisterName::out => {
                return Err(ExecutionError::CannotCpFrom(dst).into());
            },
            _ => self.register_file.read(src)
        };

        match dst {
            RegisterName::pc => Err(ExecutionError::CannotCpTo(dst).into()),
            RegisterName::ans => Err(ExecutionError::CannotCpTo(dst).into()), 
            RegisterName::out => {
                self.write_tty(val)?;
                Ok(())
            },
            _ => {
                self.register_file.write(dst, val);
                Ok(())
            }
        }
    }

    fn jmp(&mut self, addr: RegisterName) -> Result<()> {
        let addr = self.register_file.read(addr);
        let ans = self.register_file.read(RegisterName::ans);
        if ans == 0 {
            self.register_file.write(RegisterName::pc, addr);
        }
        Ok(())
    }

    fn execute_single_instruction(&mut self) -> Result<bool> {
        let mut instruction: [u8; 4] = [0; 4];
        let pc = self.register_file.read(RegisterName::pc);
        instruction[0] = self.memory[(pc as usize) + 0];
        instruction[1] = self.memory[(pc as usize) + 1];
        instruction[2] = self.memory[(pc as usize) + 2];
        instruction[3] = self.memory[(pc as usize) + 3];
        let instruction = u32::from_ne_bytes(instruction);
        let instruction = Instruction::try_from_u32(instruction)?;
        match instruction {
            Instruction::hcf => { return Ok(true) },
            Instruction::add(x, y) => self.add(x, y)?,
            Instruction::cp(src, dst) => self.cp(src, dst)?,
            Instruction::jmp(addr) => self.jmp(addr)?,
            Instruction::put(val, dst) => self.put(val, dst)?,
        }
        let pc = self.register_file.read(RegisterName::pc);
        self.register_file.write(RegisterName::pc, pc + 4);
        Ok(false)
    }

    pub fn start(&mut self) -> Result<()> {
        self.register_file.write(RegisterName::pc, 0);
        loop {
            match self.execute_single_instruction() {
                Err(e) => { return Err(e); },
                Ok(halt) => {
                    if halt {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instructions::InstructionName;

    #[test]
    fn test_tty() {
        let mut buffer: Vec<u8> = vec![];

        let mut pu = ProcessingUnit::new(&mut buffer);
        pu.write_tty(7).unwrap();
        let actual = String::from_utf8(buffer).unwrap();
        assert_eq!(&actual, "7\n");
    }

    #[test]
    fn test_loading() {
        let mut _buffer: Vec<u8> = vec![];

        let mut pu = ProcessingUnit::new(&mut _buffer);

        let source = [
            "put 7 gp0",
            "cp ans out"
        ];
        let source = source.join("\n");
        let program = Program::try_compile(&source).unwrap();

        pu.load_program(program).unwrap();

        // put 7 gp0
        assert_eq!(pu.memory[0], InstructionName::put as u8);
        assert_eq!(pu.memory[1], 7);
        assert_eq!(pu.memory[2], 0);
        assert_eq!(pu.memory[3], RegisterName::gp0 as u8);

        // cp ans out
        assert_eq!(pu.memory[4], InstructionName::cp as u8);
        assert_eq!(pu.memory[5], RegisterName::ans as u8);
        assert_eq!(pu.memory[6], RegisterName::out as u8);
        assert_eq!(pu.memory[7], 0);
    }
}
