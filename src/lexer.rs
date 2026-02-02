use std::{iter::Peekable, str::CharIndices};

#[derive(Debug)]
pub enum TokenKind {
    Punctuation(char),
    Numeric(i64),
    EOF,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Punctuation(c) => write!(f, "{c}"),
            TokenKind::Numeric(n) => write!(f, "{n}"),
            TokenKind::EOF => write!(f, "End Of File"),
        }
    }
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub index: usize,
    pub length: usize,
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

pub enum LexErrorKind {
    InvalidToken { index: usize },
}

pub struct LexError<'a> {
    input: &'a str,
    kind: LexErrorKind,
}

impl<'a> LexError<'a> {
    pub fn new(input: &'a str, kind: LexErrorKind) -> Self {
        Self { input, kind }
    }
}

impl<'a> std::fmt::Display for LexError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            LexErrorKind::InvalidToken { index } => {
                writeln!(f, "{}", self.input)?;
                writeln!(f, "{:>width$}^", "", width = index)?;
                write!(f, "invalid token")
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

    pub fn err_invalid_token(self, index: usize) -> LexError<'a> {
        LexError::new(self.input, LexErrorKind::InvalidToken { index })
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError<'a>> {
        let mut tokens = vec![];
        while let Some((index, c)) = self.chars.next() {
            match c {
                c if c.is_whitespace() => continue,
                c if c.is_ascii_digit() => {
                    let mut num = format!("{c}");
                    let mut length = 0;

                    while let Some((_, n)) = self.chars.peek()
                        && n.is_ascii_digit()
                    {
                        num.push(*n);
                        self.chars.next();
                        length += 1;
                    }

                    tokens.push(Token {
                        kind: TokenKind::Numeric(num.parse().expect("should parse as a number")),
                        index,
                        length,
                    });
                }
                c if c.is_ascii_punctuation() => {
                    tokens.push(Token {
                        kind: TokenKind::Punctuation(c),
                        index,
                        length: 1,
                    });
                }
                _ => {
                    return Err(self.err_invalid_token(index));
                }
            }
        }

        tokens.push(Token {
            kind: TokenKind::EOF,
            index: self.input().len(),
            length: 0,
        });

        Ok(tokens)
    }
}
