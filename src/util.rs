pub fn errx(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[derive(Debug)]
pub struct Info {
    pub index: usize,
    pub length: usize,
}
