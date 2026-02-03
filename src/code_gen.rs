use std::borrow::Cow;

use crate::parser::{BinaryKind, Node, UnaryKind};

pub struct CodeGen<'a> {
    depth: usize,
    instructions: Vec<Cow<'a, str>>,
}

impl<'a> CodeGen<'a> {
    pub fn new() -> Self {
        Self {
            depth: 0,
            instructions: vec![],
        }
    }

    pub fn generate_assembly(mut self, node: &Node) -> Vec<Cow<'a, str>> {
        self.traverse(&node);
        self.instructions
    }

    fn push(&mut self) {
        self.instructions.push("  push %rax".into());
        self.depth += 1;
    }

    fn pop(&mut self) {
        self.instructions.push("  pop %rdi".into());
        self.depth -= 1;
    }

    fn traverse_children(&mut self, rhs: &Node, lhs: &Node) {
        self.traverse(rhs);
        self.push();
        self.traverse(lhs);
        self.pop();
    }

    fn traverse(&mut self, node: &Node) {
        match node {
            Node::Binary { kind, lhs, rhs } => {
                self.traverse_children(rhs, lhs);
                match kind {
                    BinaryKind::Add => self.instructions.push("  add %rdi, %rax".into()),
                    BinaryKind::Subtract => self.instructions.push("  sub %rdi, %rax".into()),
                    BinaryKind::Multiply => self.instructions.push("  imul %rdi, %rax".into()),
                    BinaryKind::Divide => {
                        self.instructions.push("  cqo".into());
                        self.instructions.push("  idiv %rdi".into());
                    }
                    BinaryKind::Equal => {
                        self.instructions.push("  cmp %rdi, %rax".into());
                        self.instructions.push("  sete %al".into());
                        self.instructions.push("  movzb %al, %rax".into());
                    }
                    BinaryKind::NotEqual => {
                        self.instructions.push("  cmp %rdi, %rax".into());
                        self.instructions.push("  setne %al".into());
                        self.instructions.push("  movzb %al, %rax".into());
                    }
                    BinaryKind::LessThan => {
                        self.instructions.push("  cmp %rdi, %rax".into());
                        self.instructions.push("  setl %al".into());
                        self.instructions.push("  movzb %al, %rax".into());
                    }
                    BinaryKind::LessThanEqual => {
                        self.instructions.push("  cmp %rdi, %rax".into());
                        self.instructions.push("  setle %al".into());
                        self.instructions.push("  movzb %al, %rax".into());
                    }
                }
            }
            Node::Unary { kind, lhs } => {
                self.traverse(lhs);
                match kind {
                    UnaryKind::Negate => self.instructions.push("  neg %rax".into()),
                }
            }
            Node::Numeric(num) => self.instructions.push(format!("  mov ${num}, %rax").into()),
        }
    }
}
