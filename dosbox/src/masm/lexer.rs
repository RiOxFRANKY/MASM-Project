#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Ident(String),
    Number(i64),
    Str(Vec<u8>),
    Punct(char),
}

impl Token {
    pub fn is_ident(&self, name: &str) -> bool {
        matches!(self, Token::Ident(text) if text == name)
    }

    pub fn is_punct(&self, symbol: char) -> bool {
        matches!(self, Token::Punct(found) if *found == symbol)
    }

    pub fn ident(&self) -> Option<&str> {
        match self {
            Token::Ident(text) => Some(text),
            _ => None,
        }
    }
}

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || matches!(character, '_' | '@' | '.' | '$' | '?')
}

fn is_identifier_part(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '@' | '$' | '?')
}

pub fn parse_number(text: &str) -> Result<i64, String> {
    let upper = text.to_ascii_uppercase();
    let last = upper.chars().last().unwrap_or('0');
    let (digits, radix) = match last {
        'H' => (&upper[..upper.len() - 1], 16),
        'B' | 'Y' if upper[..upper.len() - 1].chars().all(|digit| digit == '0' || digit == '1') => {
            (&upper[..upper.len() - 1], 2)
        }
        'O' | 'Q' => (&upper[..upper.len() - 1], 8),
        'D' | 'T' if upper[..upper.len() - 1].chars().all(|digit| digit.is_ascii_digit()) => {
            (&upper[..upper.len() - 1], 10)
        }
        _ => (upper.as_str(), 10),
    };
    if digits.is_empty() {
        return Err(format!("invalid number: {text}"));
    }
    i64::from_str_radix(digits, radix).map_err(|_| format!("invalid number: {text}"))
}

pub fn strip_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    for (index, character) in line.char_indices() {
        match quote {
            Some(open) if character == open => quote = None,
            Some(_) => {}
            None if character == '\'' || character == '"' => quote = Some(character),
            None if character == ';' => return &line[..index],
            None => {}
        }
    }
    line
}

pub fn tokenize(line: &str) -> Result<Vec<Token>, String> {
    let characters: Vec<char> = strip_comment(line).chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character.is_whitespace() {
            index += 1;
        } else if character.is_ascii_digit() {
            let start = index;
            while index < characters.len() && characters[index].is_ascii_alphanumeric() {
                index += 1;
            }
            let text: String = characters[start..index].iter().collect();
            tokens.push(Token::Number(parse_number(&text)?));
        } else if character == '\'' || character == '"' {
            let mut bytes = Vec::new();
            index += 1;
            loop {
                match characters.get(index) {
                    None => return Err("unterminated string".to_string()),
                    Some(found) if *found == character => {
                        if characters.get(index + 1) == Some(&character) {
                            bytes.push(character as u8);
                            index += 2;
                        } else {
                            index += 1;
                            break;
                        }
                    }
                    Some(found) => {
                        bytes.push(crate::terminal::cp437::from_unicode(*found));
                        index += 1;
                    }
                }
            }
            tokens.push(Token::Str(bytes));
        } else if character == '?' && !characters.get(index + 1).is_some_and(|next| is_identifier_part(*next)) {
            tokens.push(Token::Punct('?'));
            index += 1;
        } else if is_identifier_start(character) {
            let start = index;
            index += 1;
            while index < characters.len() && is_identifier_part(characters[index]) {
                index += 1;
            }
            if character == '.' && index == start + 1 {
                tokens.push(Token::Punct('.'));
                continue;
            }
            let text: String = characters[start..index].iter().collect();
            tokens.push(Token::Ident(text.to_ascii_uppercase()));
        } else if "+-*/()[],:=<>&!".contains(character) {
            tokens.push(Token::Punct(character));
            index += 1;
        } else {
            return Err(format!("unexpected character '{character}'"));
        }
    }
    Ok(tokens)
}
