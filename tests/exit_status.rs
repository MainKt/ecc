use std::{path::PathBuf, process::Command};

use tempfile::TempDir;

#[test]
fn assembly_exit_status() {
    assert_exit_status("{return 0;}", 0);
    assert_exit_status("{return 1;}", 1);
    assert_exit_status("{return 42;}", 42);
}

#[test]
fn basic_arithmetic() {
    assert_exit_status("{return 5+20-4;}", 21);
    assert_exit_status("{return 40-20-4;}", 16);
}

#[test]
fn multiple_variables() {
    assert_exit_status("{ int x = 3, y = 5; return x + y;}", 8);
    assert_exit_status("{ int x, y; int x = 2; int y = 3; return x + y;}", 5);
}

#[test]
fn address_of_and_deref() {
    assert_exit_status("{ int x=3; return *&x; }", 3);
    assert_exit_status("{ int x=3; int y=&x; int **z=&y; return **z; }", 3);
    assert_exit_status("{ int x=3; int *y=&x; *y=5; return x; }", 5);

    // NOTE: commented tests are from chibicc where stack locals is inverted
    // assert_exit_status("{ int x=3; int y=5; return *(&x+1); }", 5);
    assert_exit_status("{ int x=3; int y=5; return *(&x-1); }", 5);

    // assert_exit_status("{ int x=3; int y=5; return *(&y-1); }", 3);
    assert_exit_status("{ int x=3; int y=5; return *(&y+1); }", 3);

    // assert_exit_status("{ int x=3; int y=5; *(&x+1)=7; return y; }", 7);
    assert_exit_status("{ int x=3; int y=5; *(&x-1)=7; return y; }", 7);

    // assert_exit_status("{ int x=3; int y=5; *(&y-1)=7; return x; }", 7);
    assert_exit_status("{ int x=3; int y=5; *(&y+1)=7; return x; }", 7);

    assert_exit_status(r"{ int x = 3; int y = 5; return *(&y-(-1)); }", 3);

    assert_exit_status(r"{ int x; x = 3; return (&x+2)-&x+3; }", 5);
}

#[test]
fn basic_arithmetic_with_spaces() {
    assert_exit_status("{return 12 + 34 - 5;}", 41);
    assert_exit_status("{return 12+34 - 5;}", 41);
    assert_exit_status("{return  12 + 34 - 5 ;}", 41);
    assert_exit_status("{return 5+6*7;}", 47);
}

#[test]
fn parentheses() {
    assert_exit_status("{return 5*(9-6);}", 15);
    assert_exit_status("{return (3+5)/2;}", 4);
}

#[test]
fn unary() {
    assert_exit_status("{return -10+20;}", 10);
    assert_exit_status("{return - -10;}", 10);
    assert_exit_status("{return - - +10;}", 10);
}

#[test]
fn single_char_variables() {
    assert_exit_status("{int a; a=3; return a;}", 3);
    assert_exit_status("{int a=3; return a;}", 3);
    assert_exit_status("{int a=3; int z; z = 5; return a +z;}", 8);
    assert_exit_status("{int a; int b; a = b = 3; return a + b;}", 6);
}

#[test]
fn variables() {
    assert_exit_status("{int foo=3; return foo;}", 3);
    assert_exit_status("{int foo123=3; int bar=5; return foo123+bar;}", 3 + 5);
}

#[test]
fn return_statement() {
    assert_exit_status("{return 1; 2; 3;}", 1);
    assert_exit_status("{1; return 2; 3;}", 2);
    assert_exit_status("{1; 2; return 3;}", 3);
}

#[test]
fn nested_braces() {
    assert_exit_status("{ {1; {2;} return 3;} }", 3);
}

#[test]
fn null_blocks() {
    assert_exit_status("{ ;;; return 5;}", 5);
}

#[test]
fn comparison_operators() {
    assert_exit_status("{0==1;}", 0);
    assert_exit_status("{42==42;}", 1);
    assert_exit_status("{0!=1;}", 1);
    assert_exit_status("{42!=42;}", 0);
    assert_exit_status("{0<1;}", 1);
    assert_exit_status("{1<1;}", 0);
    assert_exit_status("{2<1;}", 0);
    assert_exit_status("{0<=1;}", 1);
    assert_exit_status("{1<=1;}", 1);
    assert_exit_status("{2<=1;}", 0);
    assert_exit_status("{1>0;}", 1);
    assert_exit_status("{1>1;}", 0);
    assert_exit_status("{1>2;}", 0);
    assert_exit_status("{1>=0;}", 1);
    assert_exit_status("{1>=1;}", 1);
    assert_exit_status("{1>=2;}", 0);
}

#[test]
fn if_statement() {
    assert_exit_status("{ if (0) return 2; return 3; }", 3);
    assert_exit_status("{ if (1-1) return 2; return 3; }", 3);
    assert_exit_status("{ if (1) return 2; return 3; }", 2);
    assert_exit_status("{ if (2-1) return 2; return 3; }", 2);
    assert_exit_status("{ if (0) { 1; 2; return 3; } else { return 4; } }", 4);
    assert_exit_status("{ if (1) { 1; 2; return 3; } else { return 4; } }", 3);
}

#[test]
fn for_statement() {
    assert_exit_status(
        r"{
        for (;;) {
            return 3;
        }
        return 5;
    }",
        3,
    );
    assert_exit_status(
        "{ int i = 0; int j = 0; for (i = 0; i <= 10; i = i + 1) j = i + j; return j; }",
        55,
    );
}

#[test]
fn while_loop() {
    assert_exit_status(
        r"{
        int i = 0;
        while (i < 10) {
            i = i + 1;
        }
        return i;
    }",
        10,
    );
}

#[test]
fn function_call() {
    assert_exit_status("{ return ret3(); }", 3);
    assert_exit_status("{ return ret5(); }", 5);
}

fn define_functions(cc: &str, dir: &TempDir) -> PathBuf {
    let fn_defs = dir.path().join("fn_defs.c");
    std::fs::write(
        &fn_defs,
        r"
            int ret3() { return 3; }
            int ret5() { return 5; }
        ",
    )
    .unwrap();
    let fn_defs_out_path = dir.path().join("fn_defs.o");
    let fn_defs_cmd = Command::new(&cc)
        .arg("-xc")
        .arg("-c")
        .arg("-o")
        .arg(&fn_defs_out_path)
        .arg(&fn_defs)
        .output()
        .expect("failed to assemble");
    assert!(fn_defs_cmd.status.success());

    fn_defs_out_path
}

fn assert_exit_status(program: &str, expected_status: i32) {
    let compiler = env!("CARGO_BIN_EXE_ecc");
    let asm_out = Command::new(compiler)
        .arg(program)
        .output()
        .expect("failed to run the compiler");
    assert!(
        asm_out.status.success(),
        "{}\n---------\n{}",
        program,
        String::from_utf8(asm_out.stderr).unwrap()
    );

    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let asm_path = dir.path().join("asm.S");
    std::fs::write(&asm_path, &asm_out.stdout).unwrap();
    let exe_path = dir.path().join("exe");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let fn_defs = define_functions(&cc, &dir);
    assert!(
        Command::new(&cc)
            .arg("-static")
            .arg("-o")
            .arg(&exe_path)
            .arg(&asm_path)
            .arg(&fn_defs)
            .status()
            .expect("failed to assemble")
            .success()
    );

    let assemble = Command::new(exe_path).output().expect("failed to assemble");
    assert_eq!(
        assemble.status.code(),
        Some(expected_status),
        "\nprogram: {program}\nassembly:\n---------\n{}\n---------",
        String::from_utf8(asm_out.stdout).unwrap(),
    );
}
