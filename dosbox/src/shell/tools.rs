use super::{Quit, Shell};
use crate::formats::object::ObjectFile;
use crate::linker::{self, OutputKind};
use crate::masm::{self, diagnostics::Severity};
use std::fs;

struct Fields {
    names: Vec<String>,
    switches: Vec<String>,
}

fn parse_fields(arguments: &str) -> Fields {
    let mut switches = Vec::new();
    let mut cleaned = String::new();
    let mut rest = arguments.trim();
    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix('/') {
            let end = stripped.find(|character: char| character.is_whitespace() || "/,;".contains(character)).unwrap_or(stripped.len());
            switches.push(stripped[..end].to_ascii_uppercase());
            rest = &stripped[end..];
        } else {
            let character = rest.chars().next().unwrap_or(' ');
            cleaned.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    let cleaned = cleaned.split(';').next().unwrap_or("").to_string();
    let names = cleaned.split(',').map(|field| field.trim().to_string()).collect();
    Fields { names, switches }
}

fn with_extension(name: &str, extension: &str) -> String {
    let file_part = name.rsplit(['\\', '/', ':']).next().unwrap_or(name);
    if file_part.contains('.') {
        name.to_ascii_uppercase()
    } else {
        format!("{}.{}", name.to_ascii_uppercase(), extension)
    }
}

fn base_name(name: &str) -> String {
    let upper = name.to_ascii_uppercase();
    match upper.rfind('.') {
        Some(position) if !upper[position..].contains('\\') => upper[..position].to_string(),
        _ => upper,
    }
}

impl Shell {
    pub(super) fn masm(&mut self, arguments: &str) -> Result<(), Quit> {
        let fields = parse_fields(arguments);
        let source_field = fields.names.first().cloned().unwrap_or_default();
        if source_field.is_empty() {
            self.println("Usage: MASM source[.ASM][,object[.OBJ]][;]");
            return Ok(());
        }
        let source_name = with_extension(&source_field, "ASM");
        let object_name = match fields.names.get(1).filter(|name| !name.is_empty()) {
            Some(name) => with_extension(name, "OBJ"),
            None => format!("{}.OBJ", base_name(&source_name)),
        };
        self.println("RustMASM 8086 Assembler Version 1.0");
        self.println("");
        let source = match self.machine.drive.resolve(&source_name).ok().and_then(|path| fs::read(path).ok()) {
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(error) => error
                    .into_bytes()
                    .iter()
                    .map(|byte| if *byte < 0x80 { *byte as char } else { crate::terminal::cp437::to_unicode(*byte) })
                    .collect(),
            },
            None => {
                self.println(&format!("Unable to open input file: {source_name}"));
                return Ok(());
            }
        };
        let display_name = source_name.rsplit('\\').next().unwrap_or(&source_name).to_string();
        let assembly = masm::assemble(&source, &display_name);
        for diagnostic in &assembly.diagnostics {
            let text = diagnostic.render(&display_name);
            self.println(&text);
        }
        if let Some(object) = &assembly.object {
            match self.machine.drive.resolve(&object_name).map(|path| fs::write(path, object.to_bytes())) {
                Ok(Ok(())) => {}
                _ => self.println(&format!("Unable to write object file: {object_name}")),
            }
        }
        self.println("");
        self.println(&format!("{:>7} Source  Lines", assembly.source_lines));
        self.println(&format!("{:>7} Warning Errors", assembly.count(Severity::Warning)));
        self.println(&format!("{:>7} Severe  Errors", assembly.count(Severity::Error)));
        Ok(())
    }

    pub(super) fn link(&mut self, arguments: &str) -> Result<(), Quit> {
        let fields = parse_fields(arguments);
        let objects: Vec<String> = fields
            .names
            .first()
            .map(|field| field.split('+').map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect())
            .unwrap_or_default();
        if objects.is_empty() {
            self.println("Usage: LINK object[.OBJ][,exe][,map][;] [/T] [/M]");
            return Ok(());
        }
        self.println("RustLINK Linker Version 1.0");
        self.println("");
        if objects.len() > 1 {
            self.println("error: only one object module per program is supported");
            return Ok(());
        }
        let com = fields.switches.iter().any(|switch| switch.starts_with('T'));
        let object_name = with_extension(&objects[0], "OBJ");
        let extension = if com { "COM" } else { "EXE" };
        let output_name = match fields.names.get(1).filter(|name| !name.is_empty()) {
            Some(name) => with_extension(name, extension),
            None => format!("{}.{extension}", base_name(&object_name)),
        };
        let map_name = match fields.names.get(2).filter(|name| !name.is_empty()) {
            Some(name) => Some(with_extension(name, "MAP")),
            None if fields.switches.iter().any(|switch| switch.starts_with('M')) => Some(format!("{}.MAP", base_name(&output_name))),
            None => None,
        };
        let object = match self.machine.drive.resolve(&object_name).ok().and_then(|path| fs::read(path).ok()) {
            Some(bytes) => ObjectFile::from_bytes(&bytes),
            None => Err(format!("cannot find file {object_name}")),
        };
        let result = object.and_then(|object| linker::link(&object, if com { OutputKind::Com } else { OutputKind::Exe }));
        let linked = match result {
            Ok(linked) => linked,
            Err(message) => {
                self.println(&format!("error: {message}"));
                return Ok(());
            }
        };
        for warning in &linked.warnings {
            self.println(&format!("warning: {warning}"));
        }
        let writes = [(Some(output_name.clone()), linked.bytes), (map_name, linked.map.into_bytes())];
        for (name, bytes) in writes {
            let Some(name) = name else { continue };
            if !matches!(self.machine.drive.resolve(&name).map(|path| fs::write(path, bytes)), Ok(Ok(()))) {
                self.println(&format!("error: cannot write {name}"));
                return Ok(());
            }
        }
        self.println(&format!("{output_name} created"));
        Ok(())
    }
}
