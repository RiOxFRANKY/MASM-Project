use super::expr::{Binary, Expr, Unary};
use super::registers::{Register, BP, BX, DI, SI};
use super::segments::{Frame, Layout, SegmentId, Segments};
use super::symbols::{SymbolKind, SymbolTable};
use super::types::TypeKind;

const MAX_DEPTH: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelRef {
    pub segment: SegmentId,
    pub frame: Frame,
    pub paragraph: u16,
}

#[derive(Clone, Debug, Default)]
pub struct Value {
    pub number: i64,
    pub register: Option<Register>,
    pub base: Option<u8>,
    pub index: Option<u8>,
    pub memory: bool,
    pub bracketed: bool,
    pub label: Option<LabelRef>,
    pub frame: Option<(Frame, u16)>,
    pub kind: Option<TypeKind>,
    pub explicit: bool,
    pub segment_override: Option<u8>,
    pub unknown: bool,
    pub short: bool,
    pub count: u32,
}

impl Value {
    pub fn constant(number: i64) -> Self {
        Value { number, ..Value::default() }
    }

    pub fn is_constant(&self) -> bool {
        self.register.is_none()
            && self.base.is_none()
            && self.index.is_none()
            && self.label.is_none()
            && self.frame.is_none()
            && !self.memory
    }
}

pub struct Evaluator<'a> {
    pub symbols: &'a SymbolTable,
    pub segments: &'a Segments,
    pub layout: &'a Layout,
    pub here: Option<(SegmentId, u32)>,
    pub missing: Vec<String>,
}

impl<'a> Evaluator<'a> {
    pub fn evaluate(&mut self, expression: &Expr) -> Result<Value, String> {
        self.eval(expression, 0)
    }

    fn label_value(&self, segment: SegmentId, offset: u32) -> Value {
        let frame = self.segments.frame_of(segment);
        Value {
            number: self.layout.frame_offset(self.segments, segment, offset),
            label: Some(LabelRef { segment, frame, paragraph: self.layout.paragraph(frame) }),
            ..Value::default()
        }
    }

    fn frame_value(&self, frame: Frame) -> Value {
        let paragraph = self.layout.paragraph(frame);
        Value { number: paragraph as i64, frame: Some((frame, paragraph)), ..Value::default() }
    }

    fn symbol(&mut self, name: &str, depth: u32) -> Result<Value, String> {
        let kind = match self.symbols.get(name) {
            Some(kind) => kind.clone(),
            None => {
                let alias = match name {
                    "@DATA" | "@STACK" | "DGROUP" => self.segments.find_group("DGROUP").map(SymbolKind::Group),
                    "@CODE" => self.segments.find("_TEXT").map(SymbolKind::Segment),
                    _ => None,
                };
                match alias {
                    Some(kind) => kind,
                    None => {
                        self.missing.push(name.to_string());
                        return Ok(Value { unknown: true, ..Value::default() });
                    }
                }
            }
        };
        Ok(match kind {
            SymbolKind::Label { segment, offset, kind, count } => {
                let mut value = self.label_value(segment, offset);
                value.kind = Some(kind);
                value.memory = !kind.is_code();
                value.count = count;
                value
            }
            SymbolKind::Constant(number) => Value::constant(number),
            SymbolKind::Alias(expression) => self.eval(&expression, depth + 1)?,
            SymbolKind::Segment(segment) => self.frame_value(Frame::Segment(segment)),
            SymbolKind::Group(group) => self.frame_value(Frame::Group(group)),
        })
    }

    fn eval(&mut self, expression: &Expr, depth: u32) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err("expression nested too deeply (circular EQU?)".to_string());
        }
        match expression {
            Expr::Number(number) => Ok(Value::constant(*number)),
            Expr::Str(bytes) => {
                if bytes.len() > 4 {
                    return Err("string constant too long".to_string());
                }
                Ok(Value::constant(bytes.iter().fold(0i64, |total, byte| (total << 8) | *byte as i64)))
            }
            Expr::Symbol(name) => self.symbol(name, depth),
            Expr::Register(register) => Ok(Value { register: Some(*register), ..Value::default() }),
            Expr::Here => match self.here {
                Some((segment, offset)) => {
                    let mut value = self.label_value(segment, offset);
                    value.kind = Some(TypeKind::Near);
                    Ok(value)
                }
                None => Err("$ used outside of a segment".to_string()),
            },
            Expr::Undefined => Err("'?' is only allowed in data definitions".to_string()),
            Expr::Dup(_, _) => Err("DUP is only allowed in data definitions".to_string()),
            Expr::Memory(inner) => {
                let mut value = self.eval(inner, depth + 1)?;
                absorb_register(&mut value)?;
                value.memory = true;
                value.bracketed = true;
                Ok(value)
            }
            Expr::Ptr(kind, inner) => {
                let mut value = self.eval(inner, depth + 1)?;
                value.kind = Some(*kind);
                value.explicit = true;
                Ok(value)
            }
            Expr::Override(segment, inner) => {
                let mut value = self.eval(inner, depth + 1)?;
                absorb_register(&mut value)?;
                value.segment_override = Some(*segment);
                value.memory = true;
                Ok(value)
            }
            Expr::Unary(operator, inner) => {
                let value = self.eval(inner, depth + 1)?;
                unary(*operator, value)
            }
            Expr::Binary(operator, left, right) => {
                let left = self.eval(left, depth + 1)?;
                let right = self.eval(right, depth + 1)?;
                binary(*operator, left, right)
            }
        }
    }
}

fn absorb_register(value: &mut Value) -> Result<(), String> {
    let Some(register) = value.register.take() else {
        return Ok(());
    };
    match register {
        Register::Word(code) if code == BX || code == BP => {
            if value.base.replace(code).is_some() {
                return Err("multiple base registers".to_string());
            }
        }
        Register::Word(code) if code == SI || code == DI => {
            if value.index.replace(code).is_some() {
                return Err("multiple index registers".to_string());
            }
        }
        _ => return Err("only BX, BP, SI and DI can be used for addressing".to_string()),
    }
    Ok(())
}

fn require_constant(value: &Value) -> Result<i64, String> {
    if value.is_constant() {
        Ok(value.number)
    } else {
        Err("constant expected".to_string())
    }
}

fn unary(operator: Unary, value: Value) -> Result<Value, String> {
    let unknown = value.unknown;
    let mut result = match operator {
        Unary::Negate => Value::constant(-require_constant(&value)?),
        Unary::Not => Value::constant(!require_constant(&value)?),
        Unary::High => Value::constant((require_constant(&value)? >> 8) & 0xFF),
        Unary::Low => Value::constant(require_constant(&value)? & 0xFF),
        Unary::Offset => {
            if value.register.is_some() || value.base.is_some() || value.index.is_some() {
                return Err("OFFSET needs a label or variable".to_string());
            }
            let number = if value.frame.is_some() { 0 } else { value.number };
            Value { number, label: value.label, ..Value::default() }
        }
        Unary::Seg => match (value.label, value.frame) {
            (Some(label), _) => {
                Value { number: label.paragraph as i64, frame: Some((label.frame, label.paragraph)), ..Value::default() }
            }
            (None, Some(frame)) => Value { number: frame.1 as i64, frame: Some(frame), ..Value::default() },
            _ if value.unknown => Value::default(),
            _ => return Err("SEG needs a label or variable".to_string()),
        },
        Unary::Short => {
            let mut value = value;
            value.short = true;
            value
        }
        Unary::Type => Value::constant(value.kind.map(TypeKind::type_value).unwrap_or(0)),
        Unary::Length => Value::constant(value.count.max(1) as i64),
        Unary::Size => Value::constant(value.count.max(1) as i64 * value.kind.map(|kind| kind.size() as i64).unwrap_or(1)),
    };
    result.unknown |= unknown;
    Ok(result)
}

fn binary(operator: Binary, mut left: Value, mut right: Value) -> Result<Value, String> {
    let unknown = left.unknown || right.unknown;
    let mut result = match operator {
        Binary::Add => {
            absorb_register(&mut left)?;
            absorb_register(&mut right)?;
            if left.label.is_some() && right.label.is_some() {
                return Err("cannot add two relocatable values".to_string());
            }
            if left.base.is_some() && right.base.is_some() {
                return Err("multiple base registers".to_string());
            }
            if left.index.is_some() && right.index.is_some() {
                return Err("multiple index registers".to_string());
            }
            let kind = if left.explicit {
                left.kind
            } else if right.explicit {
                right.kind
            } else {
                left.kind.or(right.kind)
            };
            Value {
                number: left.number + right.number,
                register: None,
                base: left.base.or(right.base),
                index: left.index.or(right.index),
                memory: left.memory || right.memory,
                bracketed: left.bracketed || right.bracketed,
                label: left.label.or(right.label),
                frame: left.frame.or(right.frame),
                kind,
                explicit: left.explicit || right.explicit,
                segment_override: left.segment_override.or(right.segment_override),
                unknown,
                short: left.short || right.short,
                count: left.count.max(right.count),
            }
        }
        Binary::Sub => {
            if right.register.is_some() || right.base.is_some() || right.index.is_some() {
                return Err("cannot subtract a register".to_string());
            }
            match (left.label, right.label) {
                (Some(_), Some(_)) => Value::constant(left.number - right.number),
                (None, Some(_)) => return Err("cannot subtract a label from a constant".to_string()),
                _ => {
                    left.number -= right.number;
                    left
                }
            }
        }
        _ => {
            let a = require_constant(&left).or_else(|error| if unknown { Ok(0) } else { Err(error) })?;
            let b = require_constant(&right).or_else(|error| if unknown { Ok(0) } else { Err(error) })?;
            let truth = |condition: bool| if condition { -1 } else { 0 };
            Value::constant(match operator {
                Binary::Mul => a.wrapping_mul(b),
                Binary::Div | Binary::Mod if b == 0 => {
                    if unknown {
                        0
                    } else {
                        return Err("division by zero".to_string());
                    }
                }
                Binary::Div => a / b,
                Binary::Mod => a % b,
                Binary::Shl => a.wrapping_shl(b as u32),
                Binary::Shr => a.wrapping_shr(b as u32),
                Binary::And => a & b,
                Binary::Or => a | b,
                Binary::Xor => a ^ b,
                Binary::Eq => truth(a == b),
                Binary::Ne => truth(a != b),
                Binary::Lt => truth(a < b),
                Binary::Le => truth(a <= b),
                Binary::Gt => truth(a > b),
                _ => truth(a >= b),
            })
        }
    };
    result.unknown = unknown;
    Ok(result)
}
