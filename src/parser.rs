pub mod types;

use crate::{
    lexer::{Token, TokenKind},
    util::{self, Info},
};
use std::{borrow::Cow, collections::HashMap, iter::Peekable, rc::Rc, slice::Iter};
use types::{Type, TypeError, TypeKind};

#[derive(Debug)]
pub struct Node<'a> {
    pub kind: NodeKind<'a>,
    pub info: Info,
    pub node_type: Rc<Type>,
}

impl<'a> Node<'a> {
    fn new(kind: NodeKind<'a>, info: Info) -> Result<Self, TypeError> {
        let node_type = Type::of(&kind)?;

        Ok(Self {
            kind,
            info,
            node_type,
        })
    }

    fn new_of_type(kind: NodeKind<'a>, info: Info, node_type: Rc<Type>) -> Self {
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
    Comma,
}

#[derive(Debug)]
pub enum UnaryKind {
    Negate,
    Address,
    Deref,
    Return,
    ExprStatement,
}

#[derive(Debug)]
pub enum CompoundStatementKind {
    Block,
    StatementExpr,
}

#[derive(Debug)]
pub enum NodeKind<'a> {
    Numeric(usize),
    Variable(Rc<Object<'a>>),
    FunctionCall {
        name: &'a str,
        args: Vec<Node<'a>>,
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
    CompoundStatement {
        kind: CompoundStatementKind,
        nodes: Vec<Node<'a>>,
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
    fn binary(kind: BinaryKind, lhs: Box<Node<'a>>, rhs: Box<Node<'a>>) -> Self {
        Self::Binary { kind, lhs, rhs }
    }

    fn unary(kind: UnaryKind, lhs: Box<Node<'a>>) -> Self {
        Self::Unary { kind, lhs }
    }
}

#[derive(Debug)]
pub enum Lifetime {
    Global,
    Local { offset: isize },
}

#[derive(Debug)]
pub struct Object<'a> {
    pub name: Cow<'a, str>,
    pub kind: ObjectKind<'a>,
    pub lifetime: Lifetime,
    pub object_type: Rc<Type>,
}

#[derive(Debug)]
pub enum ObjectKind<'a> {
    Function { function: Function<'a> },
    Variable { initial_data: Vec<u8> },
    TranslationUnit(TranslationUnit<'a>),
}

impl<'a> Object<'a> {
    pub fn local_offset(&self) -> isize {
        match self.lifetime {
            Lifetime::Global => 0,
            Lifetime::Local { offset } => offset,
        }
    }

    fn global_variable_with_data(
        name: Cow<'a, str>,
        object_type: Rc<Type>,
        initial_data: Vec<u8>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: ObjectKind::Variable {
                initial_data: initial_data.into(),
            },
            lifetime: Lifetime::Global,
            object_type,
        }
    }

    fn global_variable(name: &'a str, object_type: Rc<Type>) -> Self {
        Self::global_variable_with_data(name.into(), object_type, vec![])
    }

    fn global_translation_unit(tu: TranslationUnit<'a>) -> Self {
        Self {
            name: "<global>".into(),
            kind: ObjectKind::TranslationUnit(tu),
            lifetime: Lifetime::Global,
            object_type: Type::none(),
        }
    }

    fn function(function: Function<'a>, object_type: Rc<Type>, lifetime: Lifetime) -> Self {
        Self {
            name: function.name.clone(),
            kind: ObjectKind::Function { function },
            lifetime,
            object_type,
        }
    }
}

#[derive(Debug)]
pub struct TranslationUnit<'a> {
    pub objects: HashMap<Cow<'a, str>, Rc<Object<'a>>>,
}

impl<'a> TranslationUnit<'a> {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
        }
    }

    fn get_or_allocate(&mut self, name: Cow<'a, str>, object: Object<'a>) -> Rc<Object<'a>> {
        let object = self.objects.entry(name).or_insert_with(|| Rc::new(object));

        Rc::clone(object)
    }

    fn allocate(&mut self, name: Cow<'a, str>, object: Object<'a>) {
        self.objects.entry(name).or_insert_with(|| Rc::new(object));
    }

    fn get_object(&self, name: &str) -> Option<Rc<Object<'a>>> {
        self.objects.get(name).map(|g| g.clone())
    }
}

#[derive(Debug)]
pub struct Function<'a> {
    name: Cow<'a, str>,
    params: Vec<Rc<Object<'a>>>,
    body: Vec<Node<'a>>,
    locals: HashMap<Cow<'a, str>, Rc<Object<'a>>>,
    offset: usize,
}

impl<'a> Function<'a> {
    fn new() -> Self {
        Self {
            name: "".into(),
            params: vec![],
            locals: HashMap::new(),
            offset: 0,
            body: vec![],
        }
    }

    pub fn params(&self) -> &[Rc<Object<'a>>] {
        &self.params
    }

    pub fn name(&self) -> &Cow<'a, str> {
        &self.name
    }

    fn push_param(&mut self, param: Rc<Object<'a>>) {
        self.params.push(param)
    }

    fn set_name(&mut self, name: Cow<'a, str>) {
        self.name = name
    }

    fn set_body(&mut self, nodes: Vec<Node<'a>>) {
        self.body = nodes;
    }

    pub fn body(&self) -> &[Node<'a>] {
        &self.body
    }

    fn get_local(&self, name: &str) -> Option<Rc<Object<'a>>> {
        self.locals.get(name).map(|l| l.clone())
    }

    fn get_or_allocate_local(
        &mut self,
        name: Cow<'a, str>,
        object_type: Rc<Type>,
    ) -> Rc<Object<'a>> {
        // NOTE: assigning offsets this way leads to a stack locals order
        // that is inverted compared to chibicc
        let object = self.locals.entry(name.clone()).or_insert_with(|| {
            self.offset += object_type.size;

            Rc::new(Object {
                name: name.into(),
                kind: ObjectKind::Variable {
                    initial_data: vec![],
                },
                lifetime: Lifetime::Local {
                    offset: -(self.offset as isize),
                },
                object_type,
            })
        });

        Rc::clone(object)
    }

    pub fn stack_size(&self) -> usize {
        self.offset.next_multiple_of(16)
    }
}

#[derive(Debug)]
pub struct Parser<'a> {
    file: &'a str,
    input: &'a str,
    tokens: Peekable<Iter<'a, Token<'a>>>,
    lookahead_tokens: Option<Peekable<Iter<'a, Token<'a>>>>,
    function: Option<Function<'a>>,
    translation_unit: TranslationUnit<'a>,
}

struct LookaheadGuard<'a, 'b> {
    parser: &'a mut Parser<'b>,
    actual_translation_unit: TranslationUnit<'b>,
    actual_function: Option<Function<'b>>,
}

impl<'a, 'b> LookaheadGuard<'a, 'b> {
    fn new(parser: &'a mut Parser<'b>) -> Self {
        parser.enable_lookahead();
        let actual_translation_unit =
            std::mem::replace(&mut parser.translation_unit, TranslationUnit::new());
        let actual_function = parser.function.take();
        Self {
            parser,
            actual_translation_unit,
            actual_function,
        }
    }
}
impl<'a, 'b> Drop for LookaheadGuard<'a, 'b> {
    fn drop(&mut self) {
        self.parser.disable_lookahead();
        self.parser.translation_unit =
            std::mem::replace(&mut self.actual_translation_unit, TranslationUnit::new());
        self.parser.function = self.actual_function.take();
    }
}

pub enum ParseErrorKind<'a> {
    InvalidPointerDeref,
    ExtraToken,
    UndefinedVariable,
    ExpectedExpression,
    InvalidOperands,
    UnexpectedToken { expected: TokenKind<'a> },
    UnusualEndOfTokens,
    ExpectedNumber,
    ExpectedVariableName,
    NonLValueAssignment,
    StatementExprReturnsVoid,
}

pub struct ParseError<'a> {
    file: &'a str,
    input: &'a str,
    index: usize,
    kind: ParseErrorKind<'a>,
}

impl<'a> std::fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        util::info_msg(f, self.file, self.input, self.index)?;
        match &self.kind {
            ParseErrorKind::ExtraToken => write!(f, "extra token"),
            ParseErrorKind::UnexpectedToken { expected } => write!(f, "expected `{expected}'"),
            ParseErrorKind::ExpectedExpression => write!(f, "expected an expression"),
            ParseErrorKind::InvalidOperands => write!(f, "invalid operands"),
            ParseErrorKind::InvalidPointerDeref => write!(f, "invalid pointer dereference"),
            ParseErrorKind::ExpectedNumber => write!(f, "expected a number"),
            ParseErrorKind::ExpectedVariableName => write!(f, "expected a variable name"),
            ParseErrorKind::UndefinedVariable => write!(f, "undefined variable"),
            ParseErrorKind::NonLValueAssignment => write!(f, "not an lvalue"),
            ParseErrorKind::UnusualEndOfTokens => {
                write!(f, "fatal error: Unusual end of tokens during parsing")
            }
            ParseErrorKind::StatementExprReturnsVoid => {
                write!(f, "statement expression returning void is not supported")
            }
        }
    }
}

impl<'a> Parser<'a> {
    pub fn new(file: &'a str, input: &'a str, tokens: &'a [Token<'a>]) -> Self {
        Self {
            file,
            input,
            tokens: tokens.iter().peekable(),
            lookahead_tokens: None,
            function: None,
            translation_unit: TranslationUnit::new(),
        }
    }

    fn take_function(&mut self) -> Function<'a> {
        self.function
            .take()
            .expect("Shouldn't have taken out the function this soon")
    }

    fn current_function(&mut self) -> &mut Function<'a> {
        self.function.get_or_insert_with(|| Function::new())
    }

    fn expect_next(&mut self, kind: TokenKind<'a>) -> Result<(), ParseError<'a>> {
        let Some(token) = self.tokens().next() else {
            return Err(self.err_unusual_end_of_tokens());
        };

        if kind == token.kind {
            Ok(())
        } else {
            Err(self.err_unexpected_token(kind, token.info.index))
        }
    }

    fn enable_lookahead(&mut self) {
        if self.lookahead_tokens.is_some() {
            return;
        }
        self.lookahead_tokens = Some(self.tokens.clone());
    }

    fn disable_lookahead(&mut self) {
        self.lookahead_tokens = None;
    }

    fn tokens(&mut self) -> &mut Peekable<Iter<'a, Token<'a>>> {
        if let Some(tokens) = &mut self.lookahead_tokens {
            tokens
        } else {
            &mut self.tokens
        }
    }

    fn is_following_function(&mut self, decl_type: &Rc<Type>) -> Result<bool, ParseError<'a>> {
        if let Some(Token {
            kind: TokenKind::Punctuation(";"),
            ..
        }) = self.tokens().peek()
        {
            return Ok(false);
        }

        let guard = LookaheadGuard::new(self);
        let (_, decl_type) = guard.parser.parse_declarator(decl_type.clone())?;

        Ok(matches!(&decl_type.kind, TypeKind::Function { .. }))
    }

    // program = (function-definition | global-variable)*
    pub fn parse(mut self) -> Result<Object<'a>, ParseError<'a>> {
        while let Some(Token { kind, .. }) = self.tokens().peek() {
            if let TokenKind::EOF = kind {
                break;
            }

            let decl_type = self.parse_declaration_spec()?;
            if self.is_following_function(&decl_type)? {
                self.parse_function(decl_type.clone())?;
            } else {
                self.parse_global_variable(decl_type)?;
            }
        }
        self.expect_next(TokenKind::EOF)?;

        Ok(Object::global_translation_unit(self.translation_unit))
    }

    fn parse_global_variable(&mut self, decl_type: Rc<Type>) -> Result<(), ParseError<'a>> {
        let mut variables: Vec<Object> = vec![];

        while let Some(Token { kind, .. }) = self.tokens().peek() {
            if let TokenKind::Punctuation(";") = kind {
                break;
            }

            if !variables.is_empty() {
                self.expect_next(TokenKind::Punctuation(","))?
            }

            let (name, variable_type) = self.parse_declarator(decl_type.clone())?;
            variables.push(Object::global_variable(name, variable_type));
        }
        self.expect_next(TokenKind::Punctuation(";"))?;

        for variable in variables {
            self.translation_unit
                .allocate(variable.name.clone(), variable);
        }

        Ok(())
    }

    // function = compound-stmt*
    fn parse_function(&mut self, return_type: Rc<Type>) -> Result<(), ParseError<'a>> {
        let (identifier, decl_type) = self.parse_declarator(return_type.clone())?;
        self.current_function().set_name(identifier.into());

        self.expect_next(TokenKind::Punctuation("{"))?;
        let function_body = self.parse_compound_statement()?;
        self.current_function().set_body(function_body);

        let function = self.take_function();
        self.translation_unit.allocate(
            function.name.clone(),
            Object::function(function, decl_type, Lifetime::Global),
        );

        Ok(())
    }

    // assign = equality ("=" assign)?
    fn parse_assignment(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_equality()?;

        if let Some(Token {
            kind: TokenKind::Punctuation("="),
            info,
        }) = self.tokens().peek()
        {
            self.tokens().next();
            node = Node::new(
                NodeKind::binary(
                    BinaryKind::Assign,
                    Box::new(node),
                    Box::new(self.parse_assignment()?),
                ),
                *info,
            )
            .map_err(|e| self.err_type_error(e, info.index))?
        }

        Ok(node)
    }

    // add = mul ("+" mul | "-" mul)*
    fn parse_additive(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_multiplicative()?;

        while let Some(Token { kind, info }) = self.tokens().peek() {
            match kind {
                TokenKind::Punctuation("+") => {
                    self.tokens().next();

                    let lhs = Box::new(node);
                    let rhs = Box::new(self.parse_multiplicative()?);
                    node = self.parse_addition(lhs, rhs, *info)?
                }
                TokenKind::Punctuation("-") => {
                    self.tokens().next();

                    let lhs = Box::new(node);
                    let rhs = Box::new(self.parse_multiplicative()?);
                    node = self.parse_subtraction(lhs, rhs, *info)?
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // relational = add ("<" add | "<=" add | ">" add | ">=" add)*
    fn parse_relational(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_additive()?;

        while let Some(Token { kind, info }) = self.tokens().peek() {
            let &info = info;
            match kind {
                TokenKind::Punctuation("<") => {
                    self.tokens().next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::LessThan,
                            Box::new(node),
                            Box::new(self.parse_additive()?),
                        ),
                        info,
                    )
                    .map_err(|e| self.err_type_error(e, info.index))?;
                }
                TokenKind::Punctuation("<=") => {
                    self.tokens().next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::LessThanEqual,
                            Box::new(node),
                            Box::new(self.parse_additive()?),
                        ),
                        info,
                    )
                    .map_err(|e| self.err_type_error(e, info.index))?;
                }
                TokenKind::Punctuation(">") => {
                    self.tokens().next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::LessThan,
                            Box::new(self.parse_additive()?),
                            Box::new(node),
                        ),
                        info,
                    )
                    .map_err(|e| self.err_type_error(e, info.index))?;
                }
                TokenKind::Punctuation(">=") => {
                    self.tokens().next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::LessThanEqual,
                            Box::new(self.parse_additive()?),
                            Box::new(node),
                        ),
                        info,
                    )
                    .map_err(|e| self.err_type_error(e, info.index))?;
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // equality = relational ("==" relational | "!=" relational)*
    fn parse_equality(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_relational()?;

        while let Some(Token { kind, info }) = self.tokens().peek() {
            let &info = info;
            match kind {
                TokenKind::Punctuation("==") => {
                    self.tokens().next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::Equal,
                            Box::new(node),
                            Box::new(self.parse_relational()?),
                        ),
                        info,
                    )
                    .map_err(|e| self.err_type_error(e, info.index))?
                }
                TokenKind::Punctuation("!=") => {
                    self.tokens().next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::NotEqual,
                            Box::new(node),
                            Box::new(self.parse_relational()?),
                        ),
                        info,
                    )
                    .map_err(|e| self.err_type_error(e, info.index))?
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // declspec = "char" | "int"
    fn parse_declaration_spec(&mut self) -> Result<Rc<Type>, ParseError<'a>> {
        let Some(Token { kind, info }) = self.tokens().next() else {
            return Err(self.err_unusual_end_of_tokens());
        };
        match kind {
            TokenKind::Keyword("char") => Ok(Type::char()),
            TokenKind::Keyword("int") => Ok(Type::integer()),
            _ => Err(self.err_unexpected_token(TokenKind::Keyword("int|char"), info.index)),
        }
    }

    // func-params = (param ("," param)? ")"
    // param       = declspec declarator
    fn parse_function_params(&mut self, decl_type: Rc<Type>) -> Result<Rc<Type>, ParseError<'a>> {
        while let Some(Token { kind, .. }) = self.tokens().peek() {
            if let TokenKind::Punctuation(")") = kind {
                break;
            }

            if !self.current_function().params().is_empty() {
                self.expect_next(TokenKind::Punctuation(","))?;
            }

            let param_type = self.parse_declaration_spec()?;
            let (param_name, param_type) = self.parse_declarator(param_type)?;
            let object = self
                .current_function()
                .get_or_allocate_local(param_name.into(), param_type);
            self.current_function().push_param(object);
        }
        self.expect_next(TokenKind::Punctuation(")"))?;

        return Ok(Type::function(&decl_type));
    }

    // type-suffix = "(" func-params
    //             | "[" num "]" type-suffix
    //             | epsilon
    fn parse_type_suffix(&mut self, decl_type: Rc<Type>) -> Result<Rc<Type>, ParseError<'a>> {
        if let Some(Token { kind, .. }) = self.tokens().peek() {
            match kind {
                TokenKind::Punctuation("(") => {
                    self.tokens().next();
                    return self.parse_function_params(decl_type);
                }
                TokenKind::Punctuation("[") => {
                    self.tokens().next();

                    let Some(Token { kind, info }) = self.tokens().next() else {
                        return Err(self.err_unusual_end_of_tokens());
                    };
                    let TokenKind::Numeric(size) = kind else {
                        return Err(self.emit_error(ParseErrorKind::ExpectedNumber, info.index));
                    };
                    self.expect_next(TokenKind::Punctuation("]"))?;

                    let array_type = self.parse_type_suffix(decl_type)?;
                    return Ok(Type::array_of(&array_type, *size));
                }
                _ => {}
            }
        }

        Ok(decl_type)
    }

    // declarator = "*"* ident type-suffix
    fn parse_declarator(
        &mut self,
        base_type: Rc<Type>,
    ) -> Result<(&'a str, Rc<Type>), ParseError<'a>> {
        let mut decl_type = base_type;
        while let Some(Token {
            kind: TokenKind::Punctuation("*"),
            ..
        }) = self.tokens().peek()
        {
            self.tokens().next();
            decl_type = Type::pointer_to(&decl_type);
        }

        let Some(Token { kind, info }) = self.tokens().next() else {
            return Err(self.err_unusual_end_of_tokens());
        };
        let TokenKind::Identifier(identifier) = kind else {
            return Err(self.emit_error(ParseErrorKind::ExpectedVariableName, info.index));
        };

        let decl_type = self.parse_type_suffix(decl_type)?;

        Ok((identifier, decl_type))
    }

    // declaration = declspec (declarator ("=" expr)? ("," declarator ("=" expr)?)*)? ";"
    fn parse_declaration(&mut self, info: Info) -> Result<Node<'a>, ParseError<'a>> {
        let base_type = self.parse_declaration_spec()?;
        let mut compound_statements = vec![];
        let mut decl_count = 0;

        while let Some(Token { kind, info }) = self.tokens().peek() {
            let &info = info;
            if let TokenKind::Punctuation(";") = kind {
                break;
            }

            if decl_count > 0 {
                self.expect_next(TokenKind::Punctuation(","))?
            }
            decl_count += 1;

            let (identifier, decl_type) = self.parse_declarator(base_type.clone())?;
            let object = self
                .current_function()
                .get_or_allocate_local(identifier.into(), decl_type);

            if let Some(Token {
                kind: TokenKind::Punctuation("="),
                ..
            }) = self.tokens().peek()
            {
                self.tokens().next();

                let variable = Node::new(NodeKind::Variable(object), info)
                    .map_err(|e| self.err_type_error(e, info.index))?;
                let assignment = Node::new(
                    NodeKind::binary(
                        BinaryKind::Assign,
                        Box::new(variable),
                        Box::new(self.parse_assignment()?),
                    ),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?;
                let expr_stmt = Node::new(
                    NodeKind::unary(UnaryKind::ExprStatement, Box::new(assignment)),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?;
                compound_statements.push(expr_stmt);
            }
        }

        Node::new(
            NodeKind::CompoundStatement {
                kind: CompoundStatementKind::Block,
                nodes: compound_statements,
            },
            info,
        )
        .map_err(|e| self.err_type_error(e, info.index))
    }

    // stmt = "return" expr ";"
    //      | "if" "(" expr ")" stmt ("else" stmt)?
    //      | "for" "(" expr-stmt expr? ";" expr? ")" stmt
    //      | "while" "(" expr ")" stmt
    //      | "{" compound-stmt
    //      | expr-stmt
    fn parse_statement(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        match self.tokens().peek() {
            Some(Token {
                kind: TokenKind::Keyword("return"),
                info,
            }) => {
                let &info = info;
                self.tokens().next();

                let node = Node::new(
                    NodeKind::unary(UnaryKind::Return, Box::new(self.parse_expression()?)),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?;

                self.expect_next(TokenKind::Punctuation(";"))?;

                Ok(node)
            }
            Some(Token {
                kind: TokenKind::Keyword("if"),
                info,
            }) => {
                let &info = info;

                self.tokens().next();
                self.expect_next(TokenKind::Punctuation("("))?;
                let condition = Box::new(self.parse_expression()?);
                self.expect_next(TokenKind::Punctuation(")"))?;
                let then_block = Box::new(self.parse_statement()?);

                let else_block = if let Some(Token {
                    kind: TokenKind::Keyword("else"),
                    ..
                }) = self.tokens().peek()
                {
                    self.tokens().next();
                    Some(Box::new(self.parse_statement()?))
                } else {
                    None
                };

                Node::new(
                    NodeKind::If {
                        condition,
                        then_block,
                        else_block,
                    },
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))
            }
            Some(Token {
                kind: TokenKind::Keyword("for"),
                info,
            }) => {
                let &info = info;

                self.tokens().next();
                self.expect_next(TokenKind::Punctuation("("))?;

                let init = Some(Box::new(self.parse_expr_statement()?));

                let condition = if let Some(Token {
                    kind: TokenKind::Punctuation(";"),
                    ..
                }) = self.tokens().peek()
                {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                self.expect_next(TokenKind::Punctuation(";"))?;

                let increment = if let Some(Token {
                    kind: TokenKind::Punctuation(")"),
                    ..
                }) = self.tokens().peek()
                {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                self.expect_next(TokenKind::Punctuation(")"))?;

                let loop_block = Box::new(self.parse_statement()?);

                Node::new(
                    NodeKind::Loop {
                        init,
                        condition,
                        increment,
                        loop_block,
                    },
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))
            }
            Some(Token {
                kind: TokenKind::Keyword("while"),
                info,
            }) => {
                let &info = info;

                self.tokens().next();
                self.expect_next(TokenKind::Punctuation("("))?;
                let condition = Some(Box::new(self.parse_expression()?));
                self.expect_next(TokenKind::Punctuation(")"))?;
                let loop_block = Box::new(self.parse_statement()?);

                Node::new(
                    NodeKind::Loop {
                        init: None,
                        condition,
                        increment: None,
                        loop_block,
                    },
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))
            }
            Some(Token {
                kind: TokenKind::Punctuation("{"),
                info,
            }) => {
                let &info = info;

                self.tokens().next();
                Node::new(
                    NodeKind::CompoundStatement {
                        kind: CompoundStatementKind::Block,
                        nodes: self.parse_compound_statement()?,
                    },
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))
            }
            _ => self.parse_expr_statement(),
        }
    }

    // compound-stmt = (declaration | stmt)* "}"
    fn parse_compound_statement(&mut self) -> Result<Vec<Node<'a>>, ParseError<'a>> {
        let mut nodes = vec![];

        while let Some(Token { kind, .. }) = self.tokens().peek() {
            if let TokenKind::Punctuation("}") = kind {
                self.tokens().next();
                break;
            }

            let compound_statement = if let Some(Token {
                kind: TokenKind::Keyword(keyword),
                info,
            }) = self.tokens().peek()
                && Type::is_type_name(keyword)
            {
                self.parse_declaration(*info)
            } else {
                self.parse_statement()
            }?;

            nodes.push(compound_statement);
        }

        Ok(nodes)
    }

    // expr-stmt = expr? ";"
    fn parse_expr_statement(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        if let Some(Token {
            kind: TokenKind::Punctuation(";"),
            info,
        }) = self.tokens().peek()
        {
            self.tokens().next();
            return Node::new(
                NodeKind::CompoundStatement {
                    kind: CompoundStatementKind::Block,
                    nodes: vec![],
                },
                *info,
            )
            .map_err(|e| self.err_type_error(e, info.index));
        };

        let expression = self.parse_expression()?;
        let info = expression.info;
        let expr_stmt = Node::new(
            NodeKind::unary(UnaryKind::ExprStatement, Box::new(expression)),
            info,
        )
        .map_err(|e| self.err_type_error(e, info.index))?;
        self.expect_next(TokenKind::Punctuation(";"))?;

        Ok(expr_stmt)
    }

    // expr = assign
    fn parse_expression(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let assignment = self.parse_assignment()?;

        if let Some(Token {
            kind: TokenKind::Punctuation(","),
            info,
            ..
        }) = self.tokens.peek()
        {
            self.tokens.next();
            return Ok(Node::new(
                NodeKind::binary(
                    BinaryKind::Comma,
                    Box::new(assignment),
                    Box::new(self.parse_expression()?),
                ),
                *info,
            )
            .map_err(|e| self.err_type_error(e, info.index))?);
        }

        Ok(assignment)
    }

    // funcall = ident "(" (assign ("," assign)*)? ")"
    fn parse_function_call(
        &mut self,
        name: &'a str,
        info: Info,
    ) -> Result<Node<'a>, ParseError<'a>> {
        let mut args = vec![];

        while let Some(Token { kind, .. }) = self.tokens().peek() {
            if let TokenKind::Punctuation(")") = kind {
                break;
            }
            if !args.is_empty() {
                self.expect_next(TokenKind::Punctuation(","))?;
            }
            args.push(self.parse_assignment()?)
        }
        self.expect_next(TokenKind::Punctuation(")"))?;

        Node::new(NodeKind::FunctionCall { name, args }, info)
            .map_err(|e| self.err_type_error(e, info.index))
    }

    // postfix = primary ("[" expr "]")*
    fn parse_postfix(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_primary()?;

        while let Some(Token {
            kind: TokenKind::Punctuation("["),
            info,
        }) = self.tokens().peek()
        {
            self.tokens().next();
            let index = self.parse_expression()?;
            self.expect_next(TokenKind::Punctuation("]"))?;

            node = Node::new(
                NodeKind::unary(
                    UnaryKind::Deref,
                    Box::new(self.parse_addition(Box::new(node), Box::new(index), *info)?),
                ),
                *info,
            )
            .map_err(|e| self.err_type_error(e, info.index))?
        }

        Ok(node)
    }

    // primary = "(" "{" stmt+ "}" ")"
    //        | "(" expr ")"
    //        | "sizeof" unary
    //        | ident func-args?
    //        | str
    //        | num
    fn parse_primary(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let Some(Token { kind, info }) = self.tokens().next() else {
            return Err(self.err_unusual_end_of_tokens());
        };
        let &info = info;
        match kind {
            TokenKind::Punctuation("(") => {
                if let Some(Token {
                    kind: TokenKind::Punctuation("{"),
                    ..
                }) = self.tokens().peek()
                {
                    self.tokens().next();

                    let nodes = self.parse_compound_statement()?;
                    self.expect_next(TokenKind::Punctuation(")"))?;
                    return Ok(Node::new(
                        NodeKind::CompoundStatement {
                            kind: CompoundStatementKind::StatementExpr,
                            nodes,
                        },
                        info,
                    )
                    .map_err(|e| self.err_type_error(e, info.index))?);
                }

                let node = self.parse_expression()?;
                self.expect_next(TokenKind::Punctuation(")"))?;
                Ok(node)
            }
            TokenKind::Identifier(name) => {
                if let Some(Token {
                    kind: TokenKind::Punctuation("("),
                    ..
                }) = self.tokens().peek()
                {
                    self.tokens().next();
                    return self.parse_function_call(name, info);
                }

                let object = self.get_variable(name, info)?;
                Ok(Node::new(NodeKind::Variable(object), info)
                    .map_err(|e| self.err_type_error(e, info.index))?)
            }
            TokenKind::String(string) => {
                let mut bytes: Vec<_> = string.chars().map(|c| c as u8).collect();
                bytes.push(0); // null terminate
                let object = Object::global_variable_with_data(
                    util::unique_name().into(),
                    Type::array_of(&Type::char(), bytes.len()),
                    bytes,
                );
                let object = self
                    .translation_unit
                    .get_or_allocate(object.name.clone(), object);
                Ok(Node::new(NodeKind::Variable(object), info)
                    .map_err(|e| self.err_type_error(e, info.index))?)
            }
            TokenKind::Keyword("sizeof") => {
                let size = self.parse_unary()?.node_type.size;

                Ok(Node::new(NodeKind::Numeric(size), info)
                    .map_err(|e| self.err_type_error(e, info.index))?)
            }
            TokenKind::Numeric(num) => Ok(Node::new(NodeKind::Numeric(*num), info)
                .map_err(|e| self.err_type_error(e, info.index))?),
            _ => Err(self.emit_error(ParseErrorKind::ExpectedExpression, info.index)),
        }
    }

    // unary = ("*" | "-" | "*" | "&" ) unary
    //         | postfix
    fn parse_unary(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let Some(Token { kind, info }) = self.tokens().peek() else {
            return Err(self.err_unusual_end_of_tokens());
        };
        let &info = info;

        match kind {
            TokenKind::Punctuation("+") => {
                self.tokens().next();
                self.parse_unary()
            }
            TokenKind::Punctuation("-") => {
                self.tokens().next();
                Ok(Node::new(
                    NodeKind::unary(UnaryKind::Negate, Box::new(self.parse_unary()?)),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?)
            }
            TokenKind::Punctuation("&") => {
                self.tokens().next();
                Ok(Node::new(
                    NodeKind::unary(UnaryKind::Address, Box::new(self.parse_unary()?)),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?)
            }
            TokenKind::Punctuation("*") => {
                self.tokens().next();

                Ok(Node::new(
                    NodeKind::unary(UnaryKind::Deref, Box::new(self.parse_unary()?)),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?)
            }
            _ => self.parse_postfix(),
        }
    }

    // mul = unary ("*" unary | "/" unary)*
    fn parse_multiplicative(&mut self) -> Result<Node<'a>, ParseError<'a>> {
        let mut node = self.parse_unary()?;

        while let Some(Token { kind, info }) = self.tokens().peek() {
            let &info = info;
            match kind {
                TokenKind::Punctuation("*") => {
                    self.tokens().next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::Multiply,
                            Box::new(node),
                            Box::new(self.parse_unary()?),
                        ),
                        info,
                    )
                    .map_err(|e| self.err_type_error(e, info.index))?
                }
                TokenKind::Punctuation("/") => {
                    self.tokens().next();
                    node = Node::new(
                        NodeKind::binary(
                            BinaryKind::Divide,
                            Box::new(node),
                            Box::new(self.parse_unary()?),
                        ),
                        info,
                    )
                    .map_err(|e| self.err_type_error(e, info.index))?
                }
                _ => break,
            }
        }

        Ok(node)
    }

    fn parse_addition(
        &self,
        lhs: Box<Node<'a>>,
        rhs: Box<Node<'a>>,
        info: Info,
    ) -> Result<Node<'a>, ParseError<'a>> {
        match (&&lhs.node_type.kind, &rhs.node_type.kind) {
            (TypeKind::Integer(..), TypeKind::Integer(..)) => {
                Ok(Node::new(NodeKind::binary(BinaryKind::Add, lhs, rhs), info)
                    .map_err(|e| self.err_type_error(e, info.index))?)
            }
            (TypeKind::Integer(..), TypeKind::Derived { to, .. })
            | (TypeKind::Derived { to, .. }, TypeKind::Integer(..)) => {
                let derived_element_size = to.size;
                let (lhs, rhs) = match rhs.node_type.kind {
                    TypeKind::Derived { .. } => (rhs, lhs),
                    _ => (lhs, rhs),
                };

                let rhs = Node::new(
                    NodeKind::binary(
                        BinaryKind::Multiply,
                        rhs,
                        Box::new(
                            Node::new(NodeKind::Numeric(derived_element_size), info)
                                .map_err(|e| self.err_type_error(e, info.index))?,
                        ),
                    ),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?;

                Ok(
                    Node::new(NodeKind::binary(BinaryKind::Add, lhs, Box::new(rhs)), info)
                        .map_err(|e| self.err_type_error(e, info.index))?,
                )
            }
            (TypeKind::Derived { .. }, TypeKind::Derived { .. }) | _ => {
                Err(TypeError::InvalidOperands).map_err(|e| self.err_type_error(e, info.index))
            }
        }
    }

    fn parse_subtraction(
        &self,
        lhs: Box<Node<'a>>,
        rhs: Box<Node<'a>>,
        info: Info,
    ) -> Result<Node<'a>, ParseError<'a>> {
        match (&lhs.node_type.kind, &rhs.node_type.kind) {
            (TypeKind::Integer(..), TypeKind::Integer(..)) => Ok(Node::new(
                NodeKind::binary(BinaryKind::Subtract, lhs, rhs),
                info,
            )
            .map_err(|e| self.err_type_error(e, info.index))?),
            (TypeKind::Derived { to, .. }, TypeKind::Integer(..)) => {
                let rhs = Node::new(
                    NodeKind::binary(
                        BinaryKind::Multiply,
                        rhs,
                        Box::new(
                            Node::new(NodeKind::Numeric(to.size), info)
                                .map_err(|e| self.err_type_error(e, info.index))?,
                        ),
                    ),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?;

                Ok(Node::new(
                    NodeKind::binary(BinaryKind::Subtract, lhs, Box::new(rhs)),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?)
            }
            (TypeKind::Derived { .. }, TypeKind::Derived { .. }) => {
                let lhs_size = lhs.node_type.size;
                let difference = Node::new_of_type(
                    NodeKind::binary(BinaryKind::Subtract, lhs, rhs),
                    info,
                    Type::integer(),
                );

                Ok(Node::new(
                    NodeKind::binary(
                        BinaryKind::Divide,
                        Box::new(difference),
                        Box::new(
                            Node::new(NodeKind::Numeric(lhs_size), info)
                                .map_err(|e| self.err_type_error(e, info.index))?,
                        ),
                    ),
                    info,
                )
                .map_err(|e| self.err_type_error(e, info.index))?)
            }
            _ => Err(TypeError::InvalidOperands).map_err(|e| self.err_type_error(e, info.index)),
        }
    }

    fn get_variable(&mut self, name: &str, info: Info) -> Result<Rc<Object<'a>>, ParseError<'a>> {
        self.current_function()
            .get_local(name)
            .or_else(|| self.translation_unit.get_object(name))
            .ok_or_else(|| self.emit_error(ParseErrorKind::UndefinedVariable, info.index))
    }

    fn err_type_error(&self, type_error: TypeError, index: usize) -> ParseError<'a> {
        let kind = match type_error {
            TypeError::InvalidPointerDeref => ParseErrorKind::InvalidPointerDeref,
            TypeError::InvalidOperands => ParseErrorKind::InvalidOperands,
            TypeError::NonLValueAssignment => ParseErrorKind::NonLValueAssignment,
            TypeError::StatementExprReturnsVoid => ParseErrorKind::StatementExprReturnsVoid,
        };

        self.emit_error(kind, index)
    }

    fn emit_error(&self, kind: ParseErrorKind<'a>, index: usize) -> ParseError<'a> {
        ParseError {
            file: self.file,
            input: self.input,
            kind,
            index,
        }
    }

    fn err_unusual_end_of_tokens(&self) -> ParseError<'a> {
        self.emit_error(ParseErrorKind::UnusualEndOfTokens, self.input.len())
    }

    fn err_unexpected_token(&self, expected_kind: TokenKind<'a>, index: usize) -> ParseError<'a> {
        self.emit_error(
            ParseErrorKind::UnexpectedToken {
                expected: expected_kind,
            },
            index,
        )
    }
}
