mod assembler;
mod data;
pub mod diagnostics;
mod directives;
mod encoder;
mod eval;
mod expr;
mod lexer;
mod operand;
mod output;
mod parser;
mod registers;
mod segments;
mod statement;
mod symbols;
mod types;

use crate::formats::object::ObjectFile;
use assembler::Assembler;
use diagnostics::{Diagnostic, Severity};

pub struct Assembly {
    pub object: Option<ObjectFile>,
    pub diagnostics: Vec<Diagnostic>,
    pub source_lines: usize,
}

impl Assembly {
    pub fn count(&self, severity: Severity) -> usize {
        self.diagnostics.iter().filter(|diagnostic| diagnostic.severity == severity).count()
    }
}

pub fn assemble(source: &str, source_name: &str) -> Assembly {
    let parsed = parser::parse_source(source);
    let source_lines = source.lines().count();
    let mut diagnostics = parsed.diagnostics;
    let mut assembler = Assembler::new(parsed.lines);
    let last_line = source_lines.max(1);
    if let Err(message) = assembler.run() {
        diagnostics.push(Diagnostic::error(last_line, message));
    }
    diagnostics.extend(assembler.diagnostics.iter().cloned());
    if assembler.entry.is_none() {
        diagnostics.push(Diagnostic::warning(last_line, "no entry point given with END"));
    }
    diagnostics.sort_by_key(|diagnostic| diagnostic.line);
    let has_errors = diagnostics.iter().any(|diagnostic| diagnostic.severity == Severity::Error);
    let object = if has_errors {
        None
    } else {
        match assembler.build_object(source_name) {
            Ok(object) => Some(object),
            Err(problems) => {
                diagnostics.extend(problems.into_iter().map(|problem| Diagnostic::error(last_line, problem)));
                None
            }
        }
    };
    Assembly { object, diagnostics, source_lines }
}
