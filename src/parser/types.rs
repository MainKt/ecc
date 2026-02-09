use std::{borrow::Cow, collections::HashMap, rc::Rc};

use crate::parser::{BinaryKind, NodeKind, UnaryKind};

use super::{CompoundStatementKind, Member, Node};

#[derive(Debug)]
pub struct Type<'a> {
    pub kind: TypeKind<'a>,
    pub size: usize,
    pub alignment: usize,
}

#[derive(Debug)]
pub enum DerivedKind {
    Array { length: usize },
    Pointer,
}

#[derive(Debug)]
pub enum Integer {
    Char,
    Short,
    Int,
    Long,
}

#[derive(Debug)]
pub enum CompositeKind {
    Struct,
    Union,
}

#[derive(Debug)]
pub enum TypeKind<'a> {
    None,
    Void,
    Integer(Integer),
    Derived {
        kind: DerivedKind,
        to: Rc<Type<'a>>,
    },
    Function {
        params: Vec<Rc<Type<'a>>>,
        return_type: Rc<Type<'a>>,
    },
    Composite {
        kind: CompositeKind,
        members: HashMap<Cow<'a, str>, Rc<Member<'a>>>,
    },
}

thread_local! {
    static NONE: Rc<Type<'static>> = Rc::new(Type {
        kind: TypeKind::None,
        size: 0,
        alignment: 0,
    });

    static VOID: Rc<Type<'static>> = Rc::new(Type {
        kind: TypeKind::Void,
        size: 1,
        alignment: 1,
    });

    static CHAR: Rc<Type<'static>> = Rc::new(Type {
        kind: TypeKind::Integer(Integer::Char),
        size: 1,
        alignment: 1,
    });

    static SHORT: Rc<Type<'static>> = Rc::new(Type {
        kind: TypeKind::Integer(Integer::Short),
        size: 2,
        alignment: 2,
    });

    static INT: Rc<Type<'static>> = Rc::new(Type {
        kind: TypeKind::Integer(Integer::Int),
        size: 4,
        alignment: 4,
    });

    static LONG: Rc<Type<'static>> = Rc::new(Type {
        kind: TypeKind::Integer(Integer::Long),
        size: 8,
        alignment: 8,
    });
}

#[derive(Debug)]
pub enum TypeError<'a> {
    InvalidPointerDeref,
    InvalidOperands,
    NonLValueAssignment,
    StatementExprReturnsVoid,
    MemberAccessOnNonStruct,
    InvalidStructMember { member: Cow<'a, str> },
}

impl<'a> Type<'a> {
    pub fn char() -> Rc<Self> {
        CHAR.with(|t| t.clone())
    }

    pub fn short() -> Rc<Self> {
        SHORT.with(|t| t.clone())
    }

    pub fn integer() -> Rc<Self> {
        INT.with(|t| t.clone())
    }

    pub fn long() -> Rc<Self> {
        LONG.with(|t| t.clone())
    }

    pub fn union_type(members: Vec<(Cow<'a, str>, Rc<Type<'a>>)>) -> Rc<Self> {
        let members: HashMap<_, _> = members
            .into_iter()
            .map(|(name, member_type)| {
                (
                    name,
                    Rc::new(Member {
                        offset: 0,
                        member_type,
                    }),
                )
            })
            .collect();

        let alignment = members
            .values()
            .max_by_key(|m| m.member_type.alignment)
            .map(|m| m.member_type.alignment)
            .unwrap_or(0);
        let size = members
            .values()
            .max_by_key(|m| m.member_type.size)
            .map(|m| m.member_type.size)
            .unwrap_or(0);

        Rc::new(Self {
            kind: TypeKind::Composite {
                kind: CompositeKind::Union,
                members,
            },
            size: size.next_multiple_of(alignment),
            alignment,
        })
    }

    pub fn struct_type(members: Vec<(Cow<'a, str>, Rc<Type<'a>>)>) -> Rc<Self> {
        let mut offset: usize = 0;
        let mut alignment: usize = 1;
        let members = members
            .into_iter()
            .map(|(name, member_type)| {
                offset = offset.next_multiple_of(member_type.alignment);
                let member = Rc::new(Member {
                    offset,
                    member_type,
                });
                offset += member.member_type.size;
                if alignment < member.member_type.alignment {
                    alignment = member.member_type.alignment
                }
                (name, member)
            })
            .collect();
        Rc::new(Self {
            kind: TypeKind::Composite {
                kind: CompositeKind::Struct,
                members,
            },
            size: offset.next_multiple_of(alignment),
            alignment,
        })
    }

    pub fn is_type_name(name: &str) -> bool {
        ["long", "int", "short", "char", "struct", "union", "void"]
            .iter()
            .any(|&t| t == name)
    }

    pub fn pointer_to(to: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self {
            kind: TypeKind::Derived {
                kind: DerivedKind::Pointer,
                to: to.clone(),
            },
            size: 8,
            alignment: 8,
        })
    }

    pub fn array_of(of: &Rc<Self>, length: usize) -> Rc<Self> {
        Rc::new(Self {
            kind: TypeKind::Derived {
                kind: DerivedKind::Array { length },
                to: of.clone(),
            },
            size: of.size * length,
            alignment: of.alignment,
        })
    }

    pub fn function(return_type: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self {
            kind: TypeKind::Function {
                return_type: return_type.clone(),
                params: vec![],
            },
            size: 0,
            alignment: 0,
        })
    }

    pub fn void() -> Rc<Self> {
        VOID.with(|t| t.clone())
    }

    pub fn none() -> Rc<Self> {
        NONE.with(|t| t.clone())
    }

    pub fn of(kind: &NodeKind<'a>) -> Result<Rc<Self>, TypeError<'a>> {
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
                        Ok(Self::pointer_to(&to))
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
            NodeKind::FunctionCall { .. } => Ok(Self::long()),
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
            NodeKind::MemberAccess { member, .. } => Ok(member.member_type.clone()),
        }
    }
}
