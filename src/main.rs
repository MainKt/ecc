use ecc::{code_gen::CodeGen, lexer::Lexer, parser::Parser, util};

fn main() {
    let mut args = std::env::args();
    if args.len() != 2 {
        util::errx("invalid number of arguments");
    }

    let _exe_name = args.next().unwrap();
    let input = args.next().unwrap();

    let lexer = Lexer::new(&input);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(err) => util::errx(&format!("{err}")),
    };

    let parser = Parser::new(&input, tokens);
    let ast = match parser.parse() {
        Ok(node) => node,
        Err(err) => util::errx(&format!("{err}")),
    };

    println!("  .globl main");
    println!("main:");

    let code_gen = CodeGen::new();
    for instruction in code_gen.generate_assembly(&ast) {
        println!("{instruction }")
    }

    println!("  ret");
}
