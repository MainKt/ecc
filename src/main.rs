fn main() {
    let mut args = std::env::args();
    let program_name = args.next().unwrap();

    if args.len() != 1 {
        eprintln!("{program_name}: invalid number of arguments");
        std::process::exit(1);
    }

    let Ok(exit_status) = args.next().unwrap().parse::<i32>() else {
        eprintln!("{program_name}: expected integer status code argument");
        std::process::exit(1);
    };

    println!("  .globl main");
    println!("main:");
    println!("  mov ${exit_status}, %rax");
    println!("  ret");
}
