//! Bounded filesystem bridge for the PocketShell Library workspace.
use std::env;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

const DEFAULT_ROOTS: [&str; 4] = ["/storage/docs", "/storage/books", "/storage/reports", "/storage/library"];
const DEFAULT_DATA: &str = "/storage/pocketshell/library";
pub const MAX_READ: usize = 16 * 1024;

fn roots() -> Vec<PathBuf> {
    env::var_os("POCKETSHELL_LIBRARY_ROOTS").map(|value| env::split_paths(&value).collect()).unwrap_or_else(|| DEFAULT_ROOTS.iter().map(PathBuf::from).collect())
}
fn data_dir() -> PathBuf {
    env::var_os("POCKETSHELL_LIBRARY_DATA_ROOT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_DATA))
}
fn allowed(path: &Path) -> bool {
    if path.components().any(|part| matches!(part, Component::ParentDir)) { return false; }
    let Ok(target) = path.canonicalize() else { return false; };
    roots().iter().filter_map(|root| root.canonicalize().ok()).any(|root| target.starts_with(root))
}
fn state_name(name: &str) -> bool {
    ["progress.json", "bookmarks.json", "history.json", "settings.json"].contains(&name)
}
fn escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
}

pub fn command(input: &str) -> String {
    let mut fields = input.splitn(4, '\t');
    let Some(op) = fields.next() else { return String::new() };
    match op {
        "LIST" => {
            let Some(root) = fields.next().map(PathBuf::from) else { return "[]".into() };
            if !allowed(&root) { return "[]".into() }
            let Ok(entries) = fs::read_dir(root) else { return "[]".into() };
            let values = entries.filter_map(Result::ok).filter_map(|entry| {
                let path = entry.path(); if entry.file_type().ok()?.is_symlink() || !allowed(&path) { return None }
                let metadata = entry.metadata().ok()?;
                Some(format!("{{\"path\":\"{}\",\"directory\":{},\"size\":{}}}", escape(&path.to_string_lossy()), metadata.is_dir(), metadata.len()))
            }).collect::<Vec<_>>();
            format!("[{}]", values.join(","))
        }
        "STAT" => {
            let Some(path) = fields.next().map(PathBuf::from) else { return String::new() };
            if !allowed(&path) { return String::new() }
            match fs::metadata(path) { Ok(metadata) => format!("{{\"size\":{},\"directory\":{}}}", metadata.len(), metadata.is_dir()), Err(_) => String::new() }
        }
        "READ" => {
            let (Some(path), Some(offset), Some(length)) = (fields.next(), fields.next(), fields.next()) else { return String::new() };
            let path = Path::new(path); if !allowed(path) { return String::new() }
            let (Ok(offset), Ok(length)) = (offset.parse::<u64>(), length.parse::<usize>()) else { return String::new() };
            let Ok(mut file) = fs::File::open(path) else { return String::new() };
            let Ok(metadata) = file.metadata() else { return String::new() };
            if !metadata.is_file() || offset >= metadata.len() { return String::new() }
            if file.seek(SeekFrom::Start(offset)).is_err() { return String::new() }
            let mut bytes = vec![0; length.min(MAX_READ)];
            let Ok(count) = file.read(&mut bytes) else { return String::new() };
            bytes.truncate(count);
            let start = bytes.iter().position(|byte| byte & 0xc0 != 0x80).unwrap_or(bytes.len());
            let bytes = &bytes[start..];
            let end = match std::str::from_utf8(bytes) {
                Ok(_) => bytes.len(),
                Err(error) if error.error_len().is_none() => error.valid_up_to(),
                Err(_) => bytes.len(),
            };
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        }
        "STATE_GET" => {
            let Some(name) = fields.next() else { return String::new() };
            if !state_name(name) { return String::new() }
            fs::read_to_string(data_dir().join(name)).unwrap_or_default()
        }
        "STATE_SET" => {
            let (Some(name), Some(value)) = (fields.next(), fields.next()) else { return String::new() };
            if !state_name(name) || value.len() > 1024 * 1024 { return String::new() }
            let directory = data_dir(); if fs::create_dir_all(&directory).is_err() { return String::new() }
            let destination = directory.join(name); let temporary = destination.with_extension("json.tmp");
            if fs::write(&temporary, value).is_err() || fs::rename(temporary, destination).is_err() { return String::new() }
            "ok".into()
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confines_symlinks_and_preserves_multibyte_windows() {
        let root = env::temp_dir().join(format!("pocketshell-library-test-{}", std::process::id()));
        let docs = root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        env::set_var("POCKETSHELL_LIBRARY_ROOTS", &docs);
        let outside = root.join("secret.txt");
        fs::write(&outside, "private").unwrap();
        std::os::unix::fs::symlink(&outside, docs.join("escape.txt")).unwrap();
        assert_eq!(command(&format!("READ\t{}\t0\t100", docs.join("escape.txt").display())), "");
        std::os::unix::fs::symlink(&docs, docs.join("loop")).unwrap();
        let list = command(&format!("LIST\t{}", docs.display()));
        assert!(!list.contains("escape.txt"));
        assert!(!list.contains("loop"));
        let text = format!("{}\nnext line\n", "日本語😀".repeat(3000));
        let path = docs.join("japanese.txt");
        fs::write(&path, &text).unwrap();
        let mut reconstructed = String::new();
        while reconstructed.len() < text.len() {
            let chunk = command(&format!("READ\t{}\t{}\t{}", path.display(), reconstructed.len(), MAX_READ));
            assert!(!chunk.is_empty());
            assert!(!chunk.contains('\u{fffd}'));
            reconstructed.push_str(&chunk);
        }
        assert_eq!(reconstructed, text);
        fs::remove_dir_all(root).unwrap();
    }
}
