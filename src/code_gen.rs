use std::borrow::Cow;

use crate::{
    parser::{BinaryKind, Function, Node, UnaryKind},
    util,
};

pub struct CodeGen<'a> {
    depth: usize,
    instructions: Vec<Cow<'a, str>>,
    stack_size: usize,
}

impl<'a> CodeGen<'a> {
    pub fn new() -> Self {
        Self {
            depth: 0,
            instructions: vec![],
            stack_size: 0,
        }
    }

    pub fn generate_assembly(mut self, mut f: Function) -> Vec<Cow<'a, str>> {
        self.stack_size = f.stack_size();
        self.generate_assembly_for_ast(&f.ast())
    }

    fn generate_assembly_for_ast(mut self, node: &Node) -> Vec<Cow<'a, str>> {
        self.instructions.push("  .globl main".into());
        self.instructions.push("main:".into());

        self.instructions.push("  push %rbp".into());
        self.instructions.push("  mov %rsp, %rbp".into());
        self.instructions
            .push(format!("  sub ${}, %rsp", self.stack_size).into());

        self.traverse(&node);
        assert!(self.depth == 0);

        self.instructions.push(".L.return:".into());
        self.instructions.push("  mov %rbp, %rsp".into());
        self.instructions.push("  pop %rbp".into());
        self.instructions.push("  ret".into());

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

    fn generate_address(&mut self, node: &Node) {
        let Node::Variable(object) = node else {
            util::errx("lvalue assigment")
        };

        self.instructions
            .push(format!("  lea {}(%rbp), %rax", object.borrow().offset).into());
    }

    fn traverse_children(&mut self, rhs: &Node, lhs: &Node) {
        self.traverse(rhs);
        self.push();
        self.traverse(lhs);
        self.pop();
    }

    fn traverse(&mut self, node: &Node) {
        match node {
            Node::Binary { kind, lhs, rhs } => match kind {
                BinaryKind::Add => {
                    self.traverse_children(rhs, lhs);
                    self.instructions.push("  add %rdi, %rax".into())
                }
                BinaryKind::Subtract => {
                    self.traverse_children(rhs, lhs);
                    self.instructions.push("  sub %rdi, %rax".into())
                }
                BinaryKind::Multiply => {
                    self.traverse_children(rhs, lhs);
                    self.instructions.push("  imul %rdi, %rax".into())
                }
                BinaryKind::Divide => {
                    self.traverse_children(rhs, lhs);
                    self.instructions.push("  cqo".into());
                    self.instructions.push("  idiv %rdi".into());
                }
                BinaryKind::Equal => {
                    self.traverse_children(rhs, lhs);
                    self.instructions.push("  cmp %rdi, %rax".into());
                    self.instructions.push("  sete %al".into());
                    self.instructions.push("  movzb %al, %rax".into());
                }
                BinaryKind::NotEqual => {
                    self.traverse_children(rhs, lhs);
                    self.instructions.push("  cmp %rdi, %rax".into());
                    self.instructions.push("  setne %al".into());
                    self.instructions.push("  movzb %al, %rax".into());
                }
                BinaryKind::LessThan => {
                    self.traverse_children(rhs, lhs);
                    self.instructions.push("  cmp %rdi, %rax".into());
                    self.instructions.push("  setl %al".into());
                    self.instructions.push("  movzb %al, %rax".into());
                }
                BinaryKind::LessThanEqual => {
                    self.traverse_children(rhs, lhs);
                    self.instructions.push("  cmp %rdi, %rax".into());
                    self.instructions.push("  setle %al".into());
                    self.instructions.push("  movzb %al, %rax".into());
                }
                BinaryKind::Assign => {
                    self.generate_address(lhs);
                    self.push();
                    self.traverse(rhs);
                    self.pop();
                    self.instructions.push("  mov %rax, (%rdi)".into())
                }
            },
            Node::Unary { kind, lhs } => {
                self.traverse(lhs);
                match kind {
                    UnaryKind::Negate => self.instructions.push("  neg %rax".into()),
                    UnaryKind::Return => self.instructions.push("  jmp .L.return".into()),
                }
            }
            Node::Numeric(num) => self.instructions.push(format!("  mov ${num}, %rax").into()),
            Node::ExprStatement { statements } => {
                statements
                    .iter()
                    .for_each(|statement| self.traverse(statement));
                assert!(self.depth == 0);
            }
            Node::Variable(_) => {
                self.generate_address(node);
                self.instructions.push("  mov (%rax), %rax".into())
            }
        }
    }
}
