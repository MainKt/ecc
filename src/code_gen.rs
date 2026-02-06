use crate::{
    parser::{
        BinaryKind, Function, Lifetime, Node, NodeKind, Object, ObjectKind, UnaryKind,
        types::{DerivedKind, Type, TypeKind},
    },
    util,
};
use std::{borrow::Cow, rc::Rc};

#[derive(Debug)]
pub enum CodeGenErrorKind {
    NonLValueAssignment { index: usize },
}

#[derive(Debug)]
pub struct CodeGenError<'a> {
    file: &'a str,
    input: &'a str,
    kind: CodeGenErrorKind,
}

impl<'a> std::fmt::Display for CodeGenError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            CodeGenErrorKind::NonLValueAssignment { index } => {
                util::info_msg(f, self.file, self.input, index)?;
                write!(f, "not an lvalue")
            }
        }
    }
}

pub struct CodeGen<'a> {
    file: &'a str,
    input: &'a str,
    depth: usize,
    instructions: Vec<Cow<'a, str>>,
    block_count: usize,
    current_function: &'a str,
}

static ARG_REGISTERS_8: [&'static str; 6] = ["%dil", "%sil", "%dl", "%cl", "%r8b", "%r9b"];
static ARG_REGISTERS_64: [&'static str; 6] = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];

impl<'a> CodeGen<'a> {
    pub fn new(file: &'a str, input: &'a str) -> Self {
        Self {
            file,
            input,
            depth: 0,
            instructions: vec![],
            block_count: 0,
            current_function: "",
        }
    }

    fn next_block_number(&mut self) -> usize {
        self.block_count += 1;
        self.block_count
    }

    fn emit_data(&mut self, object: &Rc<Object<'_>>, initial_data: &[u8]) {
        self.instructions.push("  .data".into());
        self.instructions
            .push(format!("  .globl {}", object.name).into());
        self.instructions.push(format!("{}:", object.name).into());
        if initial_data.is_empty() {
            self.instructions
                .push(format!("  .zero {}", object.object_type.size).into());
        } else {
            self.instructions
                .extend(initial_data.iter().map(|d| format!("  .byte {d}").into()));
        }
    }

    fn emit_text(&mut self, function: &Function<'a>) -> Result<(), CodeGenError<'a>> {
        self.current_function = function.name();

        self.instructions
            .push(format!("  .globl {}", function.name()).into());
        self.instructions.push("  .text".into());
        self.instructions
            .push(format!("{}:", function.name()).into());
        self.instructions.push("  push %rbp".into());
        self.instructions.push("  mov %rsp, %rbp".into());
        self.instructions
            .push(format!("  sub ${}, %rsp", function.stack_size()).into());

        for (i, param) in function.params().iter().enumerate() {
            if i > function.params().len() {
                break;
            }
            let register = (if param.object_type.size == 1 {
                ARG_REGISTERS_8
            } else {
                ARG_REGISTERS_64
            })[i];

            self.instructions
                .push(format!("  mov {register}, {}(%rbp)", param.local_offset()).into());
        }
        // NOTE: maybe make NodeKind::Function{..} ?
        function.body().iter().try_for_each(|n| self.traverse(n))?;

        self.instructions
            .push(format!(".L.return.{}:", function.name()).into());
        self.instructions.push("  mov %rbp, %rsp".into());
        self.instructions.push("  pop %rbp".into());
        self.instructions.push("  ret".into());

        Ok(())
    }

    pub fn generate_assembly(
        mut self,
        object: Object<'a>,
    ) -> Result<Vec<Cow<'a, str>>, CodeGenError<'a>> {
        let ObjectKind::TranslationUnit(translation_unit) = object.kind else {
            return Ok(vec![]);
        };

        for object in translation_unit.objects.values() {
            match &object.kind {
                ObjectKind::Function { function, .. } => self.emit_text(function)?,
                ObjectKind::Variable { initial_data } => self.emit_data(object, &initial_data),
                _ => break,
            }
        }

        Ok(self.instructions)
    }

    fn push(&mut self) {
        self.instructions.push("  push %rax".into());
        self.depth += 1;
    }

    fn pop(&mut self, register: &str) {
        self.instructions.push(format!("  pop {register}").into());
        self.depth -= 1;
    }

    fn generate_address(&mut self, Node { kind, info, .. }: &Node) -> Result<(), CodeGenError<'a>> {
        match kind {
            NodeKind::Variable(object) => {
                self.instructions.push(
                    match object.lifetime {
                        Lifetime::Global => format!("  lea {}(%rip), %rax", object.name),
                        Lifetime::Local { offset } => format!("  lea {offset}(%rbp), %rax"),
                    }
                    .into(),
                );

                Ok(())
            }
            NodeKind::Unary {
                kind: UnaryKind::Deref,
                lhs,
                ..
            } => self.traverse(lhs),
            NodeKind::Binary {
                kind: BinaryKind::Comma,
                rhs,
                lhs,
                ..
            } => {
                self.traverse(lhs)?;
                self.generate_address(rhs)
            }
            _ => Err(self.err_non_lvalue_assigment(info.index)),
        }
    }

    fn traverse_children(&mut self, rhs: &Node, lhs: &Node) -> Result<(), CodeGenError<'a>> {
        self.traverse(rhs)?;
        self.push();
        self.traverse(lhs)?;
        self.pop("%rdi");

        Ok(())
    }

    fn load(&mut self, load_type: &Rc<Type>) {
        if let TypeKind::Derived {
            kind: DerivedKind::Array { .. },
            ..
        } = load_type.kind
        {
            return;
        }

        self.instructions.push(
            format!(
                "  {} (%rax), %rax",
                if load_type.size == 1 { "movsbq" } else { "mov" }
            )
            .into(),
        )
    }

    fn store(&mut self, store_type: &Rc<Type>) {
        self.pop("%rdi");
        self.instructions.push(
            format!(
                "  mov %{}, (%rdi)",
                if store_type.size == 1 { "al" } else { "rax" }
            )
            .into(),
        )
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
                    self.store(&node.node_type)
                }
                BinaryKind::Comma => {
                    self.traverse(lhs)?;
                    self.traverse(rhs)?;
                }
            },
            NodeKind::Unary { kind, lhs, .. } => match kind {
                UnaryKind::Negate => {
                    self.traverse(lhs)?;
                    self.instructions.push("  neg %rax".into())
                }
                UnaryKind::Return => {
                    self.traverse(lhs)?;
                    self.instructions
                        .push(format!("  jmp .L.return.{}", self.current_function).into())
                }
                UnaryKind::Address => self.generate_address(lhs)?,
                UnaryKind::Deref => {
                    self.traverse(lhs)?;
                    self.load(&node.node_type);
                }
                UnaryKind::ExprStatement => self.traverse(lhs)?,
            },
            NodeKind::Numeric(value) => self
                .instructions
                .push(format!("  mov ${value}, %rax").into()),
            NodeKind::Variable(..) => {
                self.generate_address(node)?;
                self.load(&node.node_type);
            }
            NodeKind::CompoundStatement { nodes, .. } => {
                nodes.iter().try_for_each(|node| self.traverse(node))?
            }
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
            NodeKind::FunctionCall { name, args } => {
                for arg in args {
                    self.traverse(arg)?;
                    self.push();
                }
                for register in ARG_REGISTERS_64.iter().take(args.len()).rev() {
                    self.pop(register);
                }
                self.instructions.push("  mov $0, %rax".into());
                self.instructions.push(format!("  call {name}").into());
            }
        }

        Ok(())
    }

    fn err_non_lvalue_assigment(&self, index: usize) -> CodeGenError<'a> {
        CodeGenError {
            file: self.file,
            kind: CodeGenErrorKind::NonLValueAssignment { index },
            input: self.input,
        }
    }

    fn _dbg(&mut self, msg: &str) {
        self.instructions.push(format!("<dbg>###{msg}###").into());
    }
}
