use crate::parser::Node;

pub struct CodeGen {
    depth: usize,
    instructions: Vec<String>,
}

impl CodeGen {
    pub fn new() -> Self {
        Self {
            depth: 0,
            instructions: vec![],
        }
    }

    pub fn generate_assembly(mut self, node: &Node) -> Vec<String> {
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
            Node::Add(lhs, rhs) => {
                self.traverse_children(rhs, lhs);
                self.instructions.push("  add %rdi, %rax".into());
            }
            Node::Subtract(lhs, rhs) => {
                self.traverse_children(rhs, lhs);
                self.instructions.push("  sub %rdi, %rax".into());
            }
            Node::Multiply(lhs, rhs) => {
                self.traverse_children(rhs, lhs);
                self.instructions.push("  imul %rdi, %rax".into());
            }
            Node::Divide(lhs, rhs) => {
                self.traverse_children(rhs, lhs);
                self.instructions.push("  cqo".into());
                self.instructions.push("  idiv %rdi".into());
            }
            Node::Numeric(num) => self.instructions.push(format!("  mov ${num}, %rax")),
            Node::Negative(lhs) => {
                self.traverse(lhs);
                self.instructions.push("  neg %rax".into());
            }
            Node::Equal(lhs, rhs) => {
                self.traverse_children(rhs, lhs);
                self.instructions.push("  cmp %rdi, %rax".into());
                self.instructions.push("  sete %al".into());
                self.instructions.push("  movzb %al, %rax".into());
            }
            Node::NotEqual(lhs, rhs) => {
                self.traverse_children(rhs, lhs);
                self.instructions.push("  cmp %rdi, %rax".into());
                self.instructions.push("  setne %al".into());
                self.instructions.push("  movzb %al, %rax".into());
            }
            Node::LessThan(lhs, rhs) => {
                self.traverse_children(rhs, lhs);
                self.instructions.push("  cmp %rdi, %rax".into());
                self.instructions.push("  setl %al".into());
                self.instructions.push("  movzb %al, %rax".into());
            }
            Node::LessThanEqual(lhs, rhs) => {
                self.traverse_children(rhs, lhs);
                self.instructions.push("  cmp %rdi, %rax".into());
                self.instructions.push("  setle %al".into());
                self.instructions.push("  movzb %al, %rax".into());
            }
        }
    }
}
