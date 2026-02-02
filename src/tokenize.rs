use crate::util;

#[derive(Debug)]
pub enum Kind {
    Punctuation(char),
    Numeric(i64),
    EOF,
}

#[derive(Debug)]
pub struct Token {
    pub kind: Kind,
    pub start_byte_location: usize,
    pub length: usize,
}

pub fn tokenize(program: &str) -> Vec<Token> {
    let mut tokens = vec![];

    let mut program = program.char_indices().peekable();
    while let Some((start_byte_location, c)) = program.next() {
        match c {
            c if c.is_whitespace() => continue,
            c if c.is_ascii_digit() => {
                let mut num = format!("{c}");
                let mut length = 0;
                while let Some((_, n)) = program.peek()
                    && n.is_ascii_digit()
                {
                    num.push(*n);
                    program.next();
                    length += 1;
                }
                let num = num.parse().expect("should be parsed as a number");
                tokens.push(Token {
                    kind: Kind::Numeric(num),
                    start_byte_location,
                    length,
                });
            }
            '+' | '-' => {
                tokens.push(Token {
                    kind: Kind::Punctuation(c),
                    start_byte_location,
                    length: 1,
                });
            }
            _ => util::errx(&format!("invalid token: `{c}'")),
        }
    }

    tokens.push(Token {
        kind: Kind::EOF,
        start_byte_location: 0,
        length: 0,
    });

    tokens
}
