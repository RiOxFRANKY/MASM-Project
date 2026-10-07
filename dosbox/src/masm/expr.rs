use super::lexer::Token;
use super::registers::{lookup, Register};
use super::types::TypeKind;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unary {
    Negate,
    Not,
    Offset,
    Seg,
    Short,
    High,
    Low,
    Type,
    Length,
    Size,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binary {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shl,
    Shr,
    And,
    Or,
    Xor,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(i64),
    Str(Vec<u8>),
    Symbol(String),
    Register(Register),
    Here,
    Undefined,
    Unary(Unary, Box<Expr>),
    Binary(Binary, Box<Expr>, Box<Expr>),
    Memory(Box<Expr>),
    Ptr(TypeKind, Box<Expr>),
    Override(u8, Box<Expr>),
    Dup(Box<Expr>, Vec<Expr>),
}

pub fn split_commas(tokens: &[Token]) -> Vec<&[Token]> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Punct('(') | Token::Punct('[') | Token::Punct('<') => depth += 1,
            Token::Punct(')') | Token::Punct(']') | Token::Punct('>') => depth -= 1,
            Token::Punct(',') if depth == 0 => {
                parts.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < tokens.len() || !parts.is_empty() {
        parts.push(&tokens[start..]);
    }
    parts
}

pub fn parse_expression(tokens: &[Token]) -> Result<Expr, String> {
    let mut parser = Parser { tokens, position: 0 };
    let expression = parser.expression()?;
    parser.finish()?;
    Ok(expression)
}

pub fn parse_data_items(tokens: &[Token]) -> Result<Vec<Expr>, String> {
    split_commas(tokens)
        .into_iter()
        .map(|part| {
            let mut parser = Parser { tokens: part, position: 0 };
            let item = parser.data_item()?;
            parser.finish()?;
            Ok(item)
        })
        .collect()
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.position)
    }

    fn peek_at(&self, offset: usize) -> Option<&'a Token> {
        self.tokens.get(self.position + offset)
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.position);
        self.position += 1;
        token
    }

    fn peek_ident(&self) -> Option<&'a str> {
        self.peek().and_then(Token::ident)
    }

    fn expect(&mut self, symbol: char) -> Result<(), String> {
        match self.advance() {
            Some(token) if token.is_punct(symbol) => Ok(()),
            _ => Err(format!("expected '{symbol}'")),
        }
    }

    fn finish(&self) -> Result<(), String> {
        match self.peek() {
            None => Ok(()),
            Some(token) => Err(format!("unexpected {}", describe(token))),
        }
    }

    fn data_item(&mut self) -> Result<Expr, String> {
        if self.peek().is_some_and(|token| token.is_punct('?')) && self.peek_at(1).is_none() {
            self.advance();
            return Ok(Expr::Undefined);
        }
        let first = self.expression()?;
        if self.peek_ident() == Some("DUP") {
            self.advance();
            self.expect('(')?;
            let start = self.position;
            let mut depth = 1;
            while depth > 0 {
                match self.advance() {
                    Some(Token::Punct('(')) => depth += 1,
                    Some(Token::Punct(')')) => depth -= 1,
                    Some(_) => {}
                    None => return Err("missing ')' after DUP".to_string()),
                }
            }
            let inner = parse_data_items(&self.tokens[start..self.position - 1])?;
            return Ok(Expr::Dup(Box::new(first), inner));
        }
        Ok(first)
    }

    fn expression(&mut self) -> Result<Expr, String> {
        let mut left = self.and_expression()?;
        loop {
            let operator = match self.peek_ident() {
                Some("OR") => Binary::Or,
                Some("XOR") => Binary::Xor,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.and_expression()?;
            left = Expr::Binary(operator, Box::new(left), Box::new(right));
        }
    }

    fn and_expression(&mut self) -> Result<Expr, String> {
        let mut left = self.not_expression()?;
        while self.peek_ident() == Some("AND") {
            self.advance();
            let right = self.not_expression()?;
            left = Expr::Binary(Binary::And, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn not_expression(&mut self) -> Result<Expr, String> {
        if self.peek_ident() == Some("NOT") {
            self.advance();
            let operand = self.not_expression()?;
            return Ok(Expr::Unary(Unary::Not, Box::new(operand)));
        }
        self.relational()
    }

    fn relational(&mut self) -> Result<Expr, String> {
        let mut left = self.additive()?;
        loop {
            let operator = match self.peek_ident() {
                Some("EQ") => Binary::Eq,
                Some("NE") => Binary::Ne,
                Some("LT") => Binary::Lt,
                Some("LE") => Binary::Le,
                Some("GT") => Binary::Gt,
                Some("GE") => Binary::Ge,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.additive()?;
            left = Expr::Binary(operator, Box::new(left), Box::new(right));
        }
    }

    fn additive(&mut self) -> Result<Expr, String> {
        let mut left = self.multiplicative()?;
        loop {
            let operator = match self.peek() {
                Some(Token::Punct('+')) => Binary::Add,
                Some(Token::Punct('-')) => Binary::Sub,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.multiplicative()?;
            left = Expr::Binary(operator, Box::new(left), Box::new(right));
        }
    }

    fn multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            let operator = match self.peek() {
                Some(Token::Punct('*')) => Binary::Mul,
                Some(Token::Punct('/')) => Binary::Div,
                Some(Token::Ident(name)) if name == "MOD" => Binary::Mod,
                Some(Token::Ident(name)) if name == "SHL" => Binary::Shl,
                Some(Token::Ident(name)) if name == "SHR" => Binary::Shr,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.unary()?;
            left = Expr::Binary(operator, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Result<Expr, String> {
        let token = self.peek().ok_or("missing operand")?;
        if token.is_punct('-') {
            self.advance();
            return Ok(Expr::Unary(Unary::Negate, Box::new(self.unary()?)));
        }
        if token.is_punct('+') {
            self.advance();
            return self.unary();
        }
        if let Some(name) = token.ident() {
            if let Some(kind) = TypeKind::from_name(name) {
                if self.peek_at(1).is_some_and(|next| next.is_ident("PTR")) {
                    self.position += 2;
                    return Ok(Expr::Ptr(kind, Box::new(self.unary()?)));
                }
            }
            if let Some(Register::Segment(segment)) = lookup(name) {
                if self.peek_at(1).is_some_and(|next| next.is_punct(':')) {
                    self.position += 2;
                    return Ok(Expr::Override(segment, Box::new(self.unary()?)));
                }
            }
            let operator = match name {
                "OFFSET" => Some(Unary::Offset),
                "SEG" => Some(Unary::Seg),
                "SHORT" => Some(Unary::Short),
                "HIGH" => Some(Unary::High),
                "LOW" => Some(Unary::Low),
                "TYPE" => Some(Unary::Type),
                "LENGTH" | "LENGTHOF" => Some(Unary::Length),
                "SIZE" | "SIZEOF" => Some(Unary::Size),
                _ => None,
            };
            if let Some(operator) = operator {
                self.advance();
                return Ok(Expr::Unary(operator, Box::new(self.unary()?)));
            }
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let mut base = self.primary()?;
        while self.peek().is_some_and(|token| token.is_punct('[')) {
            let index = self.primary()?;
            base = Expr::Binary(Binary::Add, Box::new(base), Box::new(index));
        }
        Ok(base)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let token = self.advance().ok_or("missing operand")?;
        match token {
            Token::Number(value) => Ok(Expr::Number(*value)),
            Token::Str(bytes) => Ok(Expr::Str(bytes.clone())),
            Token::Punct('(') => {
                let inner = self.expression()?;
                self.expect(')')?;
                Ok(inner)
            }
            Token::Punct('[') => {
                let inner = self.expression()?;
                self.expect(']')?;
                Ok(Expr::Memory(Box::new(inner)))
            }
            Token::Punct('?') => Ok(Expr::Undefined),
            Token::Ident(name) if name == "$" => Ok(Expr::Here),
            Token::Ident(name) => match lookup(name) {
                Some(register) => Ok(Expr::Register(register)),
                None => Ok(Expr::Symbol(name.clone())),
            },
            other => Err(format!("unexpected {}", describe(other))),
        }
    }
}

fn describe(token: &Token) -> String {
    match token {
        Token::Ident(name) => format!("'{name}'"),
        Token::Number(value) => format!("number {value}"),
        Token::Str(_) => "string".to_string(),
        Token::Punct(symbol) => format!("'{symbol}'"),
    }
}
