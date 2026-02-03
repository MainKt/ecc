use std::process::Command;

#[test]
fn test_assembly_exit_status() {
    assert_exit_status("0;", 0);
    assert_exit_status("1;", 1);
    assert_exit_status("42;", 42);
}

#[test]
fn test_basic_arithmetic() {
    assert_exit_status("5+20-4;", 21);
    assert_exit_status("40-20-4;", 16);
}

#[test]
fn test_basic_arithmetic_with_spaces() {
    assert_exit_status("12 + 34 - 5;", 41);
    assert_exit_status("12+34 - 5;", 41);
    assert_exit_status(" 12 + 34 - 5 ;", 41);
    assert_exit_status("5+6*7;", 47);
}

#[test]
fn test_parentheses() {
    assert_exit_status("5*(9-6);", 15);
    assert_exit_status("(3+5)/2;", 4);
}

#[test]
fn test_unary() {
    assert_exit_status("-10+20;", 10);
    assert_exit_status("- -10;", 10);
    assert_exit_status("- - +10;", 10);
}

#[test]
fn test_variables() {
    assert_exit_status("a=3; a;", 3);
    assert_exit_status("a=3; z = 5; a +z;", 8);
    assert_exit_status("a = b = 3; a + b;", 6);
}

#[test]
fn test_comparison_operators() {
    assert_exit_status("0==1;", 0);
    assert_exit_status("42==42;", 1);
    assert_exit_status("0!=1;", 1);
    assert_exit_status("42!=42;", 0);
    assert_exit_status("0<1;", 1);
    assert_exit_status("1<1;", 0);
    assert_exit_status("2<1;", 0);
    assert_exit_status("0<=1;", 1);
    assert_exit_status("1<=1;", 1);
    assert_exit_status("2<=1;", 0);
    assert_exit_status("1>0;", 1);
    assert_exit_status("1>1;", 0);
    assert_exit_status("1>2;", 0);
    assert_exit_status("1>=0;", 1);
    assert_exit_status("1>=1;", 1);
    assert_exit_status("1>=2;", 0);
}

fn assert_exit_status(program: &str, expected_status: i32) {
    let compiler = env!("CARGO_BIN_EXE_ecc");
    let asm_out = Command::new(compiler)
        .arg(program)
        .output()
        .expect("failed to run the compiler");
    assert!(asm_out.status.success(), "{}", program);

    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let asm_path = dir.path().join("asm.S");
    std::fs::write(&asm_path, &asm_out.stdout).unwrap();
    let exe_path = dir.path().join("exe");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    assert!(
        Command::new(cc)
            .arg("-static")
            .arg("-o")
            .arg(&exe_path)
            .arg(&asm_path)
            .status()
            .expect("failed to assemble")
            .success()
    );

    assert_eq!(
        Command::new(exe_path)
            .status()
            .expect("failed to assemble")
            .code(),
        Some(expected_status)
    );
}
