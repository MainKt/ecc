pub fn errx(msg: &str) -> ! {
    let program = std::env::args().next().unwrap_or_else(|| "ecc".into());
    eprintln!("{program}: {msg}");
    std::process::exit(1);
}

pub fn err_at(msg: &str, input: &str, error_index: usize) -> ! {
    eprintln!("{input}");
    eprintln!("{:>width$}^", "", width = error_index);
    errx(msg)
}
