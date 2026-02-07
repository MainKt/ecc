use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
};

use ecc::{
    code_gen::CodeGen,
    get_opt::{ArgRef, ArgType, GetOpt},
    lexer::Lexer,
    parser::Parser,
    util,
};

fn main() {
    let long_opts = [("help", ArgType::None, None)].map(From::from);
    let mut get_opt = GetOpt::new(std::env::args(), "o:", &long_opts, false);
    let mut out_file: Option<String> = None;
    for opt in get_opt.iter() {
        let opt = match opt {
            Ok(opt) => opt,
            Err(err) => {
                util::usage(std::io::stderr());
                util::errx(&format!("{err}"))
            }
        };

        match opt.as_ref() {
            ArgRef::Flag('o', Some(file)) => out_file = Some(file.into()),
            ArgRef::Name("help", None) => {
                util::usage(std::io::stdout());
                std::process::exit(0);
            }
            _ => unreachable!(),
        }
    }
    let mut rest = get_opt.rest();
    rest.next(); // skip executable name
    let Some(file_name) = rest.next() else {
        util::errx("no input files");
    };

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

    let mut out: Box<dyn Write> = match out_file.as_deref() {
        Some("-") | None => Box::new(io::stdout()),
        Some(out_file) => {
            let out_file = match OpenOptions::new().write(true).create(true).open(out_file) {
                Ok(out_file) => out_file,
                Err(err) => util::errx(&format!("{err}")),
            };
            Box::new(out_file)
        }
    };
    if let Err(err) = instructions
        .iter()
        .try_for_each(|instruction| writeln!(out, "{instruction}"))
    {
        util::errx(&format!("{err}"))
    };
}
