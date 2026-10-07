use super::emitter::Emitter;
use super::Context;
use crate::masm::eval::{LabelRef, Value};
use crate::masm::operand::{to_operand, Operand};
use crate::masm::registers::Register;
use crate::masm::types::TypeKind;

const CONDITIONS: [(&str, u8); 30] = [
    ("JO", 0x0),
    ("JNO", 0x1),
    ("JB", 0x2),
    ("JC", 0x2),
    ("JNAE", 0x2),
    ("JAE", 0x3),
    ("JNB", 0x3),
    ("JNC", 0x3),
    ("JE", 0x4),
    ("JZ", 0x4),
    ("JNE", 0x5),
    ("JNZ", 0x5),
    ("JBE", 0x6),
    ("JNA", 0x6),
    ("JA", 0x7),
    ("JNBE", 0x7),
    ("JS", 0x8),
    ("JNS", 0x9),
    ("JP", 0xA),
    ("JPE", 0xA),
    ("JNP", 0xB),
    ("JPO", 0xB),
    ("JL", 0xC),
    ("JNGE", 0xC),
    ("JGE", 0xD),
    ("JNL", 0xD),
    ("JLE", 0xE),
    ("JNG", 0xE),
    ("JG", 0xF),
    ("JNLE", 0xF),
];

const LOOPS: [(&str, u8); 7] =
    [("LOOPNE", 0xE0), ("LOOPNZ", 0xE0), ("LOOPE", 0xE1), ("LOOPZ", 0xE1), ("LOOP", 0xE2), ("JCXZ", 0xE3), ("JCXE", 0xE3)];

enum Kind {
    Jump,
    Call,
    Conditional(u8),
    Loop(u8),
}

struct Direct {
    offset: i64,
    label: Option<LabelRef>,
    distance: Option<TypeKind>,
    explicit: bool,
    short: bool,
    unknown: bool,
}

enum Target {
    Direct(Direct),
    Indirect { operand: Operand, far: bool },
}

fn fits(relative: i64) -> bool {
    (-128..=127).contains(&relative)
}

fn classify(value: &Value) -> Result<Target, String> {
    if value.register.is_some() || value.memory || value.base.is_some() || value.index.is_some() {
        let operand = to_operand(value)?;
        if let Operand::Register(register) = operand {
            if !matches!(register, Register::Word(_)) {
                return Err("indirect jumps need a word register".to_string());
            }
        }
        let far = matches!(value.kind, Some(TypeKind::Far) | Some(TypeKind::Dword));
        return Ok(Target::Indirect { operand, far });
    }
    if value.frame.is_some() {
        return Err("cannot jump to a segment".to_string());
    }
    Ok(Target::Direct(Direct {
        offset: value.number,
        label: value.label,
        distance: value.kind.filter(|kind| kind.is_code()),
        explicit: value.explicit,
        short: value.short,
        unknown: value.unknown,
    }))
}

pub fn encode(emitter: &mut Emitter, mnemonic: &str, values: &[Value], context: &Context) -> Result<bool, String> {
    let kind = if mnemonic == "JMP" {
        Kind::Jump
    } else if mnemonic == "CALL" {
        Kind::Call
    } else if let Some((_, code)) = CONDITIONS.iter().find(|(name, _)| *name == mnemonic) {
        Kind::Conditional(*code)
    } else if let Some((_, code)) = LOOPS.iter().find(|(name, _)| *name == mnemonic) {
        Kind::Loop(*code)
    } else {
        return Ok(false);
    };
    if values.len() != 1 {
        return Err(format!("{mnemonic} needs exactly one operand"));
    }
    match classify(&values[0])? {
        Target::Indirect { operand, far } => {
            let reg_field = match kind {
                Kind::Jump => if far { 5 } else { 4 },
                Kind::Call => if far { 3 } else { 2 },
                _ => return Err(format!("{mnemonic} needs a label")),
            };
            emitter.segment_prefix(&operand);
            emitter.byte(0xFF);
            emitter.modrm(reg_field, &operand)?;
        }
        Target::Direct(target) => direct(emitter, kind, &target, context)?,
    }
    Ok(true)
}

fn is_far(target: &Direct, context: &Context) -> bool {
    match target.distance {
        Some(TypeKind::Far) => true,
        Some(TypeKind::Near) if target.explicit => false,
        _ => target.label.is_some_and(|label| label.frame != context.frame),
    }
}

fn far_target(emitter: &mut Emitter, opcode: u8, target: &Direct) -> Result<(), String> {
    let label = target.label.ok_or("far transfer needs a label")?;
    emitter.byte(opcode);
    emitter.word(target.offset as u16);
    emitter.relocations.push((emitter.len(), label.frame));
    emitter.word(label.paragraph);
    Ok(())
}

fn out_of_range(relative: i64) -> String {
    let excess = if relative > 127 { relative - 127 } else { -128 - relative };
    format!("jump out of range by {excess} byte(s)")
}

fn direct(emitter: &mut Emitter, kind: Kind, target: &Direct, context: &Context) -> Result<(), String> {
    let start = context.here + emitter.len() as i64;
    let short_relative = target.offset - (start + 2);
    let short_allowed = context.min_length <= emitter.len() + 2 && (fits(short_relative) || target.unknown);
    match kind {
        Kind::Jump => {
            if is_far(target, context) {
                return far_target(emitter, 0xEA, target);
            }
            let force_near = target.explicit && target.distance == Some(TypeKind::Near);
            if target.short || (!force_near && short_allowed) {
                if !fits(short_relative) && !target.unknown {
                    return Err(out_of_range(short_relative));
                }
                emitter.byte(0xEB);
                emitter.byte(short_relative as u8);
            } else {
                emitter.byte(0xE9);
                emitter.word((target.offset - (start + 3)) as u16);
            }
        }
        Kind::Call => {
            if is_far(target, context) {
                return far_target(emitter, 0x9A, target);
            }
            emitter.byte(0xE8);
            emitter.word((target.offset - (start + 3)) as u16);
        }
        Kind::Conditional(code) => {
            if target.short || short_allowed {
                if !fits(short_relative) && !target.unknown {
                    return Err(out_of_range(short_relative));
                }
                emitter.byte(0x70 + code);
                emitter.byte(short_relative as u8);
            } else {
                if !target.unknown && !fits(short_relative) {
                    emitter.warnings.push(format!(
                        "conditional {}; extended with JMP (MASM 5 would reject this)",
                        out_of_range(short_relative)
                    ));
                }
                emitter.byte(0x70 + (code ^ 1));
                emitter.byte(3);
                emitter.byte(0xE9);
                emitter.word((target.offset - (start + 5)) as u16);
            }
        }
        Kind::Loop(opcode) => {
            if !fits(short_relative) && !target.unknown {
                return Err(out_of_range(short_relative));
            }
            emitter.byte(opcode);
            emitter.byte(short_relative as u8);
        }
    }
    Ok(())
}
