use std::{iter::Peekable, str::CharIndices};

use crate::util::Info;

#[derive(Debug, PartialEq)]
pub enum TokenKind<'a> {
    String(&'a str),
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
    input: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

pub enum LexErrorKind {
    InvalidToken,
    UnclosedStringLiteral,
}

pub struct LexError<'a> {
    input: &'a str,
    kind: LexErrorKind,
    index: usize,
}

impl<'a> std::fmt::Display for LexError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            LexErrorKind::InvalidToken => {
                writeln!(f, "{}", self.input)?;
                write!(f, "{:>width$}^ ", "", width = self.index)?;
                write!(f, "invalid token")
            }
            LexErrorKind::UnclosedStringLiteral => {
                writeln!(f, "{}", self.input)?;
                write!(f, "{:>width$}^ ", "", width = self.index)?;
                write!(f, "unclosed string literal")
            }
        }
    }
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
        }
    }
    pub fn input(&self) -> &'a str {
        self.input
    }

    fn punctuation_len(&self, index: usize) -> Option<usize> {
        ["==", "!=", "<=", ">="]
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
                        info: Info { index },
                    });
                }
                '"' => {
                    let mut length = 0;
                    while let Some((_, c)) = self.chars.peek()
                        && *c != '"'
                    {
                        if matches!(c, '\n' | '\0') {
                            return Err(self.err_unclosed_string_literal(index));
                        }
                        self.chars.next();
                        length += 1;
                    }
                    let Some((_, '"')) = self.chars.next() else {
                        return Err(self.err_unclosed_string_literal(index));
                    };

                    let string = &self.input[index + 1..index + 1 + length];
                    tokens.push(Token {
                        kind: TokenKind::String(string),
                        info: Info { index },
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
                        info: Info { index },
                    });
                }
                _ => {
                    if let Some(length) = self.punctuation_len(index) {
                        for _ in 1..length {
                            self.chars.next();
                        }

                        tokens.push(Token {
                            kind: TokenKind::Punctuation(&self.input[index..index + length]),
                            info: Info { index },
                        });
                    } else {
                        return Err(self.err_invalid_token(index));
                    }
                }
            }
        }

        tokens.push(Token {
            kind: TokenKind::EOF,
            info: Info {
                index: self.input().len(),
            },
        });

        Ok(tokens)
    }

    fn is_identifier_head(c: char) -> bool {
        matches!(c, 'a'..='z' | 'A'..='Z' | '_')
    }

    fn is_valid_identifier_tail(c: char) -> bool {
        Lexer::is_identifier_head(c) || matches!(c, '0'..'9')
    }

    fn is_keyword(s: &str) -> bool {
        [
            "return", "if", "else", "for", "while", "int", "sizeof", "char",
        ]
        .iter()
        .any(|&keyword| s == keyword)
    }

    fn err_unclosed_string_literal(self, index: usize) -> LexError<'a> {
        LexError {
            input: self.input,
            kind: LexErrorKind::UnclosedStringLiteral,
            index,
        }
    }

    fn err_invalid_token(self, index: usize) -> LexError<'a> {
        LexError {
            input: self.input,
            kind: LexErrorKind::InvalidToken,
            index,
        }
    }
}
