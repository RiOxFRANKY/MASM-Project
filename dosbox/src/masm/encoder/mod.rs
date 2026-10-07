mod arith;
mod branch;
mod emitter;
mod misc;
mod transfer;

use super::eval::Value;
use super::operand::{to_operand, Operand};
use super::segments::Frame;
pub use emitter::Encoded;
use emitter::Emitter;

pub struct Context {
    pub here: i64,
    pub frame: Frame,
    pub far_proc: bool,
    pub min_length: usize,
}

pub fn encode(prefixes: &[u8], mnemonic: &str, values: &[Value], context: &Context) -> Result<Encoded, String> {
    let mut emitter = Emitter::default();
    for prefix in prefixes {
        emitter.byte(*prefix);
    }
    if mnemonic.is_empty() {
        return Ok(emitter.finish());
    }
    if branch::encode(&mut emitter, mnemonic, values, context)? {
        return Ok(emitter.finish());
    }
    let operands: Vec<Operand> = values.iter().map(to_operand).collect::<Result<_, _>>()?;
    let handled = arith::encode(&mut emitter, mnemonic, &operands)?
        || transfer::encode(&mut emitter, mnemonic, &operands)?
        || misc::encode(&mut emitter, mnemonic, &operands, context)?;
    if !handled {
        return Err(format!("unknown instruction: {mnemonic}"));
    }
    Ok(emitter.finish())
}

fn expect_count(operands: &[Operand], count: usize, mnemonic: &str) -> Result<(), String> {
    if operands.len() != count {
        return Err(format!("{mnemonic} needs {count} operand(s)"));
    }
    Ok(())
}
