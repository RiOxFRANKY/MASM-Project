use super::assembler::Assembler;
use super::expr::Expr;
use super::statement::{Model, SimpleSegment};
use super::symbols::SymbolKind;
use super::types::TypeKind;

const DEFAULT_STACK: i64 = 1024;
const FULL_SEGMENT_RANK: u32 = 100;

impl Assembler {
    fn ensure_segment_symbol(&mut self, name: &str, id: usize) -> Result<(), String> {
        if self.symbols.defined_in(name, self.pass) {
            return Ok(());
        }
        self.define(name, SymbolKind::Segment(id), false)
    }

    fn ensure_group(&mut self, name: &str) -> Result<usize, String> {
        let group = self.segments.group(name);
        if !self.symbols.defined_in(name, self.pass) {
            self.define(name, SymbolKind::Group(group), false)?;
        }
        Ok(group)
    }

    pub fn open_segment(&mut self, name: &str, align: u32, stack: bool, class: &str) -> Result<(), String> {
        let rank = FULL_SEGMENT_RANK + self.segments.list.len() as u32;
        let id = self.segments.open(name, class, align, stack, rank);
        let segment = &mut self.segments.list[id];
        if !class.is_empty() {
            segment.class = class.to_string();
        }
        segment.align = align;
        segment.stack |= stack;
        self.ensure_segment_symbol(name, id)?;
        self.segment_stack.push(self.current);
        self.current = Some(id);
        Ok(())
    }

    pub fn close_segment(&mut self, name: &str) -> Result<(), String> {
        let Some(current) = self.current else {
            return Err("ENDS without an open segment".to_string());
        };
        if !name.is_empty() && self.segments.list[current].name != name {
            return Err(format!("segment {name} is not the open segment"));
        }
        if self.segment_stack.is_empty() {
            return Err("ENDS cannot close a .CODE/.DATA segment".to_string());
        }
        self.current = self.segment_stack.pop().flatten();
        Ok(())
    }

    pub fn group(&mut self, name: &str, members: &[String]) -> Result<(), String> {
        let group = self.ensure_group(name)?;
        for member in members {
            let id = match self.segments.find(member) {
                Some(id) => id,
                None => {
                    let rank = FULL_SEGMENT_RANK + self.segments.list.len() as u32;
                    let id = self.segments.open(member, "", 16, false, rank);
                    self.ensure_segment_symbol(member, id)?;
                    id
                }
            };
            self.segments.join(id, group)?;
        }
        Ok(())
    }

    pub fn open_proc(&mut self, name: &str, distance: Option<TypeKind>) -> Result<(), String> {
        let far = match distance {
            Some(kind) => kind == TypeKind::Far,
            None => self.model.is_some_and(Model::far_code),
        };
        self.define_label(name, if far { TypeKind::Far } else { TypeKind::Near }, 0)?;
        self.procs.push((name.to_string(), far));
        Ok(())
    }

    pub fn close_proc(&mut self, name: &str) -> Result<(), String> {
        match self.procs.pop() {
            Some((open, _)) if open == name => Ok(()),
            Some((open, far)) => {
                self.procs.push((open.clone(), far));
                Err(format!("ENDP for {name} does not match open procedure {open}"))
            }
            None => Err(format!("ENDP for {name} without PROC")),
        }
    }

    pub fn set_model(&mut self, model: Model) -> Result<(), String> {
        self.model = Some(model);
        self.ensure_group("DGROUP")?;
        Ok(())
    }

    fn simple_segment_id(&mut self, kind: SimpleSegment) -> Result<usize, String> {
        let model = self.model.ok_or("simplified segment directives need .MODEL first")?;
        let (name, class, rank) = match kind {
            SimpleSegment::Code => ("_TEXT", "CODE", 0),
            SimpleSegment::Data => ("_DATA", "DATA", 1),
            SimpleSegment::Const => ("CONST", "CONST", 2),
            SimpleSegment::Bss => ("_BSS", "BSS", 3),
            SimpleSegment::Stack => ("STACK", "STACK", 4),
        };
        let id = self.segments.open(name, class, 16, kind == SimpleSegment::Stack, rank);
        self.ensure_segment_symbol(name, id)?;
        if kind != SimpleSegment::Code || model == Model::Tiny {
            let group = self.ensure_group("DGROUP")?;
            self.segments.join(id, group)?;
        }
        Ok(id)
    }

    pub fn simple_segment(&mut self, kind: SimpleSegment) -> Result<(), String> {
        if !self.segment_stack.is_empty() {
            return Err("close the open SEGMENT with ENDS before using .CODE or .DATA".to_string());
        }
        let id = self.simple_segment_id(kind)?;
        self.current = Some(id);
        Ok(())
    }

    pub fn stack(&mut self, size: Option<&Expr>) -> Result<(), String> {
        let bytes = match size {
            Some(expression) => {
                let value = self.evaluate(expression)?;
                if !value.is_constant() {
                    return Err(".STACK needs a constant size".to_string());
                }
                value.number
            }
            None => DEFAULT_STACK,
        };
        if !(1..=0xFFFF).contains(&bytes) {
            return Err("invalid stack size".to_string());
        }
        let id = self.simple_segment_id(SimpleSegment::Stack)?;
        self.segments.reserve(id, bytes as u32, 0);
        Ok(())
    }

    pub fn org(&mut self, expression: &Expr) -> Result<(), String> {
        let (segment, _) = self.location()?;
        let value = self.evaluate(expression)?;
        let target = match value.label {
            Some(label) if label.segment == segment => {
                value.number - self.layout.frame_offset(&self.segments, segment, 0)
            }
            Some(_) => return Err("ORG target must be in the current segment".to_string()),
            None if value.is_constant() => value.number,
            None => return Err("ORG needs a constant".to_string()),
        };
        if !(0..=0xFFFF).contains(&target) {
            return Err("ORG value out of range".to_string());
        }
        let segment_data = &mut self.segments.list[segment];
        segment_data.location = target as u32;
        segment_data.size = segment_data.size.max(segment_data.location);
        if segment_data.data.len() < target as usize {
            segment_data.data.resize(target as usize, 0);
        }
        Ok(())
    }

    pub fn align(&mut self, boundary: u32) -> Result<(), String> {
        let (segment, location) = self.location()?;
        let fill = if self.segments.list[segment].class == "CODE" { 0x90 } else { 0 };
        let padding = (boundary - location % boundary) % boundary;
        self.segments.reserve(segment, padding, fill);
        Ok(())
    }
}
