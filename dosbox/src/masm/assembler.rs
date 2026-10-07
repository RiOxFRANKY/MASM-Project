use super::diagnostics::Diagnostic;
use super::encoder::{self, Context};
use super::eval::{Evaluator, Value};
use super::expr::Expr;
use super::segments::{Layout, SegmentId, Segments};
use super::statement::{Line, Model, Stmt};
use super::symbols::{SymbolKind, SymbolTable};
use super::types::TypeKind;

const MAX_PASSES: u32 = 30;
const NOP: u8 = 0x90;

pub struct Assembler {
    pub lines: Vec<Line>,
    pub symbols: SymbolTable,
    pub min_lengths: Vec<usize>,
    pub layout: Layout,
    pub pass: u32,
    pub segments: Segments,
    pub current: Option<SegmentId>,
    pub segment_stack: Vec<Option<SegmentId>>,
    pub procs: Vec<(String, bool)>,
    pub model: Option<Model>,
    pub entry: Option<Value>,
    pub diagnostics: Vec<Diagnostic>,
    missing: Vec<String>,
}

impl Assembler {
    pub fn new(lines: Vec<Line>) -> Self {
        let count = lines.len();
        Assembler {
            lines,
            symbols: SymbolTable::default(),
            min_lengths: vec![0; count],
            layout: Layout::default(),
            pass: 0,
            segments: Segments::default(),
            current: None,
            segment_stack: Vec::new(),
            procs: Vec::new(),
            model: None,
            entry: None,
            diagnostics: Vec::new(),
            missing: Vec::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut previous = None;
        for pass in 1..=MAX_PASSES {
            self.run_pass(pass);
            let snapshot = self.symbols.snapshot();
            let layout = self.segments.layout();
            let stable = previous.as_ref() == Some(&snapshot) && layout == self.layout;
            self.layout = layout;
            previous = Some(snapshot);
            if stable {
                return Ok(());
            }
        }
        Err("phase error: symbol addresses did not settle".to_string())
    }

    fn run_pass(&mut self, pass: u32) {
        self.pass = pass;
        self.segments = Segments::default();
        self.current = None;
        self.segment_stack.clear();
        self.procs.clear();
        self.model = None;
        self.entry = None;
        self.diagnostics.clear();
        for index in 0..self.lines.len() {
            let line = self.lines[index].clone();
            self.missing.clear();
            if let Err(message) = self.statement(index, &line.stmt) {
                self.diagnostics.push(Diagnostic::error(line.number, message));
            }
            let mut missing = std::mem::take(&mut self.missing);
            missing.sort();
            missing.dedup();
            for name in missing {
                self.diagnostics.push(Diagnostic::error(line.number, format!("undefined symbol: {name}")));
            }
        }
        let last_line = self.lines.last().map(|line| line.number).unwrap_or(0);
        if let Some((name, _)) = self.procs.last() {
            self.diagnostics.push(Diagnostic::error(last_line, format!("procedure {name} is not closed with ENDP")));
        }
        if let Some(Some(segment)) = self.segment_stack.last() {
            let name = self.segments.list[*segment].name.clone();
            self.diagnostics.push(Diagnostic::error(last_line, format!("segment {name} is not closed with ENDS")));
        }
    }

    pub fn evaluate(&mut self, expression: &Expr) -> Result<Value, String> {
        let here = self.current.map(|segment| (segment, self.segments.list[segment].location));
        let mut evaluator = Evaluator {
            symbols: &self.symbols,
            segments: &self.segments,
            layout: &self.layout,
            here,
            missing: Vec::new(),
        };
        let result = evaluator.evaluate(expression);
        self.missing.extend(evaluator.missing);
        result
    }

    pub fn require_segment(&self) -> Result<SegmentId, String> {
        self.current.ok_or_else(|| "code or data outside of a segment".to_string())
    }

    pub fn location(&self) -> Result<(SegmentId, u32), String> {
        let segment = self.require_segment()?;
        Ok((segment, self.segments.list[segment].location))
    }

    pub fn define(&mut self, name: &str, kind: SymbolKind, redefinable: bool) -> Result<(), String> {
        if super::registers::lookup(name).is_some() {
            return Err(format!("reserved word used as a name: {name}"));
        }
        self.symbols.define(name, kind, self.pass, redefinable)
    }

    pub fn define_label(&mut self, name: &str, kind: TypeKind, count: u32) -> Result<(), String> {
        let (segment, offset) = self.location()?;
        self.define(name, SymbolKind::Label { segment, offset, kind, count }, false)
    }

    fn statement(&mut self, index: usize, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Label(name) => self.define_label(name, TypeKind::Near, 0),
            Stmt::Instruction { prefixes, mnemonic, operands } => self.instruction(index, prefixes, mnemonic, operands),
            Stmt::Data { name, kind, items } => self.data(name.as_deref(), *kind, items),
            Stmt::Equ { name, value } => self.equ(name, value),
            Stmt::Assign { name, value } => {
                let result = self.evaluate(value)?;
                if !result.is_constant() && !result.unknown {
                    return Err("'=' needs a constant expression".to_string());
                }
                self.define(name, SymbolKind::Constant(result.number), true)
            }
            Stmt::Segment { name, align, stack, class } => self.open_segment(name, *align, *stack, class),
            Stmt::Ends(name) => self.close_segment(name),
            Stmt::Group { name, members } => self.group(name, members),
            Stmt::Proc { name, distance } => self.open_proc(name, *distance),
            Stmt::Endp(name) => self.close_proc(name),
            Stmt::LabelDirective { name, kind } => self.define_label(name, *kind, 1),
            Stmt::Model(model) => self.set_model(*model),
            Stmt::Stack(size) => self.stack(size.as_ref()),
            Stmt::Simple(kind) => self.simple_segment(*kind),
            Stmt::Org(expression) => self.org(expression),
            Stmt::Align(boundary) => self.align(*boundary),
            Stmt::End(expression) => {
                if let Some(expression) = expression {
                    self.entry = Some(self.evaluate(expression)?);
                } else if self.symbols.get("@STARTUP").is_some() {
                    self.entry = Some(self.evaluate(&Expr::Symbol("@STARTUP".to_string()))?);
                }
                Ok(())
            }
        }
    }

    fn instruction(&mut self, index: usize, prefixes: &[u8], mnemonic: &str, operands: &[Expr]) -> Result<(), String> {
        let (segment, location) = self.location()?;
        let mut values = Vec::with_capacity(operands.len());
        let mut failure = None;
        for operand in operands {
            match self.evaluate(operand) {
                Ok(value) => values.push(value),
                Err(message) => {
                    failure = Some(message);
                    break;
                }
            }
        }
        let context = Context {
            here: self.layout.frame_offset(&self.segments, segment, location),
            frame: self.segments.frame_of(segment),
            far_proc: self.procs.last().is_some_and(|(_, far)| *far),
            min_length: self.min_lengths[index],
        };
        let encoded = match failure {
            Some(message) => Err(message),
            None => encoder::encode(prefixes, mnemonic, &values, &context),
        };
        let (mut bytes, relocations) = match encoded {
            Ok(encoded) => {
                let number = self.lines[index].number;
                for warning in encoded.warnings {
                    self.diagnostics.push(Diagnostic::warning(number, warning));
                }
                (encoded.bytes, encoded.relocations)
            }
            Err(message) => {
                self.segments.reserve(segment, self.min_lengths[index] as u32, NOP);
                return Err(message);
            }
        };
        if bytes.len() < self.min_lengths[index] {
            bytes.resize(self.min_lengths[index], NOP);
        }
        self.min_lengths[index] = bytes.len();
        for (position, frame) in relocations {
            self.segments.list[segment].relocations.push((location + position as u32, frame));
        }
        self.segments.emit(segment, &bytes);
        Ok(())
    }

    fn equ(&mut self, name: &str, expression: &Expr) -> Result<(), String> {
        let value = self.evaluate(expression);
        let kind = match value {
            Ok(value) if value.is_constant() && !value.unknown => SymbolKind::Constant(value.number),
            Ok(value)
                if value.register.is_none()
                    && value.base.is_none()
                    && value.index.is_none()
                    && value.frame.is_none()
                    && !value.explicit =>
            {
                match value.label {
                    Some(label) => {
                        let start = self.layout.frame_offset(&self.segments, label.segment, 0);
                        SymbolKind::Label {
                            segment: label.segment,
                            offset: (value.number - start).max(0) as u32,
                            kind: value.kind.unwrap_or(TypeKind::Near),
                            count: value.count,
                        }
                    }
                    None => SymbolKind::Alias(expression.clone()),
                }
            }
            _ => SymbolKind::Alias(expression.clone()),
        };
        self.define(name, kind, false)
    }
}
