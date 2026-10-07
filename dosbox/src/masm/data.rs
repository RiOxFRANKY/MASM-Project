use super::assembler::Assembler;
use super::expr::Expr;
use super::segments::Frame;
use super::types::TypeKind;

const MAX_DATA: usize = 0x10000;

#[derive(Default)]
struct DataBuffer {
    bytes: Vec<u8>,
    relocations: Vec<(usize, Frame)>,
}

impl DataBuffer {
    fn number(&mut self, value: i64, size: usize) {
        let bytes = value.to_le_bytes();
        self.bytes.extend_from_slice(&bytes[..size.min(8)]);
        if size > 8 {
            let fill = if value < 0 { 0xFF } else { 0 };
            self.bytes.extend(std::iter::repeat_n(fill, size - 8));
        }
    }

    fn repeat(&mut self, other: &DataBuffer, times: usize) -> Result<(), String> {
        if self.bytes.len() + other.bytes.len() * times > MAX_DATA {
            return Err("data definition too large".to_string());
        }
        for _ in 0..times {
            let start = self.bytes.len();
            self.bytes.extend_from_slice(&other.bytes);
            self.relocations.extend(other.relocations.iter().map(|(position, frame)| (start + position, *frame)));
        }
        Ok(())
    }
}

impl Assembler {
    pub fn data(&mut self, name: Option<&str>, kind: TypeKind, items: &[Expr]) -> Result<(), String> {
        let (segment, location) = self.location()?;
        let mut buffer = DataBuffer::default();
        let mut count = 0;
        let mut failure = None;
        for item in items {
            match self.data_item(kind, item, &mut buffer) {
                Ok(items_written) => count += items_written,
                Err(message) => {
                    failure = Some(message);
                    break;
                }
            }
        }
        if let Some(name) = name {
            self.define(name, super::symbols::SymbolKind::Label { segment, offset: location, kind, count }, false)?;
        }
        for (position, frame) in &buffer.relocations {
            self.segments.list[segment].relocations.push((location + *position as u32, *frame));
        }
        self.segments.emit(segment, &buffer.bytes);
        match failure {
            Some(message) => Err(message),
            None => Ok(()),
        }
    }

    fn data_item(&mut self, kind: TypeKind, item: &Expr, buffer: &mut DataBuffer) -> Result<u32, String> {
        let size = kind.size() as usize;
        match item {
            Expr::Undefined => {
                buffer.number(0, size);
                Ok(1)
            }
            Expr::Str(bytes) if kind == TypeKind::Byte => {
                buffer.bytes.extend_from_slice(bytes);
                Ok(bytes.len() as u32)
            }
            Expr::Str(bytes) if bytes.len() > size => Err("string too long for this data type".to_string()),
            Expr::Dup(count, inner) => {
                let times = self.evaluate(count)?;
                if !times.is_constant() || times.number < 0 {
                    return Err("DUP count must be a non-negative constant".to_string());
                }
                let mut block = DataBuffer::default();
                let mut per_block = 0;
                for element in inner {
                    per_block += self.data_item(kind, element, &mut block)?;
                }
                buffer.repeat(&block, times.number as usize)?;
                Ok(per_block * times.number as u32)
            }
            _ => {
                let value = self.evaluate(item)?;
                if value.register.is_some() || value.base.is_some() || value.index.is_some() {
                    return Err("registers are not allowed in data".to_string());
                }
                match kind {
                    TypeKind::Byte => {
                        if value.frame.is_some() || value.label.is_some() {
                            return Err("an address does not fit in a byte".to_string());
                        }
                        if !value.unknown && !(-128..=255).contains(&value.number) {
                            return Err(format!("value {} does not fit in a byte", value.number));
                        }
                        buffer.number(value.number, 1);
                    }
                    TypeKind::Word => {
                        if let Some((frame, paragraph)) = value.frame {
                            buffer.relocations.push((buffer.bytes.len(), frame));
                            buffer.number(paragraph as i64, 2);
                        } else {
                            if value.label.is_none() && !value.unknown && !(-0x8000..=0xFFFF).contains(&value.number) {
                                return Err(format!("value {} does not fit in a word", value.number));
                            }
                            buffer.number(value.number, 2);
                        }
                    }
                    TypeKind::Dword | TypeKind::Far if value.label.is_some() => {
                        let Some(label) = value.label else {
                            return Err("address expected".to_string());
                        };
                        buffer.number(value.number, 2);
                        buffer.relocations.push((buffer.bytes.len(), label.frame));
                        buffer.number(label.paragraph as i64, 2);
                    }
                    _ => {
                        if value.frame.is_some() {
                            return Err("segment values must be stored in a word".to_string());
                        }
                        buffer.number(value.number, size);
                    }
                }
                Ok(1)
            }
        }
    }
}
