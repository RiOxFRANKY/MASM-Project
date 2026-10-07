use super::expr::Expr;
use super::segments::{GroupId, SegmentId};
use super::types::TypeKind;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum SymbolKind {
    Label { segment: SegmentId, offset: u32, kind: TypeKind, count: u32 },
    Constant(i64),
    Alias(Expr),
    Segment(SegmentId),
    Group(GroupId),
}

#[derive(Default)]
pub struct SymbolTable {
    pub entries: HashMap<String, SymbolKind>,
    defined: HashMap<String, (u32, bool)>,
}

impl SymbolTable {
    pub fn get(&self, name: &str) -> Option<&SymbolKind> {
        self.entries.get(name)
    }

    pub fn defined_in(&self, name: &str, pass: u32) -> bool {
        self.defined.get(name).is_some_and(|(defined_pass, _)| *defined_pass == pass)
    }

    pub fn define(&mut self, name: &str, kind: SymbolKind, pass: u32, redefinable: bool) -> Result<(), String> {
        if let Some((defined_pass, was_redefinable)) = self.defined.get(name) {
            if *defined_pass == pass && !(redefinable && *was_redefinable) {
                return Err(format!("symbol redefinition: {name}"));
            }
        }
        self.entries.insert(name.to_string(), kind);
        self.defined.insert(name.to_string(), (pass, redefinable));
        Ok(())
    }

    pub fn snapshot(&self) -> HashMap<String, SymbolKind> {
        self.entries.clone()
    }

    pub fn labels(&self) -> Vec<(String, SegmentId, u32)> {
        let mut labels: Vec<(String, SegmentId, u32)> = self
            .entries
            .iter()
            .filter_map(|(name, kind)| match kind {
                SymbolKind::Label { segment, offset, .. } if !name.starts_with('@') => Some((name.clone(), *segment, *offset)),
                _ => None,
            })
            .collect();
        labels.sort_by(|left, right| (left.1, left.2, &left.0).cmp(&(right.1, right.2, &right.0)));
        labels
    }
}
