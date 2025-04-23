use anyhow::Result;
use std::collections::HashMap;
use std::mem::size_of;

use crate::instructions::Instruction;

pub struct Program {
    instructions: Vec<Instruction>
}

impl Program {
    pub fn try_compile(source: &str) -> Result<Self> {
        let mut instructions = vec![];
        let mut labels = HashMap::<String,usize>::new();

        for line in source.lines() {
            if line.starts_with("#") { continue; }
            if line.len() == 0 { continue; }
            let mut parts: Vec<String> = line
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            for i in 0..(parts.len() - 1) {
                if parts[i].starts_with(":") {
                    // This is a :LABEL
                    // It needs to be replaced
                    let address = labels.get(&parts[i])
                        .unwrap();
                    parts[i] = format!("{address}");
                }
            }
            if parts.len() == 4 {
                let label = parts[3].clone();
                if label.starts_with(":") {
                    let width = size_of::<Instruction>();
                    let address = instructions.len() * width;
                    labels.insert(label, address);
                }
            }
            let line = parts.join(" ");
            let instruction = Instruction::try_from_str(&line)?;
            instructions.push(instruction);
        }

        Ok(Self{ instructions })
    }

    pub fn size(&self) -> usize {
        self.instructions.len() * size_of::<Instruction>()
    }

    pub fn bytes<'p>(&'p self) -> EachByte<'p> {
        EachByte::new(self)
    }
}

pub struct EachByte<'p> {
    program: &'p Program,
    instruction_number: usize,
    offset: usize
}

impl<'p> EachByte<'p> {
    fn new(program: &'p Program) -> Self {
        let instruction_number = 0;
        let offset = 0;
        Self{ program, instruction_number, offset }
    }
}

impl<'p> Iterator for EachByte<'p> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let program_end = self.program.instructions.len();
        if self.instruction_number >= program_end {
            return None;
        }

        let instr = &self.program.instructions[
            self.instruction_number
        ];
        let bytes = instr.to_u32().to_ne_bytes();
        let result = bytes[self.offset];
        self.offset += 1;

        if self.offset >= 4 {
            self.offset = 0;
            self.instruction_number += 1;
        }

        return Some(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::RegisterName;
    use crate::instructions::InstructionName;

    #[test]
    fn compile_valid_code() {
        let source = [
            "put 7 gp0",
            "put 5 gp1",
            "add gp1 gp0",
            "copy ans out"
        ];
        let source = source.join("\n");

        let program = Program::try_compile(&source).unwrap();

        assert_eq!(program.size(), 16);
    }

    #[test]
    fn compile_code_with_comments() {
        let source = [
            "put 7 gp0",
            "put 5 gp1",
            "# add the values",
            "add gp1 gp0",
            "copy ans out"
        ];
        let source = source.join("\n");

        let program = Program::try_compile(&source).unwrap();

        assert_eq!(program.size(), 16);
    }

    #[test]
    fn compile_code_with_blank_lines() {
        let source = [
            "put 7 gp0",
            "put 5 gp1",
            "",
            "add gp1 gp0",
            "copy ans out"
        ];
        let source = source.join("\n");

        let program = Program::try_compile(&source).unwrap();

        assert_eq!(program.size(), 16);
    }

    #[test]
    fn test_address_replacement() {
        let source = [
            "put 7 gp0",
            "copy ans out :LABEL",
            "put :LABEL gp1",
        ];
        let source = source.join("\n");
        let program = Program::try_compile(&source).unwrap();

        let mut memory: Vec<u8> = vec![];
        for byte in program.bytes() {
            memory.push(byte);
        }

        // put 7 gp0
        assert_eq!(memory[0], InstructionName::put as u8);
        assert_eq!(memory[1], 7);
        assert_eq!(memory[2], 0);
        assert_eq!(memory[3], RegisterName::gp0 as u8);

        // copy ans out
        assert_eq!(memory[4], InstructionName::copy as u8);
        assert_eq!(memory[5], RegisterName::ans as u8);
        assert_eq!(memory[6], RegisterName::out as u8);
        assert_eq!(memory[7], 0);

        // put :LABEL(==4) gp1
        assert_eq!(memory[8], InstructionName::put as u8);
        assert_eq!(memory[9], 4);
        assert_eq!(memory[10], 0);
        assert_eq!(memory[11], RegisterName::gp1 as u8);
    }

    #[test]
    fn test_iterator() {
        let source = [
            "put 7 gp0",
            "copy ans out"
        ];
        let source = source.join("\n");
        let program = Program::try_compile(&source).unwrap();

        let mut memory: Vec<u8> = vec![];
        for byte in program.bytes() {
            memory.push(byte);
        }

        // put 7 gp0
        assert_eq!(memory[0], InstructionName::put as u8);
        assert_eq!(memory[1], 7);
        assert_eq!(memory[2], 0);
        assert_eq!(memory[3], RegisterName::gp0 as u8);

        // copy ans out
        assert_eq!(memory[4], InstructionName::copy as u8);
        assert_eq!(memory[5], RegisterName::ans as u8);
        assert_eq!(memory[6], RegisterName::out as u8);
        assert_eq!(memory[7], 0);
    }
}
