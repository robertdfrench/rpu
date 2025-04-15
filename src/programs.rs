use anyhow::Result;

use crate::instructions::Instruction;

pub struct Program {
    instructions: Vec<Instruction>
}

impl Program {
    pub fn try_compile(source: &str) -> Result<Self> {
        let mut instructions = vec![];

        for line in source.lines() {
            if line.starts_with("#") { continue; }
            if line.len() == 0 { continue; }
            let instruction = Instruction::try_from_str(line)?;
            instructions.push(instruction);
        }

        Ok(Self{ instructions })
    }

    pub fn size(&self) -> usize {
        let width = std::mem::size_of::<Instruction>();
        self.instructions.len() * width
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
            "cp ans out"
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
            "cp ans out"
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
        assert_eq!(results[0], InstructionName::put as u8);
        assert_eq!(results[1], 7);
        assert_eq!(results[2], 0);
        assert_eq!(results[3], RegisterName::gp0 as u8);

        // cp ans out
        assert_eq!(results[4], InstructionName::cp as u8);
        assert_eq!(results[5], RegisterName::ans as u8);
        assert_eq!(results[6], RegisterName::out as u8);
        assert_eq!(results[7], 0);
    }
}
