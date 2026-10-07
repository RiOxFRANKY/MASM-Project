use super::emitter::Emitter;
use super::Context;
use crate::masm::operand::{Immediate, Operand};

const SINGLE: [(&str, u8); 33] = [
    ("NOP", 0x90),
    ("HLT", 0xF4),
    ("CMC", 0xF5),
    ("CLC", 0xF8),
    ("STC", 0xF9),
    ("CLI", 0xFA),
    ("STI", 0xFB),
    ("CLD", 0xFC),
    ("STD", 0xFD),
    ("CBW", 0x98),
    ("CWD", 0x99),
    ("PUSHF", 0x9C),
    ("POPF", 0x9D),
    ("SAHF", 0x9E),
    ("LAHF", 0x9F),
    ("DAA", 0x27),
    ("DAS", 0x2F),
    ("AAA", 0x37),
    ("AAS", 0x3F),
    ("WAIT", 0x9B),
    ("FWAIT", 0x9B),
    ("PUSHA", 0x60),
    ("POPA", 0x61),
    ("LEAVE", 0xC9),
    ("INTO", 0xCE),
    ("IRET", 0xCF),
    ("XLATB", 0xD7),
    ("MOVSB", 0xA4),
    ("MOVSW", 0xA5),
    ("CMPSB", 0xA6),
    ("CMPSW", 0xA7),
    ("STOSB", 0xAA),
    ("STOSW", 0xAB),
];

const SINGLE_MORE: [(&str, u8); 5] = [("LODSB", 0xAC), ("LODSW", 0xAD), ("SCASB", 0xAE), ("SCASW", 0xAF), ("SALC", 0xD6)];

const STRINGS: [(&str, u8); 5] = [("MOVS", 0xA4), ("CMPS", 0xA6), ("STOS", 0xAA), ("LODS", 0xAC), ("SCAS", 0xAE)];

fn constant(operand: &Operand) -> Option<&Immediate> {
    match operand {
        Operand::Immediate(immediate) if immediate.frame.is_none() => Some(immediate),
        _ => None,
    }
}

pub fn encode(emitter: &mut Emitter, mnemonic: &str, operands: &[Operand], context: &Context) -> Result<bool, String> {
    if let Some((_, code)) = SINGLE.iter().chain(SINGLE_MORE.iter()).find(|(name, _)| *name == mnemonic) {
        if !operands.is_empty() {
            return Err(format!("{mnemonic} takes no operands"));
        }
        emitter.byte(*code);
        return Ok(true);
    }
    if let Some((_, code)) = STRINGS.iter().find(|(name, _)| *name == mnemonic) {
        let size = operands.iter().find_map(Operand::size).ok_or("string instruction needs a sized operand")?;
        let source = if matches!(mnemonic, "MOVS" | "CMPS") { operands.get(1) } else { operands.first() };
        if let Some(source) = source.filter(|_| mnemonic != "STOS" && mnemonic != "SCAS") {
            emitter.segment_prefix(source);
        }
        emitter.byte(code + if size == 2 { 1 } else { 0 });
        return Ok(true);
    }
    match mnemonic {
        "INT" => {
            let number = operands.first().and_then(constant).ok_or("INT needs a constant")?;
            if number.value == 3 && !number.unknown {
                emitter.byte(0xCC);
            } else {
                emitter.byte(0xCD);
                emitter.immediate(number, 1)?;
            }
        }
        "RET" | "RETN" | "RETF" => {
            let far = mnemonic == "RETF" || (mnemonic == "RET" && context.far_proc);
            match operands.first() {
                None => emitter.byte(if far { 0xCB } else { 0xC3 }),
                Some(operand) => {
                    let count = constant(operand).ok_or("RET needs a constant")?;
                    emitter.byte(if far { 0xCA } else { 0xC2 });
                    emitter.immediate(count, 2)?;
                }
            }
        }
        "AAM" | "AAD" => {
            emitter.byte(if mnemonic == "AAM" { 0xD4 } else { 0xD5 });
            match operands.first() {
                None => emitter.byte(10),
                Some(operand) => emitter.immediate(constant(operand).ok_or("constant expected")?, 1)?,
            }
        }
        "XLAT" => {
            if let Some(operand) = operands.first() {
                emitter.segment_prefix(operand);
            }
            emitter.byte(0xD7);
        }
        "ENTER" => {
            if operands.len() != 2 {
                return Err("ENTER needs 2 operands".to_string());
            }
            emitter.byte(0xC8);
            emitter.immediate(constant(&operands[0]).ok_or("constant expected")?, 2)?;
            emitter.immediate(constant(&operands[1]).ok_or("constant expected")?, 1)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
