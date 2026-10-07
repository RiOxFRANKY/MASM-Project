use super::emitter::{fits_signed_byte, pair_size, register_code, single_size, word_bit, Emitter};
use super::expect_count;
use crate::masm::operand::Operand;
use crate::masm::registers::Register;

const ALU: [(&str, u8); 8] = [("ADD", 0), ("OR", 1), ("ADC", 2), ("SBB", 3), ("AND", 4), ("SUB", 5), ("XOR", 6), ("CMP", 7)];
const UNARY: [(&str, u8); 6] = [("NOT", 2), ("NEG", 3), ("MUL", 4), ("IMUL", 5), ("DIV", 6), ("IDIV", 7)];
const SHIFTS: [(&str, u8); 8] = [("ROL", 0), ("ROR", 1), ("RCL", 2), ("RCR", 3), ("SHL", 4), ("SAL", 4), ("SHR", 5), ("SAR", 7)];

fn lookup(table: &[(&str, u8)], mnemonic: &str) -> Option<u8> {
    table.iter().find(|(name, _)| *name == mnemonic).map(|(_, code)| *code)
}

pub fn encode(emitter: &mut Emitter, mnemonic: &str, operands: &[Operand]) -> Result<bool, String> {
    if let Some(operation) = lookup(&ALU, mnemonic) {
        expect_count(operands, 2, mnemonic)?;
        alu(emitter, operation, &operands[0], &operands[1])?;
        return Ok(true);
    }
    if mnemonic == "IMUL" && operands.len() >= 2 {
        multiply_immediate(emitter, operands)?;
        return Ok(true);
    }
    if let Some(operation) = lookup(&UNARY, mnemonic) {
        expect_count(operands, 1, mnemonic)?;
        let size = single_size(&operands[0])?;
        if matches!(operands[0], Operand::Immediate(_)) {
            return Err(format!("{mnemonic} cannot take an immediate operand"));
        }
        emitter.segment_prefix(&operands[0]);
        emitter.byte(0xF6 | word_bit(size)?);
        emitter.modrm(operation, &operands[0])?;
        return Ok(true);
    }
    if let Some(operation) = lookup(&SHIFTS, mnemonic) {
        expect_count(operands, 2, mnemonic)?;
        shift(emitter, operation, &operands[0], &operands[1])?;
        return Ok(true);
    }
    match mnemonic {
        "INC" | "DEC" => {
            expect_count(operands, 1, mnemonic)?;
            let operation = if mnemonic == "INC" { 0 } else { 1 };
            match &operands[0] {
                Operand::Register(Register::Word(code)) => emitter.byte(0x40 + operation * 8 + code),
                Operand::Immediate(_) | Operand::Register(Register::Segment(_)) => {
                    return Err(format!("invalid operand for {mnemonic}"));
                }
                operand => {
                    let size = single_size(operand)?;
                    emitter.segment_prefix(operand);
                    emitter.byte(0xFE | word_bit(size)?);
                    emitter.modrm(operation, operand)?;
                }
            }
            Ok(true)
        }
        "TEST" => {
            expect_count(operands, 2, mnemonic)?;
            test(emitter, &operands[0], &operands[1])?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn alu(emitter: &mut Emitter, operation: u8, destination: &Operand, source: &Operand) -> Result<(), String> {
    let size = pair_size(destination, source)?;
    let w = word_bit(size)?;
    match (destination, source) {
        (Operand::Register(Register::Segment(_)), _) | (_, Operand::Register(Register::Segment(_))) => {
            Err("segment registers cannot be used in arithmetic".to_string())
        }
        (Operand::Register(target), Operand::Register(_) | Operand::Memory(_)) => {
            emitter.segment_prefix(source);
            emitter.byte(operation * 8 + 2 + w);
            emitter.modrm(register_code(*target), source)
        }
        (Operand::Memory(_), Operand::Register(value)) => {
            emitter.segment_prefix(destination);
            emitter.byte(operation * 8 + w);
            emitter.modrm(register_code(*value), destination)
        }
        (Operand::Register(Register::Byte(0) | Register::Word(0)), Operand::Immediate(immediate)) => {
            emitter.byte(operation * 8 + 4 + w);
            emitter.immediate(immediate, size)
        }
        (Operand::Register(_) | Operand::Memory(_), Operand::Immediate(immediate)) => {
            emitter.segment_prefix(destination);
            if size == 2 && immediate.frame.is_none() && !immediate.unknown && fits_signed_byte(immediate.value) {
                emitter.byte(0x83);
                emitter.modrm(operation, destination)?;
                emitter.byte(immediate.value as u8);
                return Ok(());
            }
            emitter.byte(0x80 | w);
            emitter.modrm(operation, destination)?;
            emitter.immediate(immediate, size)
        }
        _ => Err("invalid operand combination".to_string()),
    }
}

fn test(emitter: &mut Emitter, left: &Operand, right: &Operand) -> Result<(), String> {
    let size = pair_size(left, right)?;
    let w = word_bit(size)?;
    match (left, right) {
        (Operand::Register(Register::Byte(0) | Register::Word(0)), Operand::Immediate(immediate)) => {
            emitter.byte(0xA8 | w);
            emitter.immediate(immediate, size)
        }
        (Operand::Register(_) | Operand::Memory(_), Operand::Immediate(immediate)) => {
            emitter.segment_prefix(left);
            emitter.byte(0xF6 | w);
            emitter.modrm(0, left)?;
            emitter.immediate(immediate, size)
        }
        (_, Operand::Register(register)) => {
            emitter.segment_prefix(left);
            emitter.byte(0x84 | w);
            emitter.modrm(register_code(*register), left)
        }
        (Operand::Register(register), Operand::Memory(_)) => {
            emitter.segment_prefix(right);
            emitter.byte(0x84 | w);
            emitter.modrm(register_code(*register), right)
        }
        _ => Err("invalid operand combination".to_string()),
    }
}

fn shift(emitter: &mut Emitter, operation: u8, target: &Operand, count: &Operand) -> Result<(), String> {
    let size = single_size(target)?;
    let w = word_bit(size)?;
    if matches!(target, Operand::Immediate(_)) {
        return Err("cannot shift an immediate".to_string());
    }
    match count {
        Operand::Register(Register::Byte(1)) => {
            emitter.segment_prefix(target);
            emitter.byte(0xD2 | w);
            emitter.modrm(operation, target)
        }
        Operand::Immediate(immediate) if immediate.value == 1 && immediate.frame.is_none() => {
            emitter.segment_prefix(target);
            emitter.byte(0xD0 | w);
            emitter.modrm(operation, target)
        }
        Operand::Immediate(immediate) => {
            emitter.segment_prefix(target);
            emitter.byte(0xC0 | w);
            emitter.modrm(operation, target)?;
            emitter.immediate(immediate, 1)
        }
        _ => Err("shift count must be 1, CL or a constant".to_string()),
    }
}

fn multiply_immediate(emitter: &mut Emitter, operands: &[Operand]) -> Result<(), String> {
    let Operand::Register(Register::Word(target)) = operands[0] else {
        return Err("IMUL with an immediate needs a word register".to_string());
    };
    let (source, immediate) = match operands {
        [_, Operand::Immediate(immediate)] => (&operands[0], immediate),
        [_, source, Operand::Immediate(immediate)] => (source, immediate),
        _ => return Err("invalid IMUL operands".to_string()),
    };
    emitter.segment_prefix(source);
    if immediate.frame.is_none() && !immediate.unknown && fits_signed_byte(immediate.value) {
        emitter.byte(0x6B);
        emitter.modrm(target, source)?;
        emitter.byte(immediate.value as u8);
        Ok(())
    } else {
        emitter.byte(0x69);
        emitter.modrm(target, source)?;
        emitter.immediate(immediate, 2)
    }
}
