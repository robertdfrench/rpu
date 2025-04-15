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

    #[error("YBRPU Halted")]
    HaltAndCatchFire,

    #[error("Cannot 'put' into Register {0:?}")]
    CannotPut(RegisterName),

    #[error("Cannot 'add' from Register {0:?}")]
    CannotAdd(RegisterName),

    #[error("Cannot 'cp' from Register {0:?}")]
    CannotCpFrom(RegisterName),

    #[error("Cannot 'cp' to Register {0:?}")]
    CannotCpTo(RegisterName),
}

pub struct ProcessingUnit<'output, W: Write> {
    register_file: RegisterFile,
    // Register File
    memory: [u8; 65_536],
    output: &'output mut W
}

impl<'output, W: Write> ProcessingUnit<'output, W> {
    pub fn new(output: &'output mut W) -> Self {
        let register_file = RegisterFile::new();
        let memory = [0; 65_536];

        Self { register_file, memory, output }
    }

    fn write_output(&mut self, byte: u16) -> Result<()> {
        match writeln!(self.output, "{byte}") {
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

    fn hcf(&self) -> Result<()> {
        Err(ExecutionError::HaltAndCatchFire.into())
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
                self.write_output(val)?;
                Ok(())
            },
            _ => {
                self.register_file.write(dst, val);
                Ok(())
            }
        }
    }

    fn execute_single_instruction(&mut self) -> Result<()> {
        let mut instruction: [u8; 4] = [0; 4];
        let pc = self.register_file.read(RegisterName::pc);
        instruction[0] = self.memory[(pc as usize) + 0];
        instruction[1] = self.memory[(pc as usize) + 1];
        instruction[2] = self.memory[(pc as usize) + 2];
        instruction[3] = self.memory[(pc as usize) + 3];
        let instruction = u32::from_ne_bytes(instruction);
        let instruction = Instruction::try_from_u32(instruction)?;
        match instruction {
            Instruction::hcf => self.hcf()?,
            Instruction::put(val, dst) => self.put(val, dst)?,
            Instruction::add(x, y) => self.add(x, y)?,
            Instruction::cp(src, dst) => self.cp(src, dst)?
        }
        self.register_file.write(RegisterName::pc, pc + 4);
        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        loop {
            self.execute_single_instruction()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_register_file() {
        let mut _buffer: Vec<u8> = vec![];

        let pu = ProcessingUnit::new(&mut _buffer);

        let gp0 = pu.register_file.get(&RegisterName::gp0).unwrap();
        assert_eq!(gp0.name, RegisterName::gp0);
        assert_eq!(gp0.readable, true);
        assert_eq!(gp0.writeable, true);

        let gp1 = pu.register_file.get(&RegisterName::gp1).unwrap();
        assert_eq!(gp1.name, RegisterName::gp1);
        assert_eq!(gp1.readable, true);
        assert_eq!(gp1.writeable, true);

        let gp2 = pu.register_file.get(&RegisterName::gp2).unwrap();
        assert_eq!(gp2.name, RegisterName::gp2);
        assert_eq!(gp2.readable, true);
        assert_eq!(gp2.writeable, true);

        let gp3 = pu.register_file.get(&RegisterName::gp3).unwrap();
        assert_eq!(gp3.name, RegisterName::gp3);
        assert_eq!(gp3.readable, true);
        assert_eq!(gp3.writeable, true);

        let gp4 = pu.register_file.get(&RegisterName::gp4).unwrap();
        assert_eq!(gp4.name, RegisterName::gp4);
        assert_eq!(gp4.readable, true);
        assert_eq!(gp4.writeable, true);

        let gp5 = pu.register_file.get(&RegisterName::gp5).unwrap();
        assert_eq!(gp5.name, RegisterName::gp5);
        assert_eq!(gp5.readable, true);
        assert_eq!(gp5.writeable, true);

        let gp6 = pu.register_file.get(&RegisterName::gp6).unwrap();
        assert_eq!(gp6.name, RegisterName::gp6);
        assert_eq!(gp6.readable, true);
        assert_eq!(gp6.writeable, true);

        let gp7 = pu.register_file.get(&RegisterName::gp7).unwrap();
        assert_eq!(gp7.name, RegisterName::gp7);
        assert_eq!(gp7.readable, true);
        assert_eq!(gp7.writeable, true);

        let pc = pu.register_file.get(&RegisterName::pc).unwrap();
        assert_eq!(pc.name, RegisterName::pc);
        assert_eq!(pc.readable, true);
        assert_eq!(pc.writeable, false);

        let out = pu.register_file.get(&RegisterName::out).unwrap();
        assert_eq!(out.name, RegisterName::out);
        assert_eq!(out.readable, false);
        assert_eq!(out.writeable, true);

        #[allow(non_snake_case)]
        let ans = pu.register_file.get(&RegisterName::ans).unwrap();
        assert_eq!(ans.name, RegisterName::ans);
        assert_eq!(ans.readable, true);
        assert_eq!(ans.writeable, false);
    }

    #[test]
    fn test_output() {
        let mut buffer: Vec<u8> = vec![];

        let mut pu = ProcessingUnit::new(&mut buffer);
        pu.write_output(7).unwrap();
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

        pu.load_program(program);

        // put 7 gp0
        assert_eq!(pu.memory[0], 1);
        assert_eq!(pu.memory[1], 7);
        assert_eq!(pu.memory[2], 0);
        assert_eq!(pu.memory[3], 0);

        // cp ans out
        assert_eq!(pu.memory[4], 3);
        assert_eq!(pu.memory[5], 10);
        assert_eq!(pu.memory[6], 8);
        assert_eq!(pu.memory[7], 0);
    }
}
