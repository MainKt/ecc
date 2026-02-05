use std::sync::atomic::{AtomicUsize, Ordering};

pub fn errx(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[derive(Debug, Copy, Clone)]
pub struct Info {
    pub index: usize,
}

pub fn unique_name() -> String {
    static ID: AtomicUsize = AtomicUsize::new(0);
    format!(".L..{}", ID.fetch_add(1, Ordering::SeqCst))
}
