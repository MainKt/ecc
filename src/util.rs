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

pub fn info_msg(
    f: &mut std::fmt::Formatter<'_>,
    file: &str,
    content: &str,
    ptr_index: usize,
) -> std::fmt::Result {
    let start = content[..ptr_index].rfind('\n').map_or(0, |i| i + 1);
    let end = content[ptr_index..]
        .find('\n')
        .map_or(content.len(), |i| ptr_index + i);
    let line = &content[start..end].trim_end_matches('\r');
    let line_no = content[..ptr_index].matches('\n').count() + 1;
    let info = format!("{file}:{line_no}: ");
    writeln!(f, "{info}{line}")?;
    let column = info.len() + ptr_index - start;
    write!(f, "{:>width$}^ ", "", width = column)
}
