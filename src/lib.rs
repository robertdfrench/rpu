mod registers;
mod instructions;
mod programs;

use std::collections::HashMap;
use std::io::Write;
use registers::Register;
use registers::RegisterName;
use programs::Program;

use anyhow::Result;
use thiserror::Error;

pub struct ProcessingUnit<'output, W: Write> {
    register_file: HashMap<RegisterName, Register>,
    memory: [u8; 65_536],
    output: &'output mut W
}

impl<'output, W: Write> ProcessingUnit<'output, W> {
    pub fn new(output: &'output mut W) -> Self {
        let memory = [0; 65_536];
        let mut register_file = HashMap::new();

        // gr0
        register_file.insert(
            RegisterName::gr0,
            Register::new_rw(RegisterName::gr0)
        );

        // gr1
        register_file.insert(
            RegisterName::gr1,
            Register::new_rw(RegisterName::gr1)
        );

        // gr2
        register_file.insert(
            RegisterName::gr2,
            Register::new_rw(RegisterName::gr2)
        );

        // gr3
        register_file.insert(
            RegisterName::gr3,
            Register::new_rw(RegisterName::gr3)
        );

        // gr4
        register_file.insert(
            RegisterName::gr4,
            Register::new_rw(RegisterName::gr4)
        );

        // gr5
        register_file.insert(
            RegisterName::gr5,
            Register::new_rw(RegisterName::gr5)
        );

        // gr6
        register_file.insert(
            RegisterName::gr6,
            Register::new_rw(RegisterName::gr6)
        );

        // gr7
        register_file.insert(
            RegisterName::gr7,
            Register::new_rw(RegisterName::gr7)
        );

        // pc
        register_file.insert(
            RegisterName::pc,
            Register::new_ro(RegisterName::pc)
        );

        // out
        register_file.insert(
            RegisterName::out,
            Register::new_wo(RegisterName::out)
        );

        // srA
        register_file.insert(
            RegisterName::srA,
            Register::new_ro(RegisterName::srA)
        );
        Self { register_file, memory, output }
    }

    fn write_output(&mut self, byte: u16) -> Result<()> {
        match writeln!(self.output, "{byte}") {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into())
        }
    }

    fn load_program(&mut self, program: Program) {
        for (i, byte) in program.bytes().enumerate() {
            self.memory[i] = byte;
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

        let gr0 = pu.register_file.get(&RegisterName::gr0).unwrap();
        assert_eq!(gr0.name, RegisterName::gr0);
        assert_eq!(gr0.readable, true);
        assert_eq!(gr0.writeable, true);

        let gr1 = pu.register_file.get(&RegisterName::gr1).unwrap();
        assert_eq!(gr1.name, RegisterName::gr1);
        assert_eq!(gr1.readable, true);
        assert_eq!(gr1.writeable, true);

        let gr2 = pu.register_file.get(&RegisterName::gr2).unwrap();
        assert_eq!(gr2.name, RegisterName::gr2);
        assert_eq!(gr2.readable, true);
        assert_eq!(gr2.writeable, true);

        let gr3 = pu.register_file.get(&RegisterName::gr3).unwrap();
        assert_eq!(gr3.name, RegisterName::gr3);
        assert_eq!(gr3.readable, true);
        assert_eq!(gr3.writeable, true);

        let gr4 = pu.register_file.get(&RegisterName::gr4).unwrap();
        assert_eq!(gr4.name, RegisterName::gr4);
        assert_eq!(gr4.readable, true);
        assert_eq!(gr4.writeable, true);

        let gr5 = pu.register_file.get(&RegisterName::gr5).unwrap();
        assert_eq!(gr5.name, RegisterName::gr5);
        assert_eq!(gr5.readable, true);
        assert_eq!(gr5.writeable, true);

        let gr6 = pu.register_file.get(&RegisterName::gr6).unwrap();
        assert_eq!(gr6.name, RegisterName::gr6);
        assert_eq!(gr6.readable, true);
        assert_eq!(gr6.writeable, true);

        let gr7 = pu.register_file.get(&RegisterName::gr7).unwrap();
        assert_eq!(gr7.name, RegisterName::gr7);
        assert_eq!(gr7.readable, true);
        assert_eq!(gr7.writeable, true);

        let pc = pu.register_file.get(&RegisterName::pc).unwrap();
        assert_eq!(pc.name, RegisterName::pc);
        assert_eq!(pc.readable, true);
        assert_eq!(pc.writeable, false);

        let out = pu.register_file.get(&RegisterName::out).unwrap();
        assert_eq!(out.name, RegisterName::out);
        assert_eq!(out.readable, false);
        assert_eq!(out.writeable, true);

        #[allow(non_snake_case)]
        let srA = pu.register_file.get(&RegisterName::srA).unwrap();
        assert_eq!(srA.name, RegisterName::srA);
        assert_eq!(srA.readable, true);
        assert_eq!(srA.writeable, false);
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
            "put 7 gr0",
            "cp srA out"
        ];
        let source = source.join("\n");
        let program = Program::try_compile(&source).unwrap();

        pu.load_program(program);

        // put 7 gr0
        assert_eq!(pu.memory[0], 1);
        assert_eq!(pu.memory[1], 7);
        assert_eq!(pu.memory[2], 0);
        assert_eq!(pu.memory[3], 0);

        // cp srA out
        assert_eq!(pu.memory[4], 3);
        assert_eq!(pu.memory[5], 10);
        assert_eq!(pu.memory[6], 8);
        assert_eq!(pu.memory[7], 0);
    }
}
