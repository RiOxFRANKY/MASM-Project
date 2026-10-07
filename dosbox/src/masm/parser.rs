use super::diagnostics::Diagnostic;
use super::expr::{parse_data_items, parse_expression, split_commas, Expr};
use super::lexer::{tokenize, Token};
use super::statement::{Line, Model, SimpleSegment, Stmt};
use super::types::TypeKind;

const PREFIXES: [(&str, u8); 6] = [("REP", 0xF3), ("REPE", 0xF3), ("REPZ", 0xF3), ("REPNE", 0xF2), ("REPNZ", 0xF2), ("LOCK", 0xF0)];

const IGNORED: [&str; 22] = [
    "ASSUME", "PUBLIC", "TITLE", "SUBTTL", "PAGE", "NAME", "DOSSEG", ".DOSSEG", ".8086", ".186", ".286", ".286C", ".386",
    ".8087", ".287", ".387", ".LIST", ".NOLIST", ".XLIST", ".LALL", ".SALL", ".XALL",
];

const UNSUPPORTED: [&str; 14] = [
    "EXTRN", "EXTERN", "INCLUDE", "INCLUDELIB", "MACRO", "STRUC", "STRUCT", "IF", "IFDEF", "IFNDEF", "RECORD", ".RADIX",
    "INVOKE", "PROTO",
];

pub struct ParsedSource {
    pub lines: Vec<Line>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse_source(source: &str) -> ParsedSource {
    let mut parser = SourceParser { lines: Vec::new(), diagnostics: Vec::new(), model: None, comment: None };
    for (index, text) in source.lines().enumerate() {
        let number = index + 1;
        if let Some(delimiter) = parser.comment {
            if text.contains(delimiter) {
                parser.comment = None;
            }
            continue;
        }
        let tokens = match tokenize(text) {
            Ok(tokens) => tokens,
            Err(message) => {
                parser.diagnostics.push(Diagnostic::error(number, message));
                continue;
            }
        };
        if tokens.first().is_some_and(|token| token.is_ident("COMMENT")) {
            let rest = text.trim_start()[7..].trim_start();
            if let Some(delimiter) = rest.chars().next() {
                if !rest[delimiter.len_utf8()..].contains(delimiter) {
                    parser.comment = Some(delimiter);
                }
            }
            continue;
        }
        if let Err(message) = parser.line(number, &tokens) {
            parser.diagnostics.push(Diagnostic::error(number, message));
        }
        if parser.lines.last().is_some_and(|line| matches!(line.stmt, Stmt::End(_))) {
            break;
        }
    }
    ParsedSource { lines: parser.lines, diagnostics: parser.diagnostics }
}

struct SourceParser {
    lines: Vec<Line>,
    diagnostics: Vec<Diagnostic>,
    model: Option<Model>,
    comment: Option<char>,
}

fn is_data_directive(name: &str) -> bool {
    matches!(
        name,
        "DB" | "DW" | "DD" | "DQ" | "DT" | "DF" | "BYTE" | "WORD" | "DWORD" | "SBYTE" | "SWORD" | "SDWORD" | "QWORD" | "TBYTE" | "FWORD"
    )
}

impl SourceParser {
    fn push(&mut self, number: usize, stmt: Stmt) {
        self.lines.push(Line { number, stmt });
    }

    fn line(&mut self, number: usize, tokens: &[Token]) -> Result<(), String> {
        let mut tokens = tokens;
        if tokens.len() >= 2 && tokens[1].is_punct(':') {
            if let Some(name) = tokens[0].ident() {
                if super::registers::lookup(name).is_none() {
                    self.push(number, Stmt::Label(name.to_string()));
                    tokens = &tokens[2..];
                    if tokens.first().is_some_and(|token| token.is_punct(':')) {
                        tokens = &tokens[1..];
                    }
                }
            }
        }
        let Some(first) = tokens.first() else {
            return Ok(());
        };
        let Some(first_name) = first.ident() else {
            return Err("syntax error".to_string());
        };
        if let Some(second) = tokens.get(1) {
            if second.is_punct('=') {
                let value = parse_expression(&tokens[2..])?;
                self.push(number, Stmt::Assign { name: first_name.to_string(), value });
                return Ok(());
            }
            if let Some(directive) = second.ident() {
                if let Some(stmt) = self.named_directive(first_name, directive, &tokens[2..])? {
                    self.push(number, stmt);
                    return Ok(());
                }
            }
        }
        self.statement(number, first_name, &tokens[1..])
    }

    fn named_directive(&mut self, name: &str, directive: &str, rest: &[Token]) -> Result<Option<Stmt>, String> {
        let name = name.to_string();
        if rest.first().is_some_and(|token| token.is_ident("PTR")) {
            return Ok(None);
        }
        let stmt = match directive {
            _ if is_data_directive(directive) => {
                let kind = TypeKind::from_name(directive).unwrap_or(TypeKind::Byte);
                Stmt::Data { name: Some(name), kind, items: parse_data_items(rest)? }
            }
            "EQU" | "TEXTEQU" => {
                let value = if rest.first().is_some_and(|token| token.is_punct('<')) && rest.last().is_some_and(|token| token.is_punct('>')) {
                    parse_expression(&rest[1..rest.len() - 1])?
                } else {
                    parse_expression(rest)?
                };
                Stmt::Equ { name, value }
            }
            "SEGMENT" => parse_segment(name, rest)?,
            "ENDS" => Stmt::Ends(name),
            "GROUP" => {
                let members = split_commas(rest)
                    .into_iter()
                    .map(|part| part.first().and_then(Token::ident).map(str::to_string).ok_or("segment name expected"))
                    .collect::<Result<Vec<_>, _>>()?;
                Stmt::Group { name, members }
            }
            "PROC" => {
                let distance = rest.first().and_then(Token::ident).and_then(TypeKind::from_name).filter(|kind| kind.is_code());
                Stmt::Proc { name, distance }
            }
            "ENDP" => Stmt::Endp(name),
            "LABEL" => {
                let kind_name = rest.first().and_then(Token::ident).ok_or("type expected after LABEL")?;
                let kind = TypeKind::from_name(kind_name).ok_or_else(|| format!("unknown type {kind_name}"))?;
                Stmt::LabelDirective { name, kind }
            }
            "MACRO" | "STRUC" | "STRUCT" | "RECORD" => return Err(format!("{directive} is not supported")),
            _ => return Ok(None),
        };
        Ok(Some(stmt))
    }

    fn statement(&mut self, number: usize, name: &str, rest: &[Token]) -> Result<(), String> {
        if IGNORED.contains(&name) {
            return Ok(());
        }
        if UNSUPPORTED.contains(&name) {
            return Err(format!("{name} is not supported by this assembler"));
        }
        let stmt = match name {
            _ if is_data_directive(name) => {
                let kind = TypeKind::from_name(name).unwrap_or(TypeKind::Byte);
                Stmt::Data { name: None, kind, items: parse_data_items(rest)? }
            }
            ".MODEL" => {
                let model_name = rest.first().and_then(Token::ident).ok_or("memory model expected")?;
                let model = match model_name {
                    "TINY" => Model::Tiny,
                    "SMALL" => Model::Small,
                    "MEDIUM" => Model::Medium,
                    "COMPACT" => Model::Compact,
                    "LARGE" | "HUGE" => Model::Large,
                    _ => return Err(format!("unsupported memory model {model_name}")),
                };
                self.model = Some(model);
                Stmt::Model(model)
            }
            ".STACK" => Stmt::Stack(if rest.is_empty() { None } else { Some(parse_expression(rest)?) }),
            ".CODE" => Stmt::Simple(SimpleSegment::Code),
            ".DATA" => Stmt::Simple(SimpleSegment::Data),
            ".DATA?" => Stmt::Simple(SimpleSegment::Bss),
            ".CONST" => Stmt::Simple(SimpleSegment::Const),
            ".STARTUP" => return self.startup(number),
            ".EXIT" => return self.exit(number, rest),
            "ORG" => Stmt::Org(parse_expression(rest)?),
            "EVEN" => Stmt::Align(2),
            "ALIGN" => {
                let value = match rest.first() {
                    Some(Token::Number(value)) => *value as u32,
                    None => 2,
                    _ => return Err("ALIGN needs a number".to_string()),
                };
                Stmt::Align(value.max(1))
            }
            "END" => Stmt::End(if rest.is_empty() { None } else { Some(parse_expression(rest)?) }),
            "ENDS" => Stmt::Ends(String::new()),
            _ => return self.instruction(number, name, rest),
        };
        self.push(number, stmt);
        Ok(())
    }

    fn instruction(&mut self, number: usize, name: &str, rest: &[Token]) -> Result<(), String> {
        let mut prefixes = Vec::new();
        let mut mnemonic = name.to_string();
        let mut rest = rest;
        while let Some((_, code)) = PREFIXES.iter().find(|(prefix, _)| *prefix == mnemonic) {
            prefixes.push(*code);
            match rest.first().and_then(Token::ident) {
                Some(next) => {
                    mnemonic = next.to_string();
                    rest = &rest[1..];
                }
                None => {
                    mnemonic = String::new();
                    break;
                }
            }
        }
        let operands = split_commas(rest).into_iter().map(parse_expression).collect::<Result<Vec<Expr>, String>>()?;
        self.push(number, Stmt::Instruction { prefixes, mnemonic, operands });
        Ok(())
    }

    fn expand(&mut self, number: usize, lines: &[&str]) -> Result<(), String> {
        for text in lines {
            let tokens = tokenize(text)?;
            self.line(number, &tokens)?;
        }
        Ok(())
    }

    fn startup(&mut self, number: usize) -> Result<(), String> {
        if self.model == Some(Model::Tiny) {
            self.expand(number, &["ORG 100h", "@STARTUP:"])
        } else {
            self.expand(number, &["@STARTUP:", "MOV AX,DGROUP", "MOV DS,AX"])
        }
    }

    fn exit(&mut self, number: usize, rest: &[Token]) -> Result<(), String> {
        if !rest.is_empty() {
            let value = parse_expression(rest)?;
            self.push(
                number,
                Stmt::Instruction {
                    prefixes: Vec::new(),
                    mnemonic: "MOV".to_string(),
                    operands: vec![Expr::Register(super::registers::Register::Byte(0)), value],
                },
            );
        }
        let lines: &[&str] = if rest.is_empty() { &["MOV AX,4C00h", "INT 21h"] } else { &["MOV AH,4Ch", "INT 21h"] };
        self.expand(number, lines)
    }
}

fn parse_segment(name: String, rest: &[Token]) -> Result<Stmt, String> {
    let mut align = 16;
    let mut stack = false;
    let mut class = String::new();
    for token in rest {
        match token {
            Token::Ident(option) => match option.as_str() {
                "BYTE" => align = 1,
                "WORD" => align = 2,
                "DWORD" => align = 4,
                "PARA" => align = 16,
                "PAGE" => align = 256,
                "STACK" => stack = true,
                "PUBLIC" | "COMMON" | "PRIVATE" | "MEMORY" | "USE16" => {}
                "AT" => return Err("AT segments are not supported".to_string()),
                other => return Err(format!("unknown segment option {other}")),
            },
            Token::Str(bytes) => class = String::from_utf8_lossy(bytes).to_ascii_uppercase(),
            Token::Number(_) => {}
            _ => return Err("syntax error in SEGMENT".to_string()),
        }
    }
    if class == "STACK" {
        stack = true;
    }
    Ok(Stmt::Segment { name, align, stack, class })
}
