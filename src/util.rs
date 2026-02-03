pub fn errx(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[derive(Debug, Copy, Clone)]
pub struct Info {
    pub index: usize,
}
