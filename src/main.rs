use ecc::{
    lexer::{Kind, Lexer, Token},
    util,
};

fn main() {
    let mut args = std::env::args();

    if args.len() != 2 {
        util::errx("invalid number of arguments");
    }

    let _exe_name = args.next().unwrap();
    let input = args.next().unwrap();

    let lexer = Lexer::new(&input);
    let mut tokens = lexer.into_iter();

    println!("  .globl main");
    println!("main:");

    let Some(Token {
        kind: Kind::Numeric(initial),
        ..
    }) = tokens.next()
    else {
        util::err_at("expected a number", &input, 0);
    };
    println!("  mov ${initial}, %rax");

    while let Some(Token {
        kind,
        index,
        length,
    }) = tokens.next()
    {
        match kind {
            Kind::Punctuation(op @ ('+' | '-')) => {
                let Some(Token {
                    kind: Kind::Numeric(operand),
                    ..
                }) = tokens.next()
                else {
                    util::err_at("expected a number", &input, index + length);
                };

                println!(
                    "  {} ${operand}, %rax",
                    if op == '+' { "add" } else { "sub" }
                );
            }
            _ => util::err_at("got an unimplemented token", &input, index),
        }
    }

    println!("  ret");
}
