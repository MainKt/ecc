use std::rc::Rc;

use crate::parser::{BinaryKind, NodeKind, UnaryKind};

#[derive(Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub size: Option<usize>,
}

#[derive(Debug)]
pub enum DerivedKind {
    Array,
    Pointer,
}

#[derive(Debug)]
pub enum TypeKind {
    None,
    Integer,
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
        size: Some(0),
    });
    static INTEGER: Rc<Type> = Rc::new(Type {
        kind: TypeKind::Integer,
        size: Some(8)
    });
}

#[derive(Debug)]
pub enum TypeError {
    InvalidPointerDeref,
    InvalidOperands,
}

impl Type {
    pub fn integer() -> Rc<Self> {
        INTEGER.with(|t| t.clone())
    }

    pub fn pointer_to(to: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self {
            kind: TypeKind::Derived {
                kind: DerivedKind::Pointer,
                to: to.clone(),
            },
            size: Some(8),
        })
    }

    pub fn function(return_type: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self {
            kind: TypeKind::Function {
                return_type: return_type.clone(),
                params: vec![],
            },
            size: Some(0),
        })
    }

    pub fn none() -> Rc<Self> {
        NONE.with(|t| t.clone())
    }

    pub fn of(kind: &NodeKind) -> Result<Rc<Self>, TypeError> {
        match kind {
            NodeKind::Numeric { .. } => Ok(Self::integer()),
            NodeKind::Variable(object) => Ok(object.borrow().object_type.clone()),
            NodeKind::Binary { kind, lhs, .. } => match kind {
                BinaryKind::Add
                | BinaryKind::Subtract
                | BinaryKind::Multiply
                | BinaryKind::Divide
                | BinaryKind::Assign => Ok(lhs.node_type.clone()),
                BinaryKind::Equal => Ok(Self::integer()),
                BinaryKind::NotEqual => Ok(Self::integer()),
                BinaryKind::LessThan => Ok(Self::integer()),
                BinaryKind::LessThanEqual => Ok(Self::integer()),
            },
            NodeKind::Unary { kind, lhs, .. } => match kind {
                UnaryKind::Negate => Ok(lhs.node_type.clone()),
                UnaryKind::Address => Ok(Self::pointer_to(&lhs.node_type)),
                UnaryKind::Deref => match &lhs.node_type.kind {
                    TypeKind::Derived { to, .. } => Ok(to.clone()),
                    _ => Err(TypeError::InvalidPointerDeref),
                },
                UnaryKind::Return => Ok(Self::none()),
            },
            NodeKind::ExprStatement { .. } => Ok(Self::none()),
            NodeKind::Block { .. } => Ok(Self::none()),
            NodeKind::If { .. } => Ok(Self::none()),
            NodeKind::Loop { .. } => Ok(Self::none()),
            NodeKind::FunctionCall { .. } => Ok(Self::integer()),
        }
    }
}
