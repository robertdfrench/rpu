use crate::instructions::Instruction;
use crate::registers::RegisterName;
use crate::registers::RegisterFile;
use crate::programs::Program;
use crate::programs;
use crate::instructions;
use crate::registers;
use crate::devices::Device;

#[derive(Debug, PartialEq)]
pub enum ExecutionError {
    CannotPut(RegisterName),

    CannotAdd(RegisterName),

    CannotCpFrom(RegisterName),

    CannotCpTo(RegisterName),

    Overflow(u16, u16),

    Underflow(u16, u16),

    StackOverflow,

    StackUnderflow,

    Decode(instructions::DecodeError),

    Access(registers::AccessError)
}

impl From<instructions::DecodeError> for ExecutionError {
    fn from(other: instructions::DecodeError) -> Self {
        Self::Decode(other)
    }
}

impl From<registers::AccessError> for ExecutionError {
    fn from(other: registers::AccessError) -> Self {
        Self::Access(other)
    }
}

pub struct Core {
    pub register_file: RegisterFile,
    // Register File
    pub memory: [u8; 65_536],
    pub tty: String,
    pub power: bool
}

#[derive(Debug)]
pub enum BootError {
    ProgramTooBig(usize),
    Compilation(programs::CompilationError)
}

impl From<programs::CompilationError> for BootError {
    fn from(other: programs::CompilationError) -> Self {
        Self::Compilation(other)
    }
}

impl Core {
    pub fn new()
        -> Self
    {
        let register_file = RegisterFile::new();
        let memory = [0; 65_536];
        let tty = String::new();
        let power = true;

        Self { register_file, memory, tty, power }
    }

    fn write_tty(&mut self, byte: u16) {
        let byte = String::from_utf16_lossy(&[byte]);
        self.tty.push_str(&byte);
    }

    pub fn load_program(&mut self, program: &Program)
        -> Result<(),BootError>
    {
        if program.size() >= 65_536 {
            return Err(BootError::ProgramTooBig(program.size()));
        }
        for (i, byte) in program.bytes().enumerate() {
            self.memory[i] = byte;
        }
        Ok(())
    }

    pub fn load_source(&mut self, source: &str)
        -> Result<(), BootError>
    {
        let program = Program::try_compile(source)?;
        self.load_program(&program)?;
        Ok(())
    }

    fn put(&mut self, val: u16, dst: RegisterName)
        -> Result<(), ExecutionError>
    {
        match dst {
            RegisterName::pc => Err(
                ExecutionError::CannotPut(dst)
            ),
            RegisterName::ans => Err(
                ExecutionError::CannotPut(dst)
            ),
            RegisterName::out => Err(
                ExecutionError::CannotPut(dst)
            ),
            _ => {
                self.register_file.write(dst, val)?;
                Ok(())
            }
        }
    }

    fn add(&mut self, x: RegisterName, y: RegisterName)
        -> Result<(), ExecutionError>
    {
        let x: u16 = match x {
            RegisterName::out => {
                return Err(ExecutionError::CannotAdd(x));
            },
            _ => self.register_file.read(x)?
        };

        let y: u16 = match y {
            RegisterName::out => {
                return Err(ExecutionError::CannotAdd(y));
            },
            _ => self.register_file.read(y)?
        };

        let ans = x.checked_add(y).ok_or(
            ExecutionError::Overflow(x,y)
        )?;
        self.register_file.write(RegisterName::ans, ans)?;

        Ok(())
    }

    fn copy(&mut self,
        src: RegisterName,
        dst: RegisterName,
        devices: &mut [&mut dyn Device],
    )
        -> Result<(), ExecutionError>
    {
        let val = match src {
            RegisterName::out => {
                return Err(
                    ExecutionError::CannotCpFrom(dst)
                );
            },
            _ => self.register_file.read(src)?
        };

        match dst {
            RegisterName::pc => Err(
                ExecutionError::CannotCpTo(dst)
            ),
            RegisterName::ans => Err(
                ExecutionError::CannotCpTo(dst)
            ), 
            RegisterName::out => {
                match self.register_file.dvc {
                    0 => { devices[0].write(val).unwrap(); },
                    1 => { devices[1].write(val).unwrap(); },
                    _ => { self.write_tty(val); },
                }
                Ok(())
            },
            _ => {
                self.register_file.write(dst, val)?;
                Ok(())
            }
        }
    }

    fn jump(&mut self, addr: RegisterName, cond: RegisterName)
        -> Result<(), ExecutionError>
    {
        let mut addr = self.register_file.read(addr)?;
        addr = addr - (addr % 4); // Align addr to 4n
        if addr > 0 {
            // Back up to previous address unless that would go
            // negative. This means that `jump 4` and `jump 0`
            // have the same behavior.
            addr = addr - 4;
        }
        let cond = self.register_file.read(cond)?;
        if cond == 0 {
            self.register_file.write(RegisterName::pc, addr)?;
        }
        Ok(())
    }

    fn mul(&mut self, x: RegisterName, y: RegisterName)
        -> Result<(), ExecutionError>
    {
        let x: u16 = match x {
            RegisterName::out => {
                return Err(ExecutionError::CannotAdd(x));
            },
            _ => self.register_file.read(x)?
        };

        let y: u16 = match y {
            RegisterName::out => {
                return Err(ExecutionError::CannotAdd(y));
            },
            _ => self.register_file.read(y)?
        };

        let ans = x.checked_mul(y).ok_or(
            ExecutionError::Overflow(x,y)
        )?;
        self.register_file.write(RegisterName::ans, ans)?;

        Ok(())
    }

    fn sub(&mut self, x: RegisterName, y: RegisterName)
        -> Result<(), ExecutionError>
    {
        let x: u16 = match x {
            RegisterName::out => {
                return Err(ExecutionError::CannotAdd(x));
            },
            _ => self.register_file.read(x)?
        };

        let y: u16 = match y {
            RegisterName::out => {
                return Err(ExecutionError::CannotAdd(y));
            },
            _ => self.register_file.read(y)?
        };

        let ans = x.checked_sub(y).ok_or(
            ExecutionError::Underflow(x,y)
        )?;
        self.register_file.write(RegisterName::ans, ans)?;

        Ok(())
    }

    fn pop(&mut self, dst: RegisterName)
        -> Result<(), ExecutionError>
    {
        let sp = self.register_file.read(RegisterName::sp)?;
        if sp == 65_534 {
            return Err(ExecutionError::StackUnderflow);
        }

        self.register_file.write(RegisterName::sp, sp + 2)?;
        let sp = self.register_file.read(RegisterName::sp)?;

        let mut val: [u8; 2] = [0; 2];
        val[0] = self.memory[sp as usize];
        val[1] = self.memory[(sp + 1) as usize];
        let val = u16::from_ne_bytes(val);

        self.put(val, dst)
    }

    fn push(&mut self, src: RegisterName)
        -> Result<(), ExecutionError>
    {
        let sp = self.register_file.read(RegisterName::sp)?;
        if sp == 0 {
            return Err(ExecutionError::StackOverflow);
        }

        let val: u16 = match src {
            RegisterName::out => {
                return Err(ExecutionError::CannotCpFrom(src));
            },
            _ => self.register_file.read(src)?
        };

        self.memory[sp as usize] = val.to_ne_bytes()[0];
        self.memory[(sp + 1) as usize] = val.to_ne_bytes()[1];

        self.register_file.write(RegisterName::sp, sp - 2)?;

        Ok(())
    }

    fn write(&mut self, src: RegisterName, addr: RegisterName)
        -> Result<(), ExecutionError>
    {
        let val: u16 = match src {
            RegisterName::out => {
                return Err(ExecutionError::CannotCpFrom(src));
            },
            _ => self.register_file.read(src)?
        };

        let addr: u16 = match addr {
            RegisterName::out => {
                return Err(ExecutionError::CannotCpFrom(addr));
            },
            _ => self.register_file.read(addr)?
        };

        self.memory[addr as usize] = val.to_ne_bytes()[0];
        self.memory[(addr + 1) as usize] = val.to_ne_bytes()[1];
        Ok(())
    }

    fn read(&mut self, addr: RegisterName, dst: RegisterName)
        -> Result<(), ExecutionError>
    {
        let addr: u16 = match addr {
            RegisterName::out => {
                return Err(ExecutionError::CannotCpFrom(addr));
            },
            _ => self.register_file.read(addr)?
        };

        let mut val: [u8; 2] = [0; 2];
        val[0] = self.memory[addr as usize];
        val[1] = self.memory[(addr + 1) as usize];
        let val = u16::from_ne_bytes(val);
        self.put(val, dst)
    }

    pub fn halt(&mut self) -> Result<(), ExecutionError> {
        self.power = false;
        Ok(())
    }

    pub fn execute_single_instruction(
        &mut self,
        devices: &mut [&mut dyn Device],
    ) -> Result<bool, ExecutionError> {
        if ! self.power {
            return Ok(false);
        }

        let mut instr: [u8; 4] = [0; 4];
        let pc = self.register_file.read(RegisterName::pc)?;
        instr[0] = self.memory[(pc as usize) + 0];
        instr[1] = self.memory[(pc as usize) + 1];
        instr[2] = self.memory[(pc as usize) + 2];
        instr[3] = self.memory[(pc as usize) + 3];
        let instr = u32::from_ne_bytes(instr);
        let instr = Instruction::try_from_u32(instr)?;
        match instr {
            Instruction::halt => self.halt()?,
            Instruction::add(x, y) => self.add(x, y)?,
            Instruction::copy(src, dst) => self.copy(src, dst, devices)?,
            Instruction::jump(dst, cond) => self.jump(dst, cond)?,
            Instruction::mul(x, y) => self.mul(x, y)?,
            Instruction::noop => (),
            Instruction::pop(dst) => self.pop(dst)?,
            Instruction::push(src) => self.push(src)?,
            Instruction::put(val, dst) => self.put(val, dst)?,
            Instruction::sub(x, y) => self.sub(x, y)?,
            Instruction::write(src, addr) => self.write(src, addr)?,
            Instruction::read(addr, dst) => self.read(addr, dst)?,
        }
        let pc = self.register_file.read(RegisterName::pc)?;
        self.register_file.write(RegisterName::pc, pc + 4)?;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instructions::InstructionName;
    use crate::devices::Buffer;

    #[test]
    fn test_tty() {
        let mut core = Core::new();
        core.write_tty(55);
        assert_eq!(&core.tty, "7");
    }

    #[test]
    fn test_loading() {
        let mut core = Core::new();

        let source = [
            "put 7 gp0",
            "copy ans out"
        ];
        let source = source.join("\n");
        let program = Program::try_compile(&source).unwrap();

        core.load_program(&program).unwrap();

        // coret 7 gp0
        assert_eq!(core.memory[0], InstructionName::put as u8);
        assert_eq!(core.memory[1], 7);
        assert_eq!(core.memory[2], 0);
        assert_eq!(core.memory[3], RegisterName::gp0 as u8);

        // copy ans out
        assert_eq!(core.memory[4], InstructionName::copy as u8);
        assert_eq!(core.memory[5], RegisterName::ans as u8);
        assert_eq!(core.memory[6], RegisterName::out as u8);
        assert_eq!(core.memory[7], 0);
    }

    #[test]
    fn test_memory() {
        let mut core = Core::new();
        assert_eq!(core.memory[100], 0);
        assert_eq!(core.memory[101], 0);

        let source = [
            "put 258 gp0",
            "put 100 gp1",
            "write gp0 gp1",
            "read gp1 gp2",
        ];
        let source = source.join("\n");
        let program = Program::try_compile(&source).unwrap();
        core.load_program(&program).unwrap();

        let mut _lcd0 = Buffer(vec![]);
        let mut _lcd1 = Buffer(vec![]);
        let mut devices: Vec<&mut dyn Device> = vec![
            &mut _lcd0,
            &mut _lcd1
        ];
        core.execute_single_instruction(&mut devices).unwrap();
        core.execute_single_instruction(&mut devices).unwrap();
        core.execute_single_instruction(&mut devices).unwrap();
        core.execute_single_instruction(&mut devices).unwrap();

        assert_eq!(core.memory[100], 2);
        assert_eq!(core.memory[101], 1);
        assert_eq!(core.register_file.gp2, 258);
    }

    #[test]
    fn test_stack_underflow() {
        let mut core = Core::new();
        let error = core.pop(RegisterName::gp0);
        assert_eq!(error, Err(ExecutionError::StackUnderflow));
    }

    #[test]
    fn test_stack_overflow() {
        let mut core = Core::new();
        core.register_file.write(RegisterName::sp, 0).unwrap();
        let error = core.push(RegisterName::gp0);
        assert_eq!(error, Err(ExecutionError::StackOverflow));
    }

    #[test]
    fn test_stack() {
        let mut core = Core::new();
        core.put(7, RegisterName::gp0).unwrap();
        core.push(RegisterName::gp0).unwrap();
        core.put(14, RegisterName::gp0).unwrap();
        core.push(RegisterName::gp0).unwrap();
        core.put(21, RegisterName::gp0).unwrap();
        core.push(RegisterName::gp0).unwrap();

        core.pop(RegisterName::gp1).unwrap();
        let gp1 = core.register_file.read(RegisterName::gp1)
            .unwrap();
        assert_eq!(gp1, 21);

        core.pop(RegisterName::gp1).unwrap();
        let gp1 = core.register_file.read(RegisterName::gp1)
            .unwrap();
        assert_eq!(gp1, 14);

        core.pop(RegisterName::gp1).unwrap();
        let gp1 = core.register_file.read(RegisterName::gp1)
            .unwrap();
        assert_eq!(gp1, 7);
    }
}
