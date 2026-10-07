const MAGIC: &[u8; 6] = b"RMOBJ1";

#[derive(Clone)]
pub struct SegmentInfo {
    pub name: String,
    pub class: String,
    pub group: String,
    pub start: u32,
    pub length: u32,
}

#[derive(Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub segment: u16,
    pub offset: u16,
}

#[derive(Default)]
pub struct ObjectFile {
    pub source: String,
    pub image: Vec<u8>,
    pub relocations: Vec<u32>,
    pub entry: Option<(u16, u16)>,
    pub stack: Option<(u16, u16)>,
    pub segments: Vec<SegmentInfo>,
    pub symbols: Vec<SymbolInfo>,
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }
    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn text(&mut self, value: &str) {
        let bytes = value.as_bytes();
        self.u16(bytes.len() as u16);
        self.bytes.extend_from_slice(bytes);
    }
    fn pair(&mut self, value: Option<(u16, u16)>) {
        self.u8(value.is_some() as u8);
        let (first, second) = value.unwrap_or((0, 0));
        self.u16(first);
        self.u16(second);
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.position + count;
        let slice = self.bytes.get(self.position..end).ok_or("object file is truncated")?;
        self.position = end;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }
    fn u32(&mut self) -> Result<u32, String> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
    fn text(&mut self) -> Result<String, String> {
        let length = self.u16()? as usize;
        Ok(String::from_utf8_lossy(self.take(length)?).to_string())
    }
    fn pair(&mut self) -> Result<Option<(u16, u16)>, String> {
        let present = self.u8()? != 0;
        let first = self.u16()?;
        let second = self.u16()?;
        Ok(present.then_some((first, second)))
    }
}

impl ObjectFile {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = Writer { bytes: MAGIC.to_vec() };
        writer.text(&self.source);
        writer.pair(self.entry);
        writer.pair(self.stack);
        writer.u32(self.image.len() as u32);
        writer.bytes.extend_from_slice(&self.image);
        writer.u32(self.relocations.len() as u32);
        for relocation in &self.relocations {
            writer.u32(*relocation);
        }
        writer.u16(self.segments.len() as u16);
        for segment in &self.segments {
            writer.text(&segment.name);
            writer.text(&segment.class);
            writer.text(&segment.group);
            writer.u32(segment.start);
            writer.u32(segment.length);
        }
        writer.u32(self.symbols.len() as u32);
        for symbol in &self.symbols {
            writer.text(&symbol.name);
            writer.u16(symbol.segment);
            writer.u16(symbol.offset);
        }
        writer.bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if !bytes.starts_with(MAGIC) {
            return Err("not an object file produced by MASM".to_string());
        }
        let mut reader = Reader { bytes, position: MAGIC.len() };
        let source = reader.text()?;
        let entry = reader.pair()?;
        let stack = reader.pair()?;
        let image_length = reader.u32()? as usize;
        let image = reader.take(image_length)?.to_vec();
        let relocation_count = reader.u32()? as usize;
        let relocations = (0..relocation_count).map(|_| reader.u32()).collect::<Result<_, _>>()?;
        let segment_count = reader.u16()? as usize;
        let mut segments = Vec::with_capacity(segment_count);
        for _ in 0..segment_count {
            segments.push(SegmentInfo {
                name: reader.text()?,
                class: reader.text()?,
                group: reader.text()?,
                start: reader.u32()?,
                length: reader.u32()?,
            });
        }
        let symbol_count = reader.u32()? as usize;
        let mut symbols = Vec::with_capacity(symbol_count);
        for _ in 0..symbol_count {
            symbols.push(SymbolInfo { name: reader.text()?, segment: reader.u16()?, offset: reader.u16()? });
        }
        Ok(ObjectFile { source, image, relocations, entry, stack, segments, symbols })
    }
}
