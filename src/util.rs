pub fn errx(msg: &str) -> ! {
    let program = std::env::args().next().unwrap_or_else(|| "ecc".into());
    eprintln!("{program}: {msg}");
    std::process::exit(1);
}
