use std::{iter::Peekable, str::CharIndices};

use crate::util;

#[derive(Debug)]
pub enum Kind {
    Punctuation(char),
    Numeric(i64),
}

#[derive(Debug)]
pub struct Token {
    pub kind: Kind,
    pub index: usize,
    pub length: usize,
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
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

                    return Some(Token {
                        kind: Kind::Numeric(num.parse().expect("should parse as a number")),
                        index,
                        length,
                    });
                }
                '+' | '-' => {
                    return Some(Token {
                        kind: Kind::Punctuation(c),
                        index,
                        length: 1,
                    });
                }
                _ => util::err_at("invalid token", self.input, index),
            }
        }

        None
    }
}
