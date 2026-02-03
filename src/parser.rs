use std::{cell::RefCell, collections::HashMap, iter::Peekable, rc::Rc, vec::IntoIter};

use crate::lexer::{Token, TokenKind};

#[derive(Debug)]
pub enum BinaryKind {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    Assign,
}

#[derive(Debug)]
pub enum UnaryKind {
    Negate,
    Return,
}

#[derive(Debug)]
pub enum Node<'a> {
    Variable(Rc<RefCell<Object<'a>>>),
    Binary {
        kind: BinaryKind,
        lhs: Box<Node<'a>>,
        rhs: Box<Node<'a>>,
    },
    Unary {
        kind: UnaryKind,
        lhs: Box<Node<'a>>,
    },
    ExprStatement {
        statements: Vec<Node<'a>>,
    },
    Numeric(i64),
}

#[derive(Debug)]
pub struct Object<'a> {
    pub name: &'a str,
    pub offset: isize,
}

#[derive(Debug)]
pub struct Function<'a> {
    statements: Vec<Node<'a>>,
    locals: HashMap<&'a str, Rc<RefCell<Object<'a>>>>,
    offset: usize,
}

impl<'a> Function<'a> {
    pub fn new() -> Self {
        Self {
            statements: vec![],
            locals: HashMap::new(),
            offset: 0,
        }
    }

    pub fn add_statements(&mut self, statements: Vec<Node<'a>>) {
        self.statements = statements;
    }

    pub fn ast(self) -> Node<'a> {
        Node::ExprStatement {
            statements: self.statements,
        }
    }

    pub fn get_or_allocate_local(&mut self, name: &'a str) -> Rc<RefCell<Object<'a>>> {
        self.offset += 8;

        let object = self.locals.entry(name).or_insert_with(|| {
            Rc::new(RefCell::new(Object {
                name,
                offset: -(self.offset as isize),
            }))
        });

        Rc::clone(object)
    }

    pub fn stack_size(&mut self) -> usize {
        self.offset.next_multiple_of(16)
    }
}

pub struct Parser<'a> {
    input: &'a str,
    tokens: Peekable<IntoIter<Token<'a>>>,
    function: Function<'a>,
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
                write!(f, "{:>width$}^ ", "", width = index)?;
                write!(f, "extra token")
            }
            ParseErrorKind::UnexpectedToken {
                ref expected,
                index,
            } => {
                writeln!(f, "{}", self.input)?;
                write!(f, "{:>width$}^ ", "", width = index)?;
                write!(f, "expected `{expected}'")
            }
            ParseErrorKind::ExpectedExpression { index } => {
                writeln!(f, "{}", self.input)?;
                write!(f, "{:>width$}^ ", "", width = index)?;
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
            function: Function::new(),
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

    // program = stmt*
    pub fn parse(mut self) -> Result<Function<'a>, ParseError<'a>> {
        let mut statements = vec![];
        let mut got_eof = false;

        while let Some(Token { kind, .. }) = self.tokens.peek() {
            match kind {
                TokenKind::EOF => {
                    self.tokens.next();
                    got_eof = true;
                    break;
                }
                _ => statements.push(self.parse_statement()?),
            }
        }

        if !got_eof {
            return Err(self.err_unusual_end_of_tokens());
        }

        if let Some(Token { index, .. }) = self.tokens.next() {
            return Err(self.err_extra_token(index));
        }

        self.function.add_statements(statements);

        Ok(self.function)
    }

    // assign = equality ("=" assign)?
    fn parse_assignment(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_equality()?;

        if let Some(Token {
            kind: TokenKind::Punctuation("="),
            ..
        }) = self.tokens.peek()
        {
            self.tokens.next();
            node = Node::Binary {
                kind: BinaryKind::Assign,
                lhs: Box::new(node),
                rhs: Box::new(self.parse_assignment()?),
            };
        }

        Ok(node)
    }

    // add = mul ("+" mul | "-" mul)*
    fn parse_additive(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_multiplicative()?;

        while let Some(token) = self.tokens.peek() {
            match token.kind {
                TokenKind::Punctuation("+") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::Add,
                        lhs: Box::new(node),
                        rhs: Box::new(self.parse_multiplicative()?),
                    };
                }
                TokenKind::Punctuation("-") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::Subtract,
                        lhs: Box::new(node),
                        rhs: Box::new(self.parse_multiplicative()?),
                    }
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // relational = add ("<" add | "<=" add | ">" add | ">=" add)*
    fn parse_relational(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_additive()?;

        while let Some(token) = self.tokens.peek() {
            match token.kind {
                TokenKind::Punctuation("<") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::LessThan,
                        lhs: Box::new(node),
                        rhs: Box::new(self.parse_additive()?),
                    };
                }
                TokenKind::Punctuation("<=") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::LessThanEqual,
                        lhs: Box::new(node),
                        rhs: Box::new(self.parse_additive()?),
                    };
                }
                TokenKind::Punctuation(">") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::LessThan,
                        lhs: Box::new(self.parse_additive()?),
                        rhs: Box::new(node),
                    };
                }
                TokenKind::Punctuation(">=") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::LessThanEqual,
                        lhs: Box::new(self.parse_additive()?),
                        rhs: Box::new(node),
                    };
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // equality = relational ("==" relational | "!=" relational)*
    fn parse_equality(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_relational()?;

        while let Some(token) = self.tokens.peek() {
            match token.kind {
                TokenKind::Punctuation("==") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::Equal,
                        lhs: Box::new(node),
                        rhs: Box::new(self.parse_relational()?),
                    };
                }
                TokenKind::Punctuation("!=") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::NotEqual,
                        lhs: Box::new(node),
                        rhs: Box::new(self.parse_relational()?),
                    }
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // stmt = "return" expr ";" | expr-stmt
    fn parse_statement(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        if let Some(Token {
            kind: TokenKind::Keyword("return"),
            ..
        }) = self.tokens.peek()
        {
            self.tokens.next();
            let node = Node::Unary {
                kind: UnaryKind::Return,
                lhs: Box::new(self.parse_expression()?),
            };

            let Some(token) = self.tokens.next() else {
                return Err(self.err_unusual_end_of_tokens());
            };
            let Token {
                kind: TokenKind::Punctuation(";"),
                ..
            } = token
            else {
                return Err(self.err_unexpected_token(TokenKind::Punctuation(";"), token.index));
            };

            return Ok(node);
        }

        self.parse_expr_statement()
    }

    // expr-stmt = expr ";"
    fn parse_expr_statement(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let node = self.parse_expression()?;

        let Some(token) = self.tokens.next() else {
            return Err(self.err_unusual_end_of_tokens());
        };

        match token.kind {
            TokenKind::Punctuation(";") => Ok(node),
            _ => Err(self.err_unexpected_token(TokenKind::Punctuation(";"), token.index)),
        }
    }

    // expr = assign
    fn parse_expression(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        self.parse_assignment()
    }

    // primary = "(" expr ")" | ident | num
    fn parse_primary(&mut self) -> Result<Node<'a>, ParseError<'a>> {
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
            TokenKind::Identifier(name) => {
                Ok(Node::Variable(self.function.get_or_allocate_local(name)))
            }
            TokenKind::Numeric(num) => Ok(Node::Numeric(num)),
            _ => Err(self.err_expected_expression(token.index)),
        }
    }

    // unary = ("*" | "-" unary | primary
    fn parse_unary(&mut self) -> Result<Node<'a>, ParseError<'a>> {
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
                Ok(Node::Unary {
                    kind: UnaryKind::Negate,
                    lhs: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_primary(),
        }
    }

    // mul = unary ("*" unary | "/" unary)*
    fn parse_multiplicative(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_unary()?;

        while let Some(token) = self.tokens.peek() {
            match token.kind {
                TokenKind::Punctuation("*") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::Multiply,
                        lhs: Box::new(node),
                        rhs: Box::new(self.parse_unary()?),
                    }
                }
                TokenKind::Punctuation("/") => {
                    self.tokens.next();
                    node = Node::Binary {
                        kind: BinaryKind::Divide,
                        lhs: Box::new(node),
                        rhs: Box::new(self.parse_unary()?),
                    }
                }
                _ => break,
            }
        }

        Ok(node)
    }
}
