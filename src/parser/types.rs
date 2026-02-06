use std::rc::Rc;

use crate::parser::{BinaryKind, NodeKind, UnaryKind};

use super::{CompoundStatementKind, Node};

#[derive(Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub size: usize,
}

#[derive(Debug)]
pub enum DerivedKind {
    Array { length: usize },
    Pointer,
}

#[derive(Debug)]
pub enum Integer {
    Char,
    Int,
}

#[derive(Debug)]
pub enum TypeKind {
    None,
    Integer(Integer),
    Derived {
        kind: DerivedKind,
        to: Rc<Type>,
    },
    Function {
        params: Vec<Rc<Type>>,
        return_type: Rc<Type>,
    },
}

thread_local! {
    static NONE: Rc<Type> = Rc::new(Type {
        kind: TypeKind::None,
        size: 0,
    });

    static INTEGER: Rc<Type> = Rc::new(Type {
        kind: TypeKind::Integer(Integer::Int),
        size: 8
    });

    static CHAR: Rc<Type> = Rc::new(Type {
        kind: TypeKind::Integer(Integer::Char),
        size: 1
    });
}

#[derive(Debug)]
pub enum TypeError {
    InvalidPointerDeref,
    InvalidOperands,
    NonLValueAssignment,
    StatementExprReturnsVoid,
}

impl Type {
    pub fn char() -> Rc<Self> {
        CHAR.with(|t| t.clone())
    }

    pub fn integer() -> Rc<Self> {
        INTEGER.with(|t| t.clone())
    }

    pub fn is_type_name(name: &str) -> bool {
        ["int", "char"].iter().any(|&t| t == name)
    }

    pub fn pointer_to(to: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self {
            kind: TypeKind::Derived {
                kind: DerivedKind::Pointer,
                to: to.clone(),
            },
            size: 8,
        })
    }

    pub fn array_of(of: &Rc<Self>, length: usize) -> Rc<Self> {
        Rc::new(Self {
            kind: TypeKind::Derived {
                kind: DerivedKind::Array { length },
                to: of.clone(),
            },
            size: of.size * length,
        })
    }

    pub fn function(return_type: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self {
            kind: TypeKind::Function {
                return_type: return_type.clone(),
                params: vec![],
            },
            size: 0,
        })
    }

    pub fn none() -> Rc<Self> {
        NONE.with(|t| t.clone())
    }

    pub fn of(kind: &NodeKind) -> Result<Rc<Self>, TypeError> {
        match kind {
            NodeKind::Numeric { .. } => Ok(Self::integer()),
            NodeKind::Variable(object) => Ok(object.object_type.clone()),
            NodeKind::Binary { kind, rhs, lhs, .. } => match kind {
                BinaryKind::Add
                | BinaryKind::Subtract
                | BinaryKind::Multiply
                | BinaryKind::Divide => Ok(lhs.node_type.clone()),
                BinaryKind::Assign => {
                    if let TypeKind::Derived {
                        kind: DerivedKind::Array { .. },
                        ..
                    } = lhs.node_type.kind
                    {
                        Err(TypeError::NonLValueAssignment)
                    } else {
                        Ok(lhs.node_type.clone())
                    }
                }
                BinaryKind::Equal => Ok(Self::integer()),
                BinaryKind::NotEqual => Ok(Self::integer()),
                BinaryKind::LessThan => Ok(Self::integer()),
                BinaryKind::LessThanEqual => Ok(Self::integer()),
                BinaryKind::Comma => Ok(rhs.node_type.clone()),
            },
            NodeKind::Unary { kind, lhs, .. } => match kind {
                UnaryKind::Negate => Ok(lhs.node_type.clone()),
                UnaryKind::Address => {
                    if let TypeKind::Derived {
                        kind: DerivedKind::Array { .. },
                        to,
                    } = &lhs.node_type.kind
                    {
                        Ok(Self::pointer_to(to))
                    } else {
                        Ok(Self::pointer_to(&lhs.node_type))
                    }
                }
                UnaryKind::Deref => match &lhs.node_type.kind {
                    TypeKind::Derived { to, .. } => Ok(to.clone()),
                    _ => Err(TypeError::InvalidPointerDeref),
                },
                UnaryKind::Return => Ok(Self::none()),
                UnaryKind::ExprStatement => Ok(lhs.node_type.clone()),
            },
            NodeKind::If { .. } => Ok(Self::none()),
            NodeKind::Loop { .. } => Ok(Self::none()),
            NodeKind::FunctionCall { .. } => Ok(Self::integer()),
            NodeKind::CompoundStatement { kind, nodes } => match kind {
                CompoundStatementKind::Block => Ok(Self::none()),
                CompoundStatementKind::StatementExpr => {
                    if let Some(Node {
                        kind:
                            NodeKind::Unary {
                                kind: UnaryKind::ExprStatement,
                                ..
                            },
                        node_type,
                        ..
                    }) = nodes.last()
                    {
                        Ok(node_type.clone())
                    } else {
                        Err(TypeError::StatementExprReturnsVoid)
                    }
                }
            },
        }
    }
}
