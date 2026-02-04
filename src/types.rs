use std::rc::Rc;

use crate::parser::{BinaryKind, NodeKind, UnaryKind};

#[derive(Debug)]
pub enum Type {
    None,
    Integer,
    Pointer(Rc<Type>),
}

thread_local! {
    static NONE: Rc<Type> = Rc::new(Type::None);
    static INTEGER: Rc<Type> = Rc::new(Type::Integer);
}

impl Type {
    fn integer() -> Rc<Self> {
        INTEGER.with(|t| t.clone())
    }

    fn pointer_to(to: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self::Pointer(to.clone()))
    }

    fn none() -> Rc<Self> {
        NONE.with(|t| t.clone())
    }

    pub fn of(kind: &NodeKind) -> Rc<Self> {
        match kind {
            NodeKind::Numeric { .. } => Self::integer(),
            NodeKind::Variable { .. } => Self::integer(),
            NodeKind::Binary { kind, lhs, .. } => match kind {
                BinaryKind::Add
                | BinaryKind::Subtract
                | BinaryKind::Multiply
                | BinaryKind::Divide
                | BinaryKind::Assign => lhs.node_type.clone(),
                BinaryKind::Equal => Self::integer(),
                BinaryKind::NotEqual => Self::integer(),
                BinaryKind::LessThan => Self::integer(),
                BinaryKind::LessThanEqual => Self::integer(),
            },
            NodeKind::Unary { kind, lhs, .. } => match kind {
                UnaryKind::Negate => lhs.node_type.clone(),
                UnaryKind::Address => Self::pointer_to(&lhs.node_type),
                UnaryKind::Deref => match lhs.node_type.as_ref() {
                    Type::Pointer(to) => to.clone(),
                    _ => Self::integer(),
                },
                UnaryKind::Return => Self::none(),
            },
            NodeKind::ExprStatement { .. } => Self::none(),
            NodeKind::Block { .. } => Self::none(),
            NodeKind::If { .. } => Self::none(),
            NodeKind::Loop { .. } => Self::none(),
        }
    }
}
