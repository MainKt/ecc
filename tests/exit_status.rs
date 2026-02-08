use std::{path::PathBuf, process::Command};

use tempfile::TempDir;

#[test]
fn structs() {
    assert_exit_status(
        "int main() { struct {int a; int b;} x; x.a=1; x.b=2; return x.a; }",
        1,
    );
    assert_exit_status(
        "int main() { struct {int a; int b;} x; x.a=1; x.b=2; return x.b; }",
        2,
    );
    assert_exit_status(
        "int main() { struct {char a; int b; char c;} x; x.a=1; x.b=2; x.c=3; return x.a; }",
        1,
    );
    assert_exit_status(
        "int main() { struct {char a; int b; char c;} x; x.b=1; x.b=2; x.c=3; return x.b; }",
        2,
    );
    assert_exit_status(
        "int main() { struct {char a; int b; char c;} x; x.a=1; x.b=2; x.c=3; return x.c; }",
        3,
    );
    assert_exit_status(
        "int main() { struct { struct { char b; } a; } x; x.a.b=6; return x.a.b; }",
        6,
    );
    assert_exit_status("int main() { struct {int a;} x; return sizeof(x); }", 8);
    assert_exit_status(
        "int main() { struct {int a; int b;} x; return sizeof(x); }",
        16,
    );
    assert_exit_status("int main() { struct {int a, b;} x; return sizeof(x); }", 16);
    assert_exit_status("int main() { struct {int a[3];} x; return sizeof(x); }", 24);
    assert_exit_status("int main() { struct {int a;} x[4]; return sizeof(x); }", 32);
    assert_exit_status(
        "int main() { struct {int a[3];} x[2]; return sizeof(x); }",
        48,
    );
    assert_exit_status(
        "int main() { struct {char a; char b;} x; return sizeof(x); }",
        2,
    );
    assert_exit_status(
        "int main() { struct {char a; int b;} x; return sizeof(x); }",
        9,
    );
    assert_exit_status("int main() { struct {} x; return sizeof(x); }", 0);
}

#[test]
fn pointer_to_struct() {
    assert_exit_status(
        "int main() { struct {char a; char b;} x[3]; char *p=x; p[1]=19; return x[0].b; }",
        19,
    );
    // assert_exit_status(
    //     "int main() { struct {char a; char b;} x[3]; char *p=x; p[0]=0; return x[0].a; }",
    //     0,
    // );
    // assert_exit_status(
    //     "int main() { struct {char a; char b;} x[3]; char *p=x; p[1]=1; return x[0].b; }",
    //     1,
    // );
    // assert_exit_status(
    //     "int main() { struct {char a; char b;} x[3]; char *p=x; p[2]=2; return x[1].a; }",
    //     2,
    // );
    // assert_exit_status(
    //     "int main() { struct {char a; char b;} x[3]; char *p=x; p[3]=3; return x[1].b; }",
    //     3,
    // );
    // assert_exit_status(
    //     "int main() { struct {char a[3]; char b[5];} x; char *p=&x; x.a[0]=6; return p[0]; }",
    //     6,
    // );
    // assert_exit_status(
    //     "int main() { struct {char a[3]; char b[5];} x; char *p=&x; x.b[0]=7; return p[3]; }",
    //     7,
    // );
}

#[test]
fn block_scope() {
    assert_exit_status("int main() { int x=2; { int x=3; } return x; }", 2);
    assert_exit_status(
        "int main() { int x=2; { int x=3; } { int y=4; return x; }}",
        2,
    );
    assert_exit_status("int main() { int x=2; { x=3; } return x; }", 3);
}

#[test]
fn comma_operator() {
    assert_exit_status("int main() { return (1,2,3); }", 3);
    assert_exit_status("int main() { int i=2, j=3; (i=5,j)=6; return i; }", 5);
    assert_exit_status("int main() { int i=2, j=3; (i=5,j)=6; return j; }", 6);
}

#[test]
fn assembly_exit_status() {
    assert_exit_status("int main() {return 0;}", 0);
    assert_exit_status("int main() {return 1;}", 1);
    assert_exit_status("int main() {return 42;}", 42);
}

#[test]
fn octal_sequences() {
    assert_exit_status(r#"int main() { return "\0"[0]; }"#, 0);
    assert_exit_status(r#"int main() { return "\20"[0]; }"#, 16);
    assert_exit_status(r#"int main() { return "\101"[0]; }"#, 65);
    assert_exit_status(r#"int main() { return "\1500"[0]; }"#, 104);
}

#[test]
fn hex_sequence() {
    assert_exit_status(r#"int main() { return "\x00"[0]; }"#, 0);
    assert_exit_status(r#"int main() { return "\x77"[0]; }"#, 119);
    assert_exit_status(r#"int main() { return "\xA5"[0]; }"#, 165);
    assert_exit_status(r#"int main() { return "\x00ff"[0]; }"#, 255);
}

#[test]
fn comments() {
    assert_exit_status(
        r"
        int main() { // return 1;
            return 2;
        }",
        2,
    );
    assert_exit_status("int main() { /* return 1; */ return 2; }", 2);
}

#[test]
fn statement_expression() {
    assert_exit_status("int main() { return ({ 0; }); }", 0);
    assert_exit_status("int main() { return ({ 0; 1; 2; }); }", 2);
    assert_exit_status("int main() { ({ 0; return 1; 2; }); return 3; }", 1);
    assert_exit_status("int main() { return ({ 1; }) + ({ 2; }) + ({ 3; }); }", 6);
    assert_exit_status("int main() { return ({ int x=3; x; }); }", 3);
}

#[test]
fn escape_sequences() {
    assert_exit_status(r#"int main() { return "\a"[0]; }"#, 7);
    assert_exit_status(r#"int main() { return "\b"[0]; }"#, 8);
    assert_exit_status(r#"int main() { return "\t"[0]; }"#, 9);
    assert_exit_status(r#"int main() { return "\n"[0]; }"#, 10);
    assert_exit_status(r#"int main() { return "\v"[0]; }"#, 11);
    assert_exit_status(r#"int main() { return "\f"[0]; }"#, 12);
    assert_exit_status(r#"int main() { return "\r"[0]; }"#, 13);
    assert_exit_status(r#"int main() { return "\e"[0]; }"#, 27);
    assert_exit_status(r#"int main() { return "\j"[0]; }"#, 106);
    assert_exit_status(r#"int main() { return "\k"[0]; }"#, 107);
    assert_exit_status(r#"int main() { return "\l"[0]; }"#, 108);
    assert_exit_status(r#"int main() { return "\ax\ny"[0]; }"#, 7);
    assert_exit_status(r#"int main() { return "\ax\ny"[1]; }"#, 120);
    assert_exit_status(r#"int main() { return "\ax\ny"[2]; }"#, 10);
    assert_exit_status(r#"int main() { return "\ax\ny"[3]; }"#, 121);
}

#[test]
fn global_variables() {
    assert_exit_status("int x; int main() { return x; }", 0);
    assert_exit_status("int x; int main() { x=3; return x; }", 3);
    assert_exit_status("int x; int y; int main() { x=3; y=4; return x+y; }", 7);
    assert_exit_status("int x, y; int main() { x=3; y=4; return x+y; }", 7);
    assert_exit_status(
        "int x[4]; int main() { x[0]=0; x[1]=1; x[2]=2; x[3]=3; return x[0]; }",
        0,
    );
    assert_exit_status(
        "int x[4]; int main() { x[0]=0; x[1]=1; x[2]=2; x[3]=3; return x[1]; }",
        1,
    );
    assert_exit_status(
        "int x[4]; int main() { x[0]=0; x[1]=1; x[2]=2; x[3]=3; return x[2]; }",
        2,
    );
    assert_exit_status(
        "int x[4]; int main() { x[0]=0; x[1]=1; x[2]=2; x[3]=3; return x[3]; }",
        3,
    );
    assert_exit_status("int x; int main() { return sizeof(x); }", 8);
    assert_exit_status("int x[4]; int main() { return sizeof(x); }", 32);
}

#[test]
fn sizeof() {
    assert_exit_status("int main() { int x; return sizeof(x); }", 8);
    assert_exit_status("int main() { int x; return sizeof x; }", 8);
    assert_exit_status("int main() { int *x; return sizeof(x); }", 8);
    assert_exit_status("int main() { int x[4]; return sizeof(x); }", 32);
    assert_exit_status("int main() { int x[3][4]; return sizeof(x); }", 96);
    assert_exit_status("int main() { int x[3][4]; return sizeof(*x); }", 32);
    assert_exit_status("int main() { int x[3][4]; return sizeof(**x); }", 8);
    assert_exit_status("int main() { int x[3][4]; return sizeof(**x) + 1; }", 9);
    assert_exit_status("int main() { int x[3][4]; return sizeof **x + 1; }", 9);
    assert_exit_status("int main() { int x[3][4]; return sizeof(**x + 1); }", 8);
    assert_exit_status("int main() { int x=1; return sizeof(x=2); }", 8);
    assert_exit_status("int main() { int x=1; sizeof(x=2); return x; }", 1);
}

#[test]
fn array_indexing() {
    assert_exit_status(
        "int main() { int x[3]; *x=3; x[1]=4; x[2]=5; return *x; }",
        3,
    );
    assert_exit_status(
        "int main() { int x[3]; *x=3; x[1]=4; x[2]=5; return *(x+1); }",
        4,
    );
    assert_exit_status(
        "int main() { int x[3]; *x=3; x[1]=4; x[2]=5; return *(x+2); }",
        5,
    );
    assert_exit_status(
        "int main() { int x[3]; *x=3; x[1]=4; x[2]=5; return *(x+2); }",
        5,
    );
    assert_exit_status(
        "int main() { int x[3]; *x=3; x[1]=4; 2[x]=5; return *(x+2); }",
        5,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; y[0]=0; return x[0][0]; }",
        0,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; y[1]=1; return x[0][1]; }",
        1,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; y[2]=2; return x[0][2]; }",
        2,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; y[3]=3; return x[1][0]; }",
        3,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; y[4]=4; return x[1][1]; }",
        4,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; y[5]=5; return x[1][2]; }",
        5,
    );
}

#[test]
fn multi_param_functions() {
    assert_exit_status(
        "int main() { return add2(3,4); } int add2(int x, int y) { return x+y; }",
        7,
    );
    assert_exit_status(
        "int main() { return sub2(4,3); } int sub2(int x, int y) { return x-y; }",
        1,
    );
    assert_exit_status(
        "int main() { return fib(9); } int fib(int x) { if (x<=1) return 1; return fib(x-1) + fib(x-2); }",
        55,
    );
}

#[test]
fn array() {
    assert_exit_status("int main() { int x[2]; int *y=&x; *y=3; return *x; }", 3);
    assert_exit_status(
        "int main() { int x[3]; *x=3; *(x+1)=4; *(x+2)=5; return *x; }",
        3,
    );
    assert_exit_status(
        "int main() { int x[3]; *x=3; *(x+1)=4; *(x+2)=5; return *(x+1); }",
        4,
    );
    assert_exit_status(
        "int main() { int x[3]; *x=3; *(x+1)=4; *(x+2)=5; return *(x+2); }",
        5,
    );
}

#[test]
fn array_2d() {
    assert_exit_status("int main() { int x[2][3]; int *y=x; *y=0; return **x; }", 0);
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; *(y+1)=1; return *(*x+1); }",
        1,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; *(y+2)=2; return *(*x+2); }",
        2,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; *(y+3)=3; return **(x+1); }",
        3,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; *(y+4)=4; return *(*(x+1)+1); }",
        4,
    );
    assert_exit_status(
        "int main() { int x[2][3]; int *y=x; *(y+5)=5; return *(*(x+1)+2); }",
        5,
    );
}

#[test]
fn basic_arithmetic() {
    assert_exit_status("int main() {return 5+20-4;}", 21);
    assert_exit_status("int main() {return 40-20-4;}", 16);
}

#[test]
fn multiple_variables() {
    assert_exit_status("int main() { int x = 3, y = 5; return x + y;}", 8);
    assert_exit_status(
        "int main() { int x, y; int x = 2; int y = 3; return x + y;}",
        5,
    );
}

#[test]
fn address_of_and_deref() {
    assert_exit_status("int main() { int x=3; return *&x; }", 3);
    assert_exit_status(
        "int main() { int x=3; int y=&x; int **z=&y; return **z; }",
        3,
    );
    assert_exit_status("int main() { int x=3; int *y=&x; *y=5; return x; }", 5);

    // NOTE: commented tests are from chibicc where stack locals is inverted
    // assert_exit_status("int main() { int x=3; int y=5; return *(&x+1); }", 5);
    assert_exit_status("int main() { int x=3; int y=5; return *(&x-1); }", 5);

    // assert_exit_status("int main() { int x=3; int y=5; return *(&y-1); }", 3);
    assert_exit_status("int main() { int x=3; int y=5; return *(&y+1); }", 3);

    // assert_exit_status("int main() { int x=3; int y=5; *(&x+1)=7; return y; }", 7);
    assert_exit_status("int main() { int x=3; int y=5; *(&x-1)=7; return y; }", 7);

    // assert_exit_status("int main() { int x=3; int y=5; *(&y-1)=7; return x; }", 7);
    assert_exit_status("int main() { int x=3; int y=5; *(&y+1)=7; return x; }", 7);

    assert_exit_status(
        r"int main() { int x = 3; int y = 5; return *(&y-(-1)); }",
        3,
    );

    assert_exit_status(r"int main() { int x; x = 3; return (&x+2)-&x+3; }", 5);
}

#[test]
fn basic_arithmetic_with_spaces() {
    assert_exit_status("int main() {return 12 + 34 - 5;}", 41);
    assert_exit_status("int main() {return 12+34 - 5;}", 41);
    assert_exit_status("int main() {return  12 + 34 - 5 ;}", 41);
    assert_exit_status("int main() {return 5+6*7;}", 47);
}

#[test]
fn parentheses() {
    assert_exit_status("int main() {return 5*(9-6);}", 15);
    assert_exit_status("int main() {return (3+5)/2;}", 4);
}

#[test]
fn unary() {
    assert_exit_status("int main() {return -10+20;}", 10);
    assert_exit_status("int main() {return - -10;}", 10);
    assert_exit_status("int main() {return - - +10;}", 10);
}

#[test]
fn single_char_variables() {
    assert_exit_status("int main() {int a; a=3; return a;}", 3);
    assert_exit_status("int main() {int a=3; return a;}", 3);
    assert_exit_status("int main() {int a=3; int z; z = 5; return a +z;}", 8);
    assert_exit_status("int main() {int a; int b; a = b = 3; return a + b;}", 6);
}

#[test]
fn variables() {
    assert_exit_status("int main() {int foo=3; return foo;}", 3);
    assert_exit_status(
        "int main() {int foo123=3; int bar=5; return foo123+bar;}",
        3 + 5,
    );
}

#[test]
fn return_statement() {
    assert_exit_status("int main() {return 1; 2; 3;}", 1);
    assert_exit_status("int main() {1; return 2; 3;}", 2);
    assert_exit_status("int main() {1; 2; return 3;}", 3);
}

#[test]
fn nested_braces() {
    assert_exit_status("int main() { {1; {2;} return 3;} }", 3);
}

#[test]
fn null_blocks() {
    assert_exit_status("int main() { ;;; return 5;}", 5);
}

#[test]
fn comparison_operators() {
    assert_exit_status("int main() {0==1;}", 0);
    assert_exit_status("int main() {42==42;}", 1);
    assert_exit_status("int main() {0!=1;}", 1);
    assert_exit_status("int main() {42!=42;}", 0);
    assert_exit_status("int main() {0<1;}", 1);
    assert_exit_status("int main() {1<1;}", 0);
    assert_exit_status("int main() {2<1;}", 0);
    assert_exit_status("int main() {0<=1;}", 1);
    assert_exit_status("int main() {1<=1;}", 1);
    assert_exit_status("int main() {2<=1;}", 0);
    assert_exit_status("int main() {1>0;}", 1);
    assert_exit_status("int main() {1>1;}", 0);
    assert_exit_status("int main() {1>2;}", 0);
    assert_exit_status("int main() {1>=0;}", 1);
    assert_exit_status("int main() {1>=1;}", 1);
    assert_exit_status("int main() {1>=2;}", 0);
}

#[test]
fn if_statement() {
    assert_exit_status("int main() { if (0) return 2; return 3; }", 3);
    assert_exit_status("int main() { if (1-1) return 2; return 3; }", 3);
    assert_exit_status("int main() { if (1) return 2; return 3; }", 2);
    assert_exit_status("int main() { if (2-1) return 2; return 3; }", 2);
    assert_exit_status(
        "int main() { if (0) { 1; 2; return 3; } else { return 4; } }",
        4,
    );
    assert_exit_status(
        "int main() { if (1) { 1; 2; return 3; } else { return 4; } }",
        3,
    );
}

#[test]
fn for_statement() {
    assert_exit_status(
        r"int main() {
        for (;;) {
            return 3;
        }
        return 5;
    }",
        3,
    );
    assert_exit_status(
        "int main() { int i = 0; int j = 0; for (i = 0; i <= 10; i = i + 1) j = i + j; return j; }",
        55,
    );
}

#[test]
fn while_loop() {
    assert_exit_status(
        r"int main() {
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
    assert_exit_status("int main() { return ret3(); }", 3);
    assert_exit_status("int main() { return ret5(); }", 5);
    assert_exit_status("int main() { return add(4, 5); }", 9);
    assert_exit_status("int main() { return sub(5, 3); }", 2);
    assert_exit_status("int main() { return add6(1, 2, 3, 4, 5, 6); }", 21);
    assert_exit_status(
        "int main() { return add6(1,2,add6(3,4,5,6,7,8),9,10,11); }",
        66,
    );
    assert_exit_status(
        "int main() { return add6(1,2,add6(3,add6(4,5,6,7,8,9),10,11,12,13),14,15,16); }",
        136,
    );
}

#[test]
fn string_literal() {
    assert_exit_status(r#"int main() { return ""[0]; }"#, 0);
    assert_exit_status(r#"int main() { return sizeof(""); }"#, 1);
    assert_exit_status(r#"int main() { return "abc"[0]; }"#, 97);
    assert_exit_status(r#"int main() { return "abc"[1]; }"#, 98);
    assert_exit_status(r#"int main() { return "abc"[2]; }"#, 99);
    assert_exit_status(r#"int main() { return "abc"[3]; }"#, 0);
    assert_exit_status(r#"int main() { return sizeof("abc"); }"#, 4);
}

#[test]
fn char_type() {
    assert_exit_status("int main() { char x=1; return x; }", 1);
    assert_exit_status("int main() { char x=1; char y=2; return x; }", 1);
    assert_exit_status("int main() { char x=1; char y=2; return y; }", 2);
    assert_exit_status("int main() { char x; return sizeof(x); }", 1);
    assert_exit_status("int main() { char x[10]; return sizeof(x); }", 10);
    assert_exit_status(
        "int sub_char(char a, char b, char c) { return a-b-c; } int main() { return sub_char(7, 3, 3); }",
        1,
    );
}

fn define_functions(cc: &str, dir: &TempDir) -> PathBuf {
    let fn_defs = dir.path().join("fn_defs.c");
    std::fs::write(
        &fn_defs,
        r"
            int ret3() { return 3; }
            int ret5() { return 5; }
            int add(int x, int y) { return x + y; }
            int sub(int x, int y) { return x - y; }

            int add6(int a, int b, int c, int d, int e, int f) {
                return a + b + c + d + e + f;
            }
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
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let program_path = dir.path().join("program.c");
    std::fs::write(&program_path, program).unwrap();
    let asm_out = Command::new(compiler)
        .arg(&program_path)
        .output()
        .expect("failed to run the compiler");
    assert!(
        asm_out.status.success(),
        "{}\n---------\n{}",
        program,
        String::from_utf8(asm_out.stderr).unwrap()
    );

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
            .success(),
        "\nprogram: {program}\nassembly:\n---------\n{}\n---------",
        String::from_utf8(asm_out.stdout).unwrap(),
    );

    let assemble = Command::new(exe_path).output().expect("failed to assemble");
    assert_eq!(
        assemble.status.code(),
        Some(expected_status),
        "\nprogram: {program}\nassembly:\n---------\n{}\n---------",
        String::from_utf8(asm_out.stdout).unwrap(),
    );
}
