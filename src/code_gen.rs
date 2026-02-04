use std::borrow::Cow;

use crate::parser::{BinaryKind, Function, Node, NodeKind, UnaryKind};

#[derive(Debug)]
pub enum CodeGenErrorKind {
    NonLValueAssignment { index: usize },
}

#[derive(Debug)]
pub struct CodeGenError<'a> {
    input: &'a str,
    kind: CodeGenErrorKind,
}

impl<'a> std::fmt::Display for CodeGenError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            CodeGenErrorKind::NonLValueAssignment { index } => {
                writeln!(f, "{}", self.input)?;
                write!(f, "{:>width$}^ ", "", width = index)?;
                write!(f, "not an lvalue")
            }
        }
    }
}

pub struct CodeGen<'a> {
    input: &'a str,
    depth: usize,
    instructions: Vec<Cow<'a, str>>,
    stack_size: usize,
    block_count: usize,
}

impl<'a> CodeGen<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            depth: 0,
            instructions: vec![],
            stack_size: 0,
            block_count: 0,
        }
    }

    fn next_block_number(&mut self) -> usize {
        self.block_count += 1;
        self.block_count
    }

    pub fn generate_assembly(
        mut self,
        f: &Function,
    ) -> Result<Vec<Cow<'a, str>>, CodeGenError<'a>> {
        self.stack_size = f.stack_size();
        self.generate_assembly_for_ast(f.body())
    }

    fn generate_assembly_for_ast(
        mut self,
        node: &Node,
    ) -> Result<Vec<Cow<'a, str>>, CodeGenError<'a>> {
        self.instructions.push("  .globl main".into());
        self.instructions.push("main:".into());

        self.instructions.push("  push %rbp".into());
        self.instructions.push("  mov %rsp, %rbp".into());
        self.instructions
            .push(format!("  sub ${}, %rsp", self.stack_size).into());

        self.traverse(&node)?;

        self.instructions.push(".L.return:".into());
        self.instructions.push("  mov %rbp, %rsp".into());
        self.instructions.push("  pop %rbp".into());
        self.instructions.push("  ret".into());

        Ok(self.instructions)
    }

    fn push(&mut self) {
        self.instructions.push("  push %rax".into());
        self.depth += 1;
    }

    fn pop(&mut self) {
        self.instructions.push("  pop %rdi".into());
        self.depth -= 1;
    }

    fn generate_address(&mut self, Node { kind, info, .. }: &Node) -> Result<(), CodeGenError<'a>> {
        match kind {
            NodeKind::Variable(object, ..) => {
                self.instructions
                    .push(format!("  lea {}(%rbp), %rax", object.borrow().offset).into());

                Ok(())
            }
            NodeKind::Unary {
                kind: UnaryKind::Deref,
                lhs,
                ..
            } => self.traverse(lhs),
            _ => Err(self.err_non_lvalue_assigment(info.index)),
        }
    }

    fn traverse_children(&mut self, rhs: &Node, lhs: &Node) -> Result<(), CodeGenError<'a>> {
        self.traverse(rhs)?;
        self.push();
        self.traverse(lhs)?;
        self.pop();

        Ok(())
    }

    fn traverse(&mut self, node: &Node) -> Result<(), CodeGenError<'a>> {
        match &node.kind {
            NodeKind::Binary { kind, lhs, rhs, .. } => match kind {
                BinaryKind::Add => {
                    self.traverse_children(rhs, lhs)?;
                    self.instructions.push("  add %rdi, %rax".into())
                }
                BinaryKind::Subtract => {
                    self.traverse_children(rhs, lhs)?;
                    self.instructions.push("  sub %rdi, %rax".into())
                }
                BinaryKind::Multiply => {
                    self.traverse_children(rhs, lhs)?;
                    self.instructions.push("  imul %rdi, %rax".into())
                }
                BinaryKind::Divide => {
                    self.traverse_children(rhs, lhs)?;
                    self.instructions.push("  cqo".into());
                    self.instructions.push("  idiv %rdi".into());
                }
                BinaryKind::Equal => {
                    self.traverse_children(rhs, lhs)?;
                    self.instructions.push("  cmp %rdi, %rax".into());
                    self.instructions.push("  sete %al".into());
                    self.instructions.push("  movzb %al, %rax".into());
                }
                BinaryKind::NotEqual => {
                    self.traverse_children(rhs, lhs)?;
                    self.instructions.push("  cmp %rdi, %rax".into());
                    self.instructions.push("  setne %al".into());
                    self.instructions.push("  movzb %al, %rax".into());
                }
                BinaryKind::LessThan => {
                    self.traverse_children(rhs, lhs)?;
                    self.instructions.push("  cmp %rdi, %rax".into());
                    self.instructions.push("  setl %al".into());
                    self.instructions.push("  movzb %al, %rax".into());
                }
                BinaryKind::LessThanEqual => {
                    self.traverse_children(rhs, lhs)?;
                    self.instructions.push("  cmp %rdi, %rax".into());
                    self.instructions.push("  setle %al".into());
                    self.instructions.push("  movzb %al, %rax".into());
                }
                BinaryKind::Assign => {
                    self.generate_address(lhs)?;
                    self.push();
                    self.traverse(rhs)?;
                    self.pop();
                    self.instructions.push("  mov %rax, (%rdi)".into())
                }
            },
            NodeKind::Unary { kind, lhs, .. } => match kind {
                UnaryKind::Negate => {
                    self.traverse(lhs)?;
                    self.instructions.push("  neg %rax".into())
                }
                UnaryKind::Return => {
                    self.traverse(lhs)?;
                    self.instructions.push("  jmp .L.return".into())
                }
                UnaryKind::Address => self.generate_address(lhs)?,
                UnaryKind::Deref => {
                    self.traverse(lhs)?;
                    self.instructions.push("  mov (%rax), %rax".into())
                }
            },
            NodeKind::Numeric(value) => self
                .instructions
                .push(format!("  mov ${value}, %rax").into()),
            NodeKind::ExprStatement { statements } => {
                statements
                    .iter()
                    .try_for_each(|statement| self.traverse(statement))?;
                assert!(self.depth == 0);
            }
            NodeKind::Variable { .. } => {
                self.generate_address(node)?;
                self.instructions.push("  mov (%rax), %rax".into())
            }
            NodeKind::Block {
                compound_statements,
            } => compound_statements
                .iter()
                .try_for_each(|statement| self.traverse(statement))?,
            NodeKind::If {
                condition,
                then_block,
                else_block,
            } => {
                let block = self.next_block_number();
                self.traverse(condition)?;
                self.instructions.push("  cmp $0, %rax".into());
                self.instructions
                    .push(format!("  je .L.else.{block}").into());
                self.traverse(then_block)?;
                self.instructions
                    .push(format!("  jmp .L.end.{block}").into());
                self.instructions.push(format!(".L.else.{block}:").into());
                if let Some(else_block) = else_block {
                    self.traverse(else_block)?;
                }
                self.instructions.push(format!(".L.end.{block}:").into());
            }
            NodeKind::Loop {
                init,
                condition,
                increment,
                loop_block,
            } => {
                let block = self.next_block_number();
                if let Some(init) = init {
                    self.traverse(init)?;
                }
                self.instructions.push(format!(".L.begin.{block}:").into());
                if let Some(condition) = condition {
                    self.traverse(condition)?;
                    self.instructions.push("  cmp $0, %rax".into());
                    self.instructions
                        .push(format!("  je .L.end.{block}").into());
                }
                self.traverse(loop_block)?;
                if let Some(increment) = increment {
                    self.traverse(increment)?;
                }
                self.instructions
                    .push(format!("  jmp .L.begin.{block}").into());
                self.instructions.push(format!(".L.end.{block}:").into());
            }
        }

        Ok(())
    }

    fn err_non_lvalue_assigment(&self, index: usize) -> CodeGenError<'a> {
        CodeGenError {
            kind: CodeGenErrorKind::NonLValueAssignment { index },
            input: self.input,
        }
    }
}
