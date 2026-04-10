use crate::instructions::Instruction;
use crate::registers::RegisterName;
use crate::registers::RegisterFile;
use crate::programs::Program;
use crate::programs;
use crate::instructions;
use crate::registers;
use crate::devices::Device;
use crate::devices;

pub const RAM: usize = 256;

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

    /// A read or write would touch a byte at or past the end of
    /// RAM. Holds the offending base address.
    AddressOutOfBounds(u16),

    /// `Computer::run_to_halt` aborted because the program ran for
    /// more steps than the safety limit allows. Almost always means
    /// an infinite loop without a `halt`.
    StepLimitExceeded,

    /// The program selected a `dvc` value that doesn't map to any
    /// device in the device table. Holds the offending dvc value.
    /// Replaces the old "fall back to tty" behavior.
    NoSuchDevice(u16),

    /// A device's `write` or `read` returned an error. Bubbles the
    /// underlying `devices::Error` so the cause is visible in the
    /// error console.
    Device(devices::Error),

    Decode(instructions::DecodeError),

    Access(registers::AccessError)
}

impl From<devices::Error> for ExecutionError {
    fn from(other: devices::Error) -> Self {
        Self::Device(other)
    }
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

    /// 256 bytes of RAM. Small enough that the whole address space
    /// fits on one screen — no scrolling needed at all to see what
    /// the program is doing. Programs that touch addresses past the
    /// end get a clean `AddressOutOfBounds` error.
    pub memory: [u8; RAM],

    /// Is the CPU running? Becomes `false` when 'halt' is
    /// issued.
    pub power: bool
}

#[derive(Debug)]
pub enum BootError {
    ProgramTooBig(usize),
    Compilation(programs::CompilationError)
}

impl std::fmt::Display for BootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootError::ProgramTooBig(size) => {
                write!(f, "program too big ({size} bytes, max {RAM})")
            }
            BootError::Compilation(e) => write!(f, "{e}"),
        }
    }
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
        let memory = [0; RAM];
        let power = true;

        Self { register_file, memory, power }
    }

    pub fn load_program(&mut self, program: &Program)
        -> Result<(),BootError>
    {
        if program.size() >= RAM {
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
                // Indexed dispatch through the device table. Adding
                // a new device is now a one-line change in
                // `Devices::as_slice()` — `core.rs` doesn't need to
                // know how many devices exist or what they are. An
                // unknown `dvc` is a clean execution error instead
                // of the old silent-fallback-to-tty behavior.
                let dvc = self.register_file.dvc as usize;
                let device = devices
                    .get_mut(dvc)
                    .ok_or(ExecutionError::NoSuchDevice(dvc as u16))?;
                device.write(val)?;
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
        if usize::from(sp) == (RAM - 2) {
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

        // Both bytes of the u16 must be in bounds. Compute the upper
        // index in usize so we don't overflow u16 at addr == 0xFFFF.
        let lo = addr as usize;
        let hi = lo + 1;
        if hi >= RAM {
            return Err(ExecutionError::AddressOutOfBounds(addr));
        }
        self.memory[lo] = val.to_ne_bytes()[0];
        self.memory[hi] = val.to_ne_bytes()[1];
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

        let lo = addr as usize;
        let hi = lo + 1;
        if hi >= RAM {
            return Err(ExecutionError::AddressOutOfBounds(addr));
        }
        let val = u16::from_ne_bytes([self.memory[lo], self.memory[hi]]);
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

        // Fetch the next 4-byte instruction. The fetch must fully fit
        // within RAM; pc == RAM - 3 would touch byte RAM (out of
        // range), so the maximum legal pc is RAM - 4.
        let pc = self.register_file.read(RegisterName::pc)?;
        let pc_lo = pc as usize;
        if pc_lo + 4 > RAM {
            return Err(ExecutionError::AddressOutOfBounds(pc));
        }
        let instr = u32::from_ne_bytes([
            self.memory[pc_lo],
            self.memory[pc_lo + 1],
            self.memory[pc_lo + 2],
            self.memory[pc_lo + 3],
        ]);
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

        // Advance pc. Done in usize so the +4 can't overflow u16 at
        // the top of memory; if the next pc would land at or past
        // RAM, the CPU has walked off the end and we auto-halt.
        let pc = self.register_file.read(RegisterName::pc)? as usize;
        let next = pc + 4;
        if next >= RAM {
            self.power = false;
        } else {
            self.register_file.write(RegisterName::pc, next as u16)?;
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Behavioral / end-to-end tests of program loading and
    // execution have moved to `tests/cpu.rs`, where they go through
    // the `Computer` API. The tests left here exercise `Core`
    // primitives directly — stack underflow / overflow guards and
    // raw push/pop semantics — because they're unit-level
    // invariants of the CPU itself.

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
