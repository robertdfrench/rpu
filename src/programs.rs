use anyhow::Result;

use crate::instructions::Instruction;

pub struct Program {
    instructions: Vec<Instruction>
}

impl Program {
    pub fn try_compile(source: &str) -> Result<Self> {
        let mut instructions = vec![];

        for line in source.lines() {
            let instruction = Instruction::try_from_str(line)?;
            instructions.push(instruction);
        }

        Ok(Self{ instructions })
    }

    pub fn size(&self) -> usize {
        self.instructions.len() * std::mem::size_of::<Instruction>()
    }

    pub fn bytes<'program>(&'program self) -> EachByte<'program> {
        EachByte::new(self)
    }
}

pub struct EachByte<'program> {
    program: &'program Program,
    instruction_number: usize,
    offset: usize
}

impl<'program> EachByte<'program> {
    fn new(program: &'program Program) -> Self {
        let instruction_number = 0;
        let offset = 0;
        Self{ program, instruction_number, offset }
    }
}

impl<'program> Iterator for EachByte<'program> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.instruction_number >= self.program.instructions.len() {
            return None;
        }

        let instr = &self.program.instructions[self.instruction_number];
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

    #[test]
    fn compile_valid_code() {
        let source = [
            "put 7 gp0",
            "put 5 gp1",
            "add gp1 gp0",
            "cp ans out"
        ];
        let source = source.join("\n");

        let program = Program::try_compile(&source).unwrap();

        assert_eq!(program.size(), 16);
    }

    #[test]
    fn test_iterator() {
        let source = [
            "put 7 gp0",
            "cp ans out"
        ];
        let source = source.join("\n");
        let program = Program::try_compile(&source).unwrap();

        let mut results: Vec<u8> = vec![];
        for byte in program.bytes() {
            results.push(byte);
        }

        // put 7 gp0
        assert_eq!(results[0], 1);
        assert_eq!(results[1], 7);
        assert_eq!(results[2], 0);
        assert_eq!(results[3], 0);

        // cp ans out
        assert_eq!(results[4], 3);
        assert_eq!(results[5], 10);
        assert_eq!(results[6], 8);
        assert_eq!(results[7], 0);
    }
}
