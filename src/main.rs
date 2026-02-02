use ecc::{lexer::Lexer, parser::Parser, util};

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
    let node = match parser.parse() {
        Ok(node) => node,
        Err(err) => util::errx(&format!("{err}")),
    };

    println!("  .globl main");
    println!("main:");

    dbg!(node);

    println!("  ret");
}
