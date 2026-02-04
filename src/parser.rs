use std::{cell::RefCell, collections::HashMap, iter::Peekable, rc::Rc, vec::IntoIter};

use crate::{
    lexer::{Token, TokenKind},
    types::Type,
    util::Info,
};

#[derive(Debug)]
pub struct Node<'a> {
    pub kind: NodeKind<'a>,
    pub info: Info,
    pub node_type: Rc<Type>,
}

impl<'a> Node<'a> {
    pub fn new(kind: NodeKind<'a>, info: Info) -> Self {
        let node_type = Type::of(&kind);

        Self {
            kind,
            info,
            node_type,
        }
    }
}

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
    Address,
    Deref,
    Return,
}

#[derive(Debug)]
pub enum NodeKind<'a> {
    Numeric {
        value: i64,
    },
    Variable {
        object: Rc<RefCell<Object<'a>>>,
    },
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
    Block {
        compound_statements: Vec<Node<'a>>,
    },
    If {
        condition: Box<Node<'a>>,
        then_block: Box<Node<'a>>,
        else_block: Option<Box<Node<'a>>>,
    },
    Loop {
        init: Option<Box<Node<'a>>>,
        condition: Option<Box<Node<'a>>>,
        increment: Option<Box<Node<'a>>>,
        loop_block: Box<Node<'a>>,
    },
}

impl<'a> NodeKind<'a> {
    pub fn binary(kind: BinaryKind, lhs: Box<Node<'a>>, rhs: Box<Node<'a>>) -> Self {
        Self::Binary { kind, lhs, rhs }
    }

    pub fn unary(kind: UnaryKind, lhs: Box<Node<'a>>) -> Self {
        Self::Unary { kind, lhs }
    }

    pub fn numeric(value: i64) -> Self {
        Self::Numeric { value }
    }

    pub fn variable(object: Rc<RefCell<Object<'a>>>) -> Self {
        Self::Variable { object }
    }
}

#[derive(Debug)]
pub struct Object<'a> {
    pub name: &'a str,
    pub offset: isize,
}

#[derive(Debug)]
pub struct Function<'a> {
    body: Node<'a>,
    locals: HashMap<&'a str, Rc<RefCell<Object<'a>>>>,
    offset: usize,
}

impl<'a> Function<'a> {
    pub fn new() -> Self {
        Self {
            locals: HashMap::new(),
            offset: 0,
            body: Node::new(NodeKind::numeric(0), Info { index: 0 }),
        }
    }

    pub fn set_body(&mut self, node: Node<'a>) {
        self.body = node;
    }

    pub fn body(&self) -> &Node<'a> {
        &self.body
    }

    pub fn get_or_allocate_local(&mut self, name: &'a str) -> Rc<RefCell<Object<'a>>> {
        // NOTE: assigning offsets this way leads to a stack locals order
        // that is inverted compared to chibicc
        let object = self.locals.entry(name).or_insert_with(|| {
            self.offset += 8;

            Rc::new(RefCell::new(Object {
                name,
                offset: -(self.offset as isize),
            }))
        });

        Rc::clone(object)
    }

    pub fn stack_size(&self) -> usize {
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
    InvalidOperands {
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
            ParseErrorKind::InvalidOperands { index } => {
                writeln!(f, "{}", self.input)?;
                write!(f, "{:>width$}^ ", "", width = index)?;
                write!(f, "invalid operands")
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

    fn expect_next(&mut self, kind: TokenKind<'a>) -> Result<(), ParseError<'a>> {
        let Some(token) = self.tokens.next() else {
            return Err(self.err_unusual_end_of_tokens());
        };

        if kind == token.kind {
            Ok(())
        } else {
            Err(self.err_unexpected_token(kind, token.info.index))
        }
    }

    // program = compound-stmt*
    pub fn parse(mut self) -> Result<Function<'a>, ParseError<'a>> {
        let Some(Token {
            kind: TokenKind::Punctuation("{"),
            info,
        }) = self.tokens.next()
        else {
            return Err(self.err_unexpected_token(TokenKind::Punctuation("{"), 0));
        };

        let block = self.parse_compound_statement(info)?;
        self.function.set_body(block);

        Ok(self.function)
    }

    // assign = equality ("=" assign)?
    fn parse_assignment(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_equality()?;

        if let Some(&Token {
            kind: TokenKind::Punctuation("="),
            info,
        }) = self.tokens.peek()
        {
            self.tokens.next();
            node = Node::new(
                NodeKind::binary(
                    BinaryKind::Assign,
                    Box::new(node),
                    Box::new(self.parse_assignment()?),
                ),
                info,
            )
        }

        Ok(node)
    }

    // add = mul ("+" mul | "-" mul)*
    fn parse_additive(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_multiplicative()?;

        while let Some(&Token { ref kind, info }) = self.tokens.peek() {
            match kind {
                TokenKind::Punctuation("+") => {
                    self.tokens.next();

                    let lhs = Box::new(node);
                    let rhs = Box::new(self.parse_multiplicative()?);
                    node = self.parse_addition(lhs, rhs, info)?
                }
                TokenKind::Punctuation("-") => {
                    self.tokens.next();

                    let lhs = Box::new(node);
                    let rhs = Box::new(self.parse_multiplicative()?);
                    node = self.parse_subtraction(lhs, rhs, info)?
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // relational = add ("<" add | "<=" add | ">" add | ">=" add)*
    fn parse_relational(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_additive()?;

        while let Some(&Token { ref kind, info }) = self.tokens.peek() {
            match kind {
                TokenKind::Punctuation("<") => {
                    self.tokens.next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::LessThan,
                            Box::new(node),
                            Box::new(self.parse_additive()?),
                        ),
                        info,
                    );
                }
                TokenKind::Punctuation("<=") => {
                    self.tokens.next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::LessThanEqual,
                            Box::new(node),
                            Box::new(self.parse_additive()?),
                        ),
                        info,
                    );
                }
                TokenKind::Punctuation(">") => {
                    self.tokens.next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::LessThan,
                            Box::new(self.parse_additive()?),
                            Box::new(node),
                        ),
                        info,
                    );
                }
                TokenKind::Punctuation(">=") => {
                    self.tokens.next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::LessThanEqual,
                            Box::new(self.parse_additive()?),
                            Box::new(node),
                        ),
                        info,
                    );
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // equality = relational ("==" relational | "!=" relational)*
    fn parse_equality(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_relational()?;

        while let Some(&Token { ref kind, info }) = self.tokens.peek() {
            match kind {
                TokenKind::Punctuation("==") => {
                    self.tokens.next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::Equal,
                            Box::new(node),
                            Box::new(self.parse_relational()?),
                        ),
                        info,
                    );
                }
                TokenKind::Punctuation("!=") => {
                    self.tokens.next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::NotEqual,
                            Box::new(node),
                            Box::new(self.parse_relational()?),
                        ),
                        info,
                    )
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // stmt = "return" expr ";"
    //      | "if" "(" expr ")" stmt ("else" stmt)?
    //      | "for" "(" expr-stmt expr? ";" expr? ")" stmt
    //      | "while" "(" expr ")" stmt
    //      | "{" compound-stmt
    //      | expr-stmt
    fn parse_statement(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        match self.tokens.peek() {
            Some(&Token {
                kind: TokenKind::Keyword("return"),
                info,
            }) => {
                self.tokens.next();

                let node = Node::new(
                    NodeKind::unary(UnaryKind::Return, Box::new(self.parse_expression()?)),
                    info,
                );

                self.expect_next(TokenKind::Punctuation(";"))?;

                Ok(node)
            }
            Some(&Token {
                kind: TokenKind::Keyword("if"),
                info,
            }) => {
                self.tokens.next();
                self.expect_next(TokenKind::Punctuation("("))?;
                let condition = Box::new(self.parse_expression()?);
                self.expect_next(TokenKind::Punctuation(")"))?;
                let then_block = Box::new(self.parse_statement()?);

                let else_block = if let Some(Token {
                    kind: TokenKind::Keyword("else"),
                    ..
                }) = self.tokens.peek()
                {
                    self.tokens.next();
                    Some(Box::new(self.parse_statement()?))
                } else {
                    None
                };

                Ok(Node::new(
                    NodeKind::If {
                        condition,
                        then_block,
                        else_block,
                    },
                    info,
                ))
            }
            Some(&Token {
                kind: TokenKind::Keyword("for"),
                info,
            }) => {
                self.tokens.next();
                self.expect_next(TokenKind::Punctuation("("))?;

                let init = Some(Box::new(self.parse_expr_statement()?));

                let condition = if let Some(Token {
                    kind: TokenKind::Punctuation(";"),
                    ..
                }) = self.tokens.peek()
                {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                self.expect_next(TokenKind::Punctuation(";"))?;

                let increment = if let Some(Token {
                    kind: TokenKind::Punctuation(")"),
                    ..
                }) = self.tokens.peek()
                {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                self.expect_next(TokenKind::Punctuation(")"))?;

                let loop_block = Box::new(self.parse_statement()?);

                Ok(Node::new(
                    NodeKind::Loop {
                        init,
                        condition,
                        increment,
                        loop_block,
                    },
                    info,
                ))
            }
            Some(&Token {
                kind: TokenKind::Keyword("while"),
                info,
            }) => {
                self.tokens.next();
                self.expect_next(TokenKind::Punctuation("("))?;
                let condition = Some(Box::new(self.parse_expression()?));
                self.expect_next(TokenKind::Punctuation(")"))?;
                let loop_block = Box::new(self.parse_statement()?);

                Ok(Node::new(
                    NodeKind::Loop {
                        init: None,
                        condition,
                        increment: None,
                        loop_block,
                    },
                    info,
                ))
            }
            Some(&Token {
                kind: TokenKind::Punctuation("{"),
                info,
            }) => {
                self.tokens.next();
                self.parse_compound_statement(info)
            }
            _ => self.parse_expr_statement(),
        }
    }

    // compound-stmt = stmt* "}"
    fn parse_compound_statement(&mut self, info: Info) -> Result<Node<'a>, ParseError<'a>> {
        let mut compound_statements = vec![];

        while let Some(Token { kind, .. }) = self.tokens.peek() {
            if let TokenKind::Punctuation("}") = kind {
                self.tokens.next();
                break;
            }
            compound_statements.push(self.parse_statement()?);
        }

        Ok(Node::new(
            NodeKind::Block {
                compound_statements,
            },
            info,
        ))
    }

    // expr-stmt = expr? ";"
    fn parse_expr_statement(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        if let Some(&Token {
            kind: TokenKind::Punctuation(";"),
            info,
        }) = self.tokens.peek()
        {
            self.tokens.next();
            return Ok(Node::new(
                NodeKind::ExprStatement { statements: vec![] },
                info,
            ));
        };

        let node = self.parse_expression()?;
        self.expect_next(TokenKind::Punctuation(";"))?;

        Ok(node)
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
                self.expect_next(TokenKind::Punctuation(")"))?;
                Ok(node)
            }
            TokenKind::Identifier(name) => Ok(Node::new(
                NodeKind::variable(self.function.get_or_allocate_local(name)),
                token.info,
            )),
            TokenKind::Numeric(num) => Ok(Node::new(NodeKind::numeric(num), token.info)),
            _ => Err(self.err_expected_expression(token.info.index)),
        }
    }

    // unary = ("*" | "-" | "*" | "&" ) unary
    //         | primary
    fn parse_unary(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let Some(&Token { ref kind, info }) = self.tokens.peek() else {
            return Err(self.err_unusual_end_of_tokens());
        };

        match kind {
            TokenKind::Punctuation("+") => {
                self.tokens.next();
                self.parse_unary()
            }
            TokenKind::Punctuation("-") => {
                self.tokens.next();
                Ok(Node::new(
                    NodeKind::unary(UnaryKind::Negate, Box::new(self.parse_unary()?)),
                    info,
                ))
            }
            TokenKind::Punctuation("&") => {
                self.tokens.next();
                Ok(Node::new(
                    NodeKind::unary(UnaryKind::Address, Box::new(self.parse_unary()?)),
                    info,
                ))
            }
            TokenKind::Punctuation("*") => {
                self.tokens.next();
                Ok(Node::new(
                    NodeKind::unary(UnaryKind::Deref, Box::new(self.parse_unary()?)),
                    info,
                ))
            }
            _ => self.parse_primary(),
        }
    }

    // mul = unary ("*" unary | "/" unary)*
    fn parse_multiplicative(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_unary()?;

        while let Some(&Token { ref kind, info }) = self.tokens.peek() {
            match kind {
                TokenKind::Punctuation("*") => {
                    self.tokens.next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::Multiply,
                            Box::new(node),
                            Box::new(self.parse_unary()?),
                        ),
                        info,
                    )
                }
                TokenKind::Punctuation("/") => {
                    self.tokens.next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::Divide,
                            Box::new(node),
                            Box::new(self.parse_unary()?),
                        ),
                        info,
                    )
                }
                _ => break,
            }
        }

        Ok(node)
    }

    pub fn parse_addition(
        &self,
        lhs: Box<Node<'a>>,
        rhs: Box<Node<'a>>,
        info: Info,
    ) -> Result<Node<'a>, ParseError<'a>> {
        match (lhs.node_type.as_ref(), rhs.node_type.as_ref()) {
            (Type::Integer, Type::Integer) => {
                Ok(Node::new(NodeKind::binary(BinaryKind::Add, lhs, rhs), info))
            }
            (Type::Integer, Type::Pointer(_)) | (Type::Pointer(_), Type::Integer) => {
                let (lhs, rhs) = match rhs.node_type.as_ref() {
                    Type::Pointer(_) => (rhs, lhs),
                    _ => (lhs, rhs),
                };

                let rhs = Node::new(
                    NodeKind::binary(
                        BinaryKind::Multiply,
                        rhs,
                        Box::new(Node::new(NodeKind::numeric(8), info)),
                    ),
                    info,
                );

                Ok(Node::new(
                    NodeKind::binary(BinaryKind::Add, lhs, Box::new(rhs)),
                    info,
                ))
            }
            (Type::Pointer(_), Type::Pointer(_)) | _ => Err(self.err_invalid_operands(info.index)),
        }
    }

    pub fn parse_subtraction(
        &self,
        lhs: Box<Node<'a>>,
        rhs: Box<Node<'a>>,
        info: Info,
    ) -> Result<Node<'a>, ParseError<'a>> {
        match (lhs.node_type.as_ref(), rhs.node_type.as_ref()) {
            (Type::Integer, Type::Integer) => Ok(Node::new(
                NodeKind::binary(BinaryKind::Subtract, lhs, rhs),
                info,
            )),
            (Type::Pointer(_), Type::Integer) => {
                let rhs = Node::new(
                    NodeKind::binary(
                        BinaryKind::Multiply,
                        rhs,
                        Box::new(Node::new(NodeKind::numeric(8), info)),
                    ),
                    info,
                );

                Ok(Node::new(
                    NodeKind::binary(BinaryKind::Subtract, lhs, Box::new(rhs)),
                    info,
                ))
            }
            (Type::Pointer(_), Type::Pointer(_)) => {
                let difference = Node::new(NodeKind::binary(BinaryKind::Subtract, lhs, rhs), info);

                Ok(Node::new(
                    NodeKind::binary(
                        BinaryKind::Divide,
                        Box::new(difference),
                        Box::new(Node::new(NodeKind::numeric(8), info)),
                    ),
                    info,
                ))
            }
            _ => Err(self.err_invalid_operands(info.index)),
        }
    }

    fn err_invalid_operands(&self, index: usize) -> ParseError<'a> {
        ParseError {
            input: self.input,
            kind: ParseErrorKind::InvalidOperands { index },
        }
    }

    fn err_unusual_end_of_tokens(&self) -> ParseError<'a> {
        ParseError {
            input: self.input,
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

    fn _err_extra_token(&self, index: usize) -> ParseError<'a> {
        ParseError {
            input: self.input,
            kind: ParseErrorKind::ExtraToken { index },
        }
    }
}
