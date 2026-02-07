use std::sync::atomic::{AtomicUsize, Ordering};

pub fn errx(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[derive(Debug, Copy, Clone)]
pub struct Info {
    pub index: usize,
    pub line: usize,
}

pub fn unique_name() -> String {
    static ID: AtomicUsize = AtomicUsize::new(0);
    format!(".L..{}", ID.fetch_add(1, Ordering::SeqCst))
}

pub fn info_msg(
    f: &mut std::fmt::Formatter<'_>,
    file: &str,
    content: &str,
    info: Info,
) -> std::fmt::Result {
    let start = content[..info.index].rfind('\n').map_or(0, |i| i + 1);
    let end = content[info.index..]
        .find('\n')
        .map_or(content.len(), |i| info.index + i);
    let line = &content[start..end].trim_end_matches('\r');
    let info_msg = format!("{file}:{}: ", info.line);
    writeln!(f, "{info_msg}{line}")?;
    let column = info_msg.len() + info.index - start;
    write!(f, "{:>width$}^ ", "", width = column)
}

pub fn usage(mut stream: impl std::io::Write) {
    let program = std::env::args().next().unwrap_or_else(|| "diff3".into());
    writeln!(stream, "Usage: {program} [ -o <path> ] <file>").unwrap();
}
