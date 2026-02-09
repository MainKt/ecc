use std::{iter::Peekable, str::CharIndices};

use crate::util::{self, Info};

#[derive(Debug, PartialEq)]
pub enum TokenKind<'a> {
    String(String),
    Identifier(&'a str),
    Keyword(&'a str),
    Punctuation(&'a str),
    Numeric(usize),
    EOF,
}

impl<'a> std::fmt::Display for TokenKind<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Punctuation(c) => write!(f, "{c}"),
            TokenKind::Numeric(n) => write!(f, "{n}"),
            TokenKind::EOF => write!(f, "End Of File"),
            TokenKind::Identifier(name) => write!(f, "{name}"),
            TokenKind::Keyword(keyword) => write!(f, "{keyword}"),
            TokenKind::String(string) => write!(f, "{string}"),
        }
    }
}

#[derive(Debug)]
pub struct Token<'a> {
    pub kind: TokenKind<'a>,
    pub info: Info,
}

pub struct Lexer<'a> {
    file: &'a str,
    input: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

pub enum LexErrorKind {
    InvalidToken,
    UnclosedStringLiteral,
    UnclosedBlockComment,
    InvalidHexEscapeSequence,
}

pub struct LexError<'a> {
    file: &'a str,
    input: &'a str,
    kind: LexErrorKind,
    info: Info,
}

impl<'a> std::fmt::Display for LexError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        util::info_msg(f, self.file, self.input, self.info)?;
        match self.kind {
            LexErrorKind::InvalidToken => write!(f, "invalid token"),
            LexErrorKind::UnclosedBlockComment => write!(f, "unclosed block comment"),
            LexErrorKind::UnclosedStringLiteral => write!(f, "unclosed string literal"),
            LexErrorKind::InvalidHexEscapeSequence => write!(f, "invalid hex escape sequence"),
        }
    }
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a str, input: &'a str) -> Self {
        Self {
            file,
            input,
            chars: input.char_indices().peekable(),
        }
    }
    pub fn input(&self) -> &'a str {
        self.input
    }

    fn punctuation_len(&self, index: usize) -> Option<usize> {
        ["==", "!=", "<=", ">=", "->"]
            .iter()
            .find_map(|p| self.input[index..].starts_with(p).then_some(p.len()))
            .or_else(|| {
                self.input[index..]
                    .starts_with(|c: char| c.is_ascii_punctuation())
                    .then_some(1)
            })
    }

    pub fn tokenize(mut self) -> Result<Vec<Token<'a>>, LexError<'a>> {
        let mut tokens = vec![];
        while let Some((index, c)) = self.chars.next() {
            match c {
                c if c.is_whitespace() => continue,
                c if c.is_ascii_digit() => {
                    let mut num = format!("{c}");

                    while let Some((_, n)) = self.chars.peek()
                        && n.is_ascii_digit()
                    {
                        num.push(*n);
                        self.chars.next();
                    }

                    tokens.push(Token {
                        kind: TokenKind::Numeric(num.parse().expect("should parse as a number")),
                        info: Info {
                            index,
                            line: self.line_number(index),
                        },
                    });
                }
                '/' if matches!(self.chars.peek(), Some((_, '/'))) => {
                    self.chars.next();
                    while let Some((_, c)) = self.chars.next() {
                        if c == '\n' {
                            break;
                        }
                    }
                }
                '/' if matches!(self.chars.peek(), Some((_, '*'))) => {
                    let mut is_closed = false;
                    while let Some((_, c)) = self.chars.next() {
                        if c == '*' && matches!(self.chars.peek(), Some((_, '/'))) {
                            self.chars.next();
                            is_closed = true;
                            break;
                        }
                    }
                    if !is_closed {
                        return Err(self.emit_error(
                            LexErrorKind::UnclosedBlockComment,
                            Info {
                                index,
                                line: self.line_number(index),
                            },
                        ));
                    }
                }
                '"' => {
                    let info = Info {
                        index,
                        line: self.line_number(index),
                    };
                    let mut length = 0;
                    while let Some((_, c)) = self.chars.peek()
                        && *c != '"'
                    {
                        if matches!(c, '\n' | '\0') {
                            return Err(self.emit_error(LexErrorKind::UnclosedStringLiteral, info));
                        }
                        self.chars.next();
                        length += 1;
                    }
                    let Some((_, '"')) = self.chars.next() else {
                        return Err(self.emit_error(LexErrorKind::UnclosedStringLiteral, info));
                    };

                    let escaped = self
                        .to_escaped_string(&self.input[index + 1..index + 1 + length], index + 1)?;
                    tokens.push(Token {
                        kind: TokenKind::String(escaped),
                        info: Info {
                            index,
                            line: self.line_number(index),
                        },
                    });
                }
                _ if Lexer::is_identifier_head(c) => {
                    let mut length = 1;
                    while let Some((_, i)) = self.chars.peek()
                        && Lexer::is_valid_identifier_tail(*i)
                    {
                        self.chars.next();
                        length += 1;
                    }

                    let input = &self.input[index..index + length];
                    let kind = if Lexer::is_keyword(input) {
                        TokenKind::Keyword(input)
                    } else {
                        TokenKind::Identifier(input)
                    };

                    tokens.push(Token {
                        kind,
                        info: Info {
                            index,
                            line: self.line_number(index),
                        },
                    });
                }
                _ => {
                    let info = Info {
                        index,
                        line: self.line_number(index),
                    };

                    if let Some(length) = self.punctuation_len(index) {
                        for _ in 1..length {
                            self.chars.next();
                        }

                        tokens.push(Token {
                            kind: TokenKind::Punctuation(&self.input[index..index + length]),
                            info,
                        });
                    } else {
                        return Err(self.emit_error(LexErrorKind::InvalidToken, info));
                    }
                }
            }
        }

        tokens.push(Token {
            kind: TokenKind::EOF,
            info: Info {
                index: self.input().len(),
                line: self.line_number(self.input().len()),
            },
        });

        Ok(tokens)
    }

    fn is_identifier_head(c: char) -> bool {
        matches!(c, 'a'..='z' | 'A'..='Z' | '_')
    }

    fn get_escaped_char(c: char) -> char {
        match c {
            'a' => '\x07',
            'b' => '\x08',
            't' => '\t',
            'n' => '\n',
            'v' => '\x0b',
            'f' => '\x0c',
            'r' => '\r',
            'e' => '\x1b',
            c => c,
        }
    }

    fn read_hex_seq(mut hex: u32, chars: &mut Peekable<CharIndices>) -> u32 {
        while let Some(&(_, h)) = chars.peek() {
            match h.to_digit(16) {
                Some(h) => {
                    hex = (hex << 4) + h;
                    chars.next();
                }
                None => break,
            }
        }
        hex
    }

    fn read_octal_seq(mut octal: u32, chars: &mut Peekable<CharIndices>) -> u32 {
        for _ in 0..2 {
            if let Some(&(_, o)) = chars.peek()
                && matches!(o, '0'..='7')
            {
                match o.to_digit(8) {
                    Some(o) => {
                        octal = (octal << 3) + o;
                        chars.next();
                    }
                    None => break,
                }
            }
        }
        octal
    }

    fn to_escaped_string(&self, s: &str, index: usize) -> Result<String, LexError<'a>> {
        let mut escaped = String::with_capacity(s.len());
        let mut chars = s.char_indices().peekable();

        while let Some((_, c)) = chars.next() {
            match c {
                '\\' => {
                    if let Some((i, c)) = chars.next() {
                        escaped.push(match c {
                            o @ '0'..='7' => {
                                Lexer::read_octal_seq(o as u32 - '0' as u32, &mut chars) as u8
                                    as char
                            }
                            'x' => {
                                let Some(hex) = chars.next().and_then(|(_, c)| c.to_digit(16))
                                else {
                                    return Err(self.emit_error(
                                        LexErrorKind::InvalidHexEscapeSequence,
                                        Info {
                                            index: index + i,
                                            line: self.line_number(index + i),
                                        },
                                    ));
                                };
                                Lexer::read_hex_seq(hex, &mut chars) as u8 as char
                            }
                            c => Lexer::get_escaped_char(c),
                        });
                    } else {
                        escaped.push('\\');
                    }
                }
                c => escaped.push(c),
            }
        }
        Ok(escaped)
    }

    fn is_valid_identifier_tail(c: char) -> bool {
        Lexer::is_identifier_head(c) || matches!(c, '0'..'9')
    }

    fn is_keyword(s: &str) -> bool {
        [
            "return", "if", "else", "for", "while", "int", "sizeof", "char", "struct", "union",
            "short", "long", "void",
        ]
        .iter()
        .any(|&keyword| s == keyword)
    }

    fn line_number(&self, index: usize) -> usize {
        self.input[..index].matches('\n').count() + 1
    }

    fn emit_error(&self, kind: LexErrorKind, info: Info) -> LexError<'a> {
        LexError {
            file: self.file,
            input: self.input,
            kind,
            info,
        }
    }
}
