const SIGNATURE: &[u8; 2] = b"MZ";
const FIXED_HEADER: usize = 0x1C;
const PAGE: usize = 512;

pub struct Executable {
    pub image: Vec<u8>,
    pub relocations: Vec<(u16, u16)>,
    pub min_alloc: u16,
    pub max_alloc: u16,
    pub ss: u16,
    pub sp: u16,
    pub ip: u16,
    pub cs: u16,
}

fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

pub fn is_executable(bytes: &[u8]) -> bool {
    bytes.len() >= FIXED_HEADER && (&bytes[0..2] == SIGNATURE || &bytes[0..2] == b"ZM")
}

pub fn parse(bytes: &[u8]) -> Result<Executable, String> {
    if !is_executable(bytes) {
        return Err("not an MZ executable".to_string());
    }
    let last_page = word(bytes, 0x02) as usize;
    let pages = word(bytes, 0x04) as usize;
    let relocation_count = word(bytes, 0x06) as usize;
    let header_size = word(bytes, 0x08) as usize * 16;
    let relocation_table = word(bytes, 0x18) as usize;
    let mut total = pages * PAGE;
    if last_page != 0 {
        total = total.saturating_sub(PAGE - last_page);
    }
    let total = total.min(bytes.len());
    if header_size > total {
        return Err("corrupt MZ header".to_string());
    }
    let mut relocations = Vec::with_capacity(relocation_count);
    for index in 0..relocation_count {
        let position = relocation_table + index * 4;
        if position + 4 > bytes.len() {
            return Err("corrupt relocation table".to_string());
        }
        relocations.push((word(bytes, position), word(bytes, position + 2)));
    }
    Ok(Executable {
        image: bytes[header_size..total].to_vec(),
        relocations,
        min_alloc: word(bytes, 0x0A),
        max_alloc: word(bytes, 0x0C),
        ss: word(bytes, 0x0E),
        sp: word(bytes, 0x10),
        ip: word(bytes, 0x14),
        cs: word(bytes, 0x16),
    })
}

pub fn build(executable: &Executable) -> Vec<u8> {
    let table_end = FIXED_HEADER + executable.relocations.len() * 4;
    let header_paragraphs = table_end.div_ceil(16).max(2);
    let header_size = header_paragraphs * 16;
    let total = header_size + executable.image.len();
    let pages = total.div_ceil(PAGE);
    let mut output = vec![0u8; header_size];
    let fields: [(usize, u16); 13] = [
        (0x02, (total % PAGE) as u16),
        (0x04, pages as u16),
        (0x06, executable.relocations.len() as u16),
        (0x08, header_paragraphs as u16),
        (0x0A, executable.min_alloc),
        (0x0C, executable.max_alloc),
        (0x0E, executable.ss),
        (0x10, executable.sp),
        (0x12, 0),
        (0x14, executable.ip),
        (0x16, executable.cs),
        (0x18, FIXED_HEADER as u16),
        (0x1A, 0),
    ];
    output[0..2].copy_from_slice(SIGNATURE);
    for (offset, value) in fields {
        output[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    for (index, (offset, segment)) in executable.relocations.iter().enumerate() {
        let position = FIXED_HEADER + index * 4;
        output[position..position + 2].copy_from_slice(&offset.to_le_bytes());
        output[position + 2..position + 4].copy_from_slice(&segment.to_le_bytes());
    }
    output.extend_from_slice(&executable.image);
    output
}
