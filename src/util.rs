pub fn errx(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
