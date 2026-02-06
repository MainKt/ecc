use std::{
    fs,
    io::{self, Read},
};

use ecc::{code_gen::CodeGen, lexer::Lexer, parser::Parser, util};

fn main() {
    let mut args = std::env::args();
    if args.len() != 2 {
        util::errx("invalid number of arguments");
    }

    let _exe_name = args.next().unwrap();
    let file_name = args.next().unwrap();

    let input = if file_name == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).map(|_| buffer)
    } else {
        fs::read_to_string(&file_name)
    };
    let input = match input {
        Ok(input) => input,
        Err(err) => util::errx(&format!("failed to read file, {file_name}: {err}")),
    };

    let lexer = Lexer::new(&file_name, &input);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(err) => util::errx(&format!("{err}")),
    };

    let parser = Parser::new(&file_name, &input, &tokens);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(err) => util::errx(&format!("{err}")),
    };

    let code_gen = CodeGen::new(&file_name, &input);
    let instructions = match code_gen.generate_assembly(ast) {
        Ok(instructions) => instructions,
        Err(err) => util::errx(&format!("{err}")),
    };
    for instruction in instructions {
        println!("{instruction}")
    }
}
