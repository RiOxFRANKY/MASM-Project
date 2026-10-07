use crate::formats::object::ObjectFile;
use std::fmt::Write;

pub fn render(object: &ObjectFile, entry: (u16, u16)) -> String {
    let mut text = String::new();
    let _ = writeln!(text, " Start  Stop   Length Name               Class      Group");
    for segment in &object.segments {
        let stop = (segment.start + segment.length).saturating_sub(1).max(segment.start);
        let _ = writeln!(
            text,
            " {:05X}H {:05X}H {:05X}H {:<18} {:<10} {}",
            segment.start, stop, segment.length, segment.name, segment.class, segment.group
        );
    }
    let _ = writeln!(text);
    let _ = writeln!(text, "  Address         Symbol");
    for symbol in &object.symbols {
        let _ = writeln!(text, "  {:04X}:{:04X}       {}", symbol.segment, symbol.offset, symbol.name);
    }
    let _ = writeln!(text);
    let _ = writeln!(text, "Program entry point at {:04X}:{:04X}", entry.0, entry.1);
    text
}
