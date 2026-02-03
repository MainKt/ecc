use std::{iter::Peekable, vec::IntoIter};

use crate::lexer::{Token, TokenKind};

pub enum Node {
    Add(Box<Node>, Box<Node>),
    Subtract(Box<Node>, Box<Node>),
    Multiply(Box<Node>, Box<Node>),
    Divide(Box<Node>, Box<Node>),
    Negative(Box<Node>),
    Equal(Box<Node>, Box<Node>),
    NotEqual(Box<Node>, Box<Node>),
    LessThan(Box<Node>, Box<Node>),
    LessThanEqual(Box<Node>, Box<Node>),
    Numeric(i64),
}

pub struct Parser<'a> {
    input: &'a str,
    tokens: Peekable<IntoIter<Token<'a>>>,
}

pub enum ParseErrorKind<'a> {
    ExtraToken {
        index: usize,
    },
    ExpectedExpression {
        index: usize,
    },
    UnexpectedToken {
        index: usize,
        expected: TokenKind<'a>,
    },
    UnusualEndOfTokens,
}

pub struct ParseError<'a> {
    input: &'a str,
    kind: ParseErrorKind<'a>,
}

impl<'a> std::fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ParseErrorKind::ExtraToken { index } => {
                writeln!(f, "{}", self.input)?;
                writeln!(f, "{:>width$}^", "", width = index)?;
                write!(f, "extra token")
            }
            ParseErrorKind::UnexpectedToken {
                ref expected,
                index,
            } => {
                writeln!(f, "{}", self.input)?;
                writeln!(f, "{:>width$}^", "", width = index)?;
                write!(f, "expected `{expected}'")
            }
            ParseErrorKind::ExpectedExpression { index } => {
                writeln!(f, "{}", self.input)?;
                writeln!(f, "{:>width$}^", "", width = index)?;
                write!(f, "expected an expression")
            }
            ParseErrorKind::UnusualEndOfTokens => {
                write!(f, "fatal error: Unusual end of tokens during parsing")
            }
        }
    }
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str, tokens: Vec<Token<'a>>) -> Self {
        Self {
            input,
            tokens: tokens.into_iter().peekable(),
        }
    }

    fn err_unusual_end_of_tokens(&self) -> ParseError<'a> {
        ParseError {
            input: "",
            kind: ParseErrorKind::UnusualEndOfTokens,
        }
    }

    fn err_expected_expression(&self, index: usize) -> ParseError<'a> {
        ParseError {
            input: self.input,
            kind: ParseErrorKind::ExpectedExpression { index },
        }
    }

    fn err_unexpected_token(&self, expected_kind: TokenKind<'a>, index: usize) -> ParseError<'a> {
        ParseError {
            input: self.input,
            kind: ParseErrorKind::UnexpectedToken {
                expected: expected_kind,
                index,
            },
        }
    }

    fn err_extra_token(&self, index: usize) -> ParseError<'a> {
        ParseError {
            input: self.input,
            kind: ParseErrorKind::ExtraToken { index },
        }
    }

    pub fn parse(mut self) -> Result<Node, ParseError<'a>> {
        let node = self.parse_expression()?;

        let Some(Token { kind, index, .. }) = self.tokens.next() else {
            return Err(self.err_unusual_end_of_tokens());
        };

        match kind {
            TokenKind::EOF => Ok(node),
            _ => Err(self.err_extra_token(index)),
        }
    }

    fn parse_relational(&mut self) -> Result<Node, ParseError<'a>> {
        todo!()
    }

    pub fn parse_equality(&mut self) -> Result<Node, ParseError<'a>> {
        let node = self.parse_relational()?;

        Ok(node)
    }

    fn parse_expression(&mut self) -> Result<Node, ParseError<'a>> {
        let mut node = self.parse_multiplicative()?;

        while let Some(token) = self.tokens.peek() {
            match token.kind {
                TokenKind::Punctuation("+") => {
                    self.tokens.next();
                    node = Node::Add(Box::new(node), Box::new(self.parse_multiplicative()?))
                }
                TokenKind::Punctuation("-") => {
                    self.tokens.next();
                    node = Node::Subtract(Box::new(node), Box::new(self.parse_multiplicative()?))
                }
                _ => break,
            }
        }

        Ok(node)
    }

    fn parse_primary(&mut self) -> Result<Node, ParseError<'a>> {
        let Some(token) = self.tokens.next() else {
            return Err(self.err_unusual_end_of_tokens());
        };

        match token.kind {
            TokenKind::Punctuation("(") => {
                let node = self.parse_expression()?;

                let Some(token) = self.tokens.next() else {
                    return Err(self.err_expected_expression(token.index + token.length));
                };

                match token.kind {
                    TokenKind::Punctuation(")") => Ok(node),
                    _ => Err(self.err_unexpected_token(TokenKind::Punctuation(")"), token.index)),
                }
            }
            TokenKind::Numeric(num) => Ok(Node::Numeric(num)),
            _ => Err(self.err_expected_expression(token.index)),
        }
    }

    fn parse_unary(&mut self) -> Result<Node, ParseError<'a>> {
        let Some(token) = self.tokens.peek() else {
            return Err(self.err_unusual_end_of_tokens());
        };

        match token.kind {
            TokenKind::Punctuation("+") => {
                self.tokens.next();
                self.parse_unary()
            }
            TokenKind::Punctuation("-") => {
                self.tokens.next();
                Ok(Node::Negative(Box::new(self.parse_unary()?)))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_multiplicative(&mut self) -> Result<Node, ParseError<'a>> {
        let mut node = self.parse_unary()?;

        while let Some(token) = self.tokens.peek() {
            match token.kind {
                TokenKind::Punctuation("*") => {
                    self.tokens.next();
                    node = Node::Multiply(Box::new(node), Box::new(self.parse_unary()?))
                }
                TokenKind::Punctuation("/") => {
                    self.tokens.next();
                    node = Node::Divide(Box::new(node), Box::new(self.parse_unary()?))
                }
                _ => break,
            }
        }

        Ok(node)
    }
}
