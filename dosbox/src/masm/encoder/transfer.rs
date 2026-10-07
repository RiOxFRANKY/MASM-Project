use super::emitter::{fits_signed_byte, pair_size, register_code, single_size, word_bit, Emitter};
use super::expect_count;
use crate::masm::operand::Operand;
use crate::masm::registers::Register;

const DX: u8 = 2;

pub fn encode(emitter: &mut Emitter, mnemonic: &str, operands: &[Operand]) -> Result<bool, String> {
    match mnemonic {
        "MOV" => {
            expect_count(operands, 2, mnemonic)?;
            mov(emitter, &operands[0], &operands[1])?;
        }
        "XCHG" => {
            expect_count(operands, 2, mnemonic)?;
            exchange(emitter, &operands[0], &operands[1])?;
        }
        "LEA" | "LDS" | "LES" => {
            expect_count(operands, 2, mnemonic)?;
            let Operand::Register(Register::Word(target)) = operands[0] else {
                return Err(format!("{mnemonic} needs a word register as destination"));
            };
            if !matches!(operands[1], Operand::Memory(_)) {
                return Err(format!("{mnemonic} needs a memory operand"));
            }
            emitter.segment_prefix(&operands[1]);
            emitter.byte(match mnemonic {
                "LEA" => 0x8D,
                "LDS" => 0xC5,
                _ => 0xC4,
            });
            emitter.modrm(target, &operands[1])?;
        }
        "PUSH" | "POP" => {
            if operands.is_empty() {
                return Err(format!("{mnemonic} needs an operand"));
            }
            for operand in operands {
                push_pop(emitter, mnemonic == "PUSH", operand)?;
            }
        }
        "IN" => {
            expect_count(operands, 2, mnemonic)?;
            let w = accumulator(&operands[0])?;
            port(emitter, 0xE4 | w, 0xEC | w, &operands[1])?;
        }
        "OUT" => {
            expect_count(operands, 2, mnemonic)?;
            let w = accumulator(&operands[1])?;
            port(emitter, 0xE6 | w, 0xEE | w, &operands[0])?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn mov(emitter: &mut Emitter, destination: &Operand, source: &Operand) -> Result<(), String> {
    match (destination, source) {
        (Operand::Register(Register::Segment(segment)), other) => {
            if *segment == 1 {
                return Err("cannot move into CS".to_string());
            }
            if matches!(other, Operand::Immediate(_)) || matches!(other, Operand::Register(Register::Segment(_))) {
                return Err("segment registers can only be loaded from a word register or memory".to_string());
            }
            if other.size().is_some_and(|size| size != 2) {
                return Err("operand types must match".to_string());
            }
            emitter.segment_prefix(other);
            emitter.byte(0x8E);
            emitter.modrm(*segment, other)
        }
        (other, Operand::Register(Register::Segment(segment))) => {
            if matches!(other, Operand::Immediate(_)) {
                return Err("invalid destination".to_string());
            }
            if other.size().is_some_and(|size| size != 2) {
                return Err("operand types must match".to_string());
            }
            emitter.segment_prefix(other);
            emitter.byte(0x8C);
            emitter.modrm(*segment, other)
        }
        (Operand::Immediate(_), _) => Err("immediate value cannot be a destination".to_string()),
        (Operand::Register(target), Operand::Immediate(immediate)) => {
            let size = pair_size(destination, source)?;
            emitter.byte(if size == 1 { 0xB0 } else { 0xB8 } + register_code(*target));
            emitter.immediate(immediate, size)
        }
        (Operand::Memory(_), Operand::Immediate(immediate)) => {
            let size = single_size(destination)?;
            let w = word_bit(size)?;
            emitter.segment_prefix(destination);
            emitter.byte(0xC6 | w);
            emitter.modrm(0, destination)?;
            emitter.immediate(immediate, size)
        }
        (Operand::Register(target), Operand::Memory(memory)) => {
            let w = word_bit(pair_size(destination, source)?)?;
            emitter.segment_prefix(source);
            if register_code(*target) == 0 && memory.direct {
                emitter.byte(0xA0 | w);
                emitter.word(memory.displacement as u16);
                return Ok(());
            }
            emitter.byte(0x8A | w);
            emitter.modrm(register_code(*target), source)
        }
        (Operand::Memory(memory), Operand::Register(value)) => {
            let w = word_bit(pair_size(destination, source)?)?;
            emitter.segment_prefix(destination);
            if register_code(*value) == 0 && memory.direct {
                emitter.byte(0xA2 | w);
                emitter.word(memory.displacement as u16);
                return Ok(());
            }
            emitter.byte(0x88 | w);
            emitter.modrm(register_code(*value), destination)
        }
        (Operand::Register(target), Operand::Register(_)) => {
            let w = word_bit(pair_size(destination, source)?)?;
            emitter.byte(0x8A | w);
            emitter.modrm(register_code(*target), source)
        }
        (Operand::Memory(_), Operand::Memory(_)) => Err("cannot move memory to memory".to_string()),
    }
}

fn exchange(emitter: &mut Emitter, left: &Operand, right: &Operand) -> Result<(), String> {
    let w = word_bit(pair_size(left, right)?)?;
    match (left, right) {
        (Operand::Register(Register::Word(0)), Operand::Register(Register::Word(code)))
        | (Operand::Register(Register::Word(code)), Operand::Register(Register::Word(0))) => {
            emitter.byte(0x90 + code);
            Ok(())
        }
        (Operand::Register(Register::Segment(_)), _) | (_, Operand::Register(Register::Segment(_))) => {
            Err("XCHG cannot use segment registers".to_string())
        }
        (Operand::Register(register), other) | (other, Operand::Register(register)) => {
            if matches!(other, Operand::Immediate(_)) {
                return Err("XCHG cannot use an immediate".to_string());
            }
            emitter.segment_prefix(other);
            emitter.byte(0x86 | w);
            emitter.modrm(register_code(*register), other)
        }
        _ => Err("invalid operand combination".to_string()),
    }
}

fn push_pop(emitter: &mut Emitter, push: bool, operand: &Operand) -> Result<(), String> {
    match operand {
        Operand::Register(Register::Word(code)) => emitter.byte(if push { 0x50 } else { 0x58 } + code),
        Operand::Register(Register::Segment(segment)) => {
            if !push && *segment == 1 {
                return Err("cannot POP CS".to_string());
            }
            emitter.byte(segment * 8 + if push { 0x06 } else { 0x07 });
        }
        Operand::Register(Register::Byte(_)) => return Err("PUSH and POP need a word operand".to_string()),
        Operand::Memory(_) => {
            if operand.size().is_some_and(|size| size != 2) {
                return Err("PUSH and POP need a word operand".to_string());
            }
            emitter.segment_prefix(operand);
            emitter.byte(if push { 0xFF } else { 0x8F });
            emitter.modrm(if push { 6 } else { 0 }, operand)?;
        }
        Operand::Immediate(immediate) => {
            if !push {
                return Err("cannot POP into an immediate".to_string());
            }
            if immediate.frame.is_none() && !immediate.unknown && fits_signed_byte(immediate.value) {
                emitter.byte(0x6A);
                emitter.byte(immediate.value as u8);
            } else {
                emitter.byte(0x68);
                emitter.immediate(immediate, 2)?;
            }
        }
    }
    Ok(())
}

fn accumulator(operand: &Operand) -> Result<u8, String> {
    match operand {
        Operand::Register(Register::Byte(0)) => Ok(0),
        Operand::Register(Register::Word(0)) => Ok(1),
        _ => Err("IN and OUT need AL or AX".to_string()),
    }
}

fn port(emitter: &mut Emitter, immediate_form: u8, dx_form: u8, operand: &Operand) -> Result<(), String> {
    match operand {
        Operand::Register(Register::Word(DX)) => {
            emitter.byte(dx_form);
            Ok(())
        }
        Operand::Immediate(immediate) if (0..=255).contains(&immediate.value) || immediate.unknown => {
            emitter.byte(immediate_form);
            emitter.byte(immediate.value as u8);
            Ok(())
        }
        _ => Err("port must be DX or a constant 0-255".to_string()),
    }
}
