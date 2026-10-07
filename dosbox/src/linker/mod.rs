mod map;

use crate::formats::mz::{self, Executable};
use crate::formats::object::ObjectFile;

const COM_ORIGIN: usize = 0x100;
const COM_LIMIT: usize = 0xFF00;

pub enum OutputKind {
    Exe,
    Com,
}

pub struct Linked {
    pub bytes: Vec<u8>,
    pub warnings: Vec<String>,
    pub map: String,
}

pub fn link(object: &ObjectFile, kind: OutputKind) -> Result<Linked, String> {
    let mut warnings = Vec::new();
    let entry = match object.entry {
        Some(entry) => entry,
        None => {
            warnings.push("no starting address (END has no label); execution starts at 0000:0000".to_string());
            (0, 0)
        }
    };
    let map = map::render(object, entry);
    let bytes = match kind {
        OutputKind::Exe => {
            let (ss, sp) = match object.stack {
                Some(stack) => stack,
                None => {
                    warnings.push("no stack segment".to_string());
                    (0, 0)
                }
            };
            let relocations = object
                .relocations
                .iter()
                .map(|position| (((position % 16) as u16), (position / 16) as u16))
                .collect();
            mz::build(&Executable {
                image: object.image.clone(),
                relocations,
                min_alloc: 0,
                max_alloc: 0xFFFF,
                ss,
                sp,
                ip: entry.1,
                cs: entry.0,
            })
        }
        OutputKind::Com => {
            if !object.relocations.is_empty() {
                return Err("a .COM program cannot contain segment references (use .MODEL TINY)".to_string());
            }
            if entry != (0, COM_ORIGIN as u16) {
                return Err("a .COM program must start at offset 100h (use ORG 100h)".to_string());
            }
            if object.image.len() <= COM_ORIGIN {
                return Err("program is empty".to_string());
            }
            let body = object.image[COM_ORIGIN..].to_vec();
            if body.len() > COM_LIMIT {
                return Err("program is too large for a .COM file".to_string());
            }
            body
        }
    };
    Ok(Linked { bytes, warnings, map })
}
