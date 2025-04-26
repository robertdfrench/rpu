use std::collections::HashMap;
use std::mem::size_of;

use crate::instructions::Instruction;
use crate::instructions;

pub struct Program {
    instructions: Vec<Instruction>,
    pub source_lines: Vec<String>,
    pub source_addrs: HashMap<u16, usize>,
}

fn skippable(line: &str) -> bool {
    line.starts_with("#") 
        || line.starts_with(";")
        || line.len() == 0
}

fn tokenize(line: &str) -> Vec<String> {
    line.split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug)]
pub enum CompilationError {
    InstructionParseError(instructions::ParseError),
    UndefinedLabel(String),
}


impl From<instructions::ParseError> for CompilationError {
    fn from(other: instructions::ParseError) -> Self {
        Self::InstructionParseError(other)
    }
}

impl Program {
    pub fn try_compile(source: &str) -> Result<Self, CompilationError> {
        let mut instructions = vec![];
        let mut source_lines = vec![];
        let mut source_addrs = HashMap::new();
        let mut labels = HashMap::<String,usize>::new();

        const WIDTH: usize = size_of::<Instruction>();

        let mut estimated_address = 0;
        for line in source.lines() {
            if skippable(line) { continue; }

            let tokens = tokenize(line);
            let final_token = &tokens[tokens.len() - 1];
            if final_token.starts_with(".") {
                if !labels.contains_key(final_token) {
                    labels.insert(
                        final_token.to_string(),
                        estimated_address
                    );
                }
            }
            estimated_address += WIDTH;
        }

        for (n, line) in source.lines().enumerate() {
            let address = instructions.len() * WIDTH;
            source_lines.push(line.to_string());
            if skippable(line) { continue; }

            let mut tokens = tokenize(line);
            for token in tokens.iter_mut() {
                if token.starts_with(".") {
                    let address = labels.get(token).ok_or(
                        CompilationError::UndefinedLabel(token.to_string())
                    )?;
                    *token = format!("{address}");
                }
            }
            let line = tokens.join(" ");
            let instruction = Instruction::try_from_str(&line)?;
            instructions.push(instruction);
            source_addrs.insert(address as u16, n);
        }

        Ok(Self{ instructions, source_lines, source_addrs })
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
            "copy ans out .LABEL",
            "put .LABEL gp1",
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

    #[test]
    fn test_source_lines() {
        let source = [
            "put 7 gp0",
            "# comment",
            "copy ans out"
        ];
        let source = source.join("\n");
        let program = Program::try_compile(&source).unwrap();

        assert_eq!(&program.source_lines[1], "# comment");
    }

    #[test]
    fn test_source_addrs() {
        let source = [
            "put 7 gp0",
            "# comment",
            "copy ans out"
        ];
        let source = source.join("\n");
        let program = Program::try_compile(&source).unwrap();

        assert_eq!(*program.source_addrs.get(&0).unwrap(), 0);
        assert_eq!(*program.source_addrs.get(&4).unwrap(), 2);
        assert_eq!(program.source_addrs.get(&8), None);
    }
}
