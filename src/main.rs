fn main() {
    let mut args = std::env::args();
    let program_name = args.next().unwrap();

    if args.len() != 1 {
        eprintln!("{program_name}: invalid number of arguments");
        std::process::exit(1);
    }

    println!("  .globl main");
    println!("main:");

    let program = args.next().unwrap();
    let (operand, program) = program.split_at(
        program
            .find(|c: char| !c.is_numeric())
            .unwrap_or(program.len()),
    );
    let Ok(operand) = operand.parse::<i32>() else {
        eprintln!("{program_name}: expected integer operands");
        std::process::exit(1);
    };
    println!("  mov ${operand}, %rax");

    let mut program = program.chars().peekable();
    while let Some(c) = program.next() {
        let mut operand = String::new();
        while let Some(&n) = program.peek()
            && n.is_numeric()
        {
            operand.push(n);
            program.next();
        }

        match c {
            '+' => println!("  add ${operand}, %rax"),
            '-' => println!("  sub ${operand}, %rax"),
            c => {
                eprintln!("{program_name}: unexpected character: `{c}'");
                std::process::exit(1);
            }
        }
    }

    println!("  ret");
}
