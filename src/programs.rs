use std::collections::HashMap;
use std::mem::size_of;

use miette::{Diagnostic, NamedSource, SourceSpan};

use crate::instructions::Instruction;
use crate::instructions;

pub struct Program {
    instructions: Vec<Instruction>,
    pub source_lines: Vec<String>,
    pub source_addrs: HashMap<u16, usize>,
    /// Label name (with leading `.`) to byte address. Populated
    /// during compile so the TUI can resolve labels for jump-to and
    /// the decoded sidebar in stage 5.
    pub labels: HashMap<String, u16>,
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

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub struct CompilationError {
    #[source_code]
    src: NamedSource<String>,

    #[label("{kind}")]
    bad: SourceSpan,

    kind: CompilationErrorKind,
}

impl std::fmt::Display for CompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for CompilationError {}

#[derive(Debug)]
pub enum CompilationErrorKind {
    InstructionParseError(instructions::ParseError),
    UndefinedLabel(String),
}

impl std::fmt::Display for CompilationErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilationErrorKind::InstructionParseError(e) => write!(f, "{e}"),
            CompilationErrorKind::UndefinedLabel(label) => {
                write!(f, "undefined label `{label}`")
            }
        }
    }
}

/// Find the byte offset of `token` within `line`, searching from the
/// start. Returns 0 if not found (defensive fallback so the span
/// still points at the line start rather than panicking).
fn token_offset(line: &str, token: &str) -> usize {
    line.find(token).unwrap_or(0)
}

impl Program {
    pub fn try_compile(source: &str) -> Result<Self, CompilationError> {
        Self::try_compile_named("<source>", source)
    }

    pub fn try_compile_named(name: &str, source: &str)
        -> Result<Self, CompilationError>
    {
        let mut instructions = vec![];
        let mut source_lines = vec![];
        let mut source_addrs = HashMap::new();
        let mut labels = HashMap::<String, u16>::new();

        const WIDTH: usize = size_of::<Instruction>();

        let mut estimated_address: u16 = 0;
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
            estimated_address += WIDTH as u16;
        }

        let src_base = source.as_ptr() as usize;
        let make_src = || NamedSource::new(name, source.to_string());

        for (n, line) in source.lines().enumerate() {
            let address = instructions.len() * WIDTH;
            source_lines.push(line.to_string());
            if skippable(line) { continue; }

            let line_offset = line.as_ptr() as usize - src_base;

            let mut tokens = tokenize(line);
            for token in tokens.iter_mut() {
                if token.starts_with(".") {
                    let resolved = labels.get(token.as_str());
                    match resolved {
                        Some(addr) => *token = format!("{addr}"),
                        None => {
                            let off = line_offset + token_offset(line, token);
                            return Err(CompilationError {
                                src: make_src(),
                                bad: SourceSpan::from((off, token.len())),
                                kind: CompilationErrorKind::UndefinedLabel(
                                    token.to_string()
                                ),
                            });
                        }
                    }
                }
            }
            let joined = tokens.join(" ");
            match Instruction::try_from_str(&joined) {
                Ok(instr) => {
                    instructions.push(instr);
                    source_addrs.insert(address as u16, n);
                }
                Err(parse_err) => {
                    let bad_token = parse_err.offending_token();
                    let off = line_offset
                        + token_offset(line, bad_token);
                    return Err(CompilationError {
                        src: make_src(),
                        bad: SourceSpan::from((off, bad_token.len())),
                        kind: CompilationErrorKind::InstructionParseError(
                            parse_err
                        ),
                    });
                }
            }
        }

        Ok(Self{ instructions, source_lines, source_addrs, labels })
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
