use ecc::{
    tokenize::{Kind, tokenize},
    util,
};

fn main() {
    let mut args = std::env::args();

    if args.len() != 2 {
        util::errx("invalid number of arguments");
    }

    let _exe_name = args.next().unwrap();
    let program = args.next().unwrap();
    let mut tokens = tokenize(&program).into_iter();

    println!("  .globl main");
    println!("main:");

    let mut instructions: Vec<String> = Vec::new();
    while let Some(token) = tokens.next() {
        if instructions.is_empty() {
            let Kind::Numeric(num) = token.kind else {
                util::errx("expected a number");
            };
            instructions.push(format!("  mov ${num}, %rax"));
            continue;
        }

        match token.kind {
            Kind::Punctuation(op @ ('+' | '-')) => {
                let Some(token) = tokens.next() else {
                    util::errx("expected a number");
                };
                let Kind::Numeric(num) = token.kind else {
                    util::errx("expected a number");
                };
                instructions.push(format!(
                    "  {} ${num}, %rax",
                    if op == '+' { "add " } else { "sub" }
                ));
            }
            Kind::EOF => break,
            _ => util::errx("got an unexpected token"),
        }
    }

    for instruction in instructions {
        println!("{instruction }");
    }
    println!("  ret");
}
