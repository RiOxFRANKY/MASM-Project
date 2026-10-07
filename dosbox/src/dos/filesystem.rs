use std::fs;
use std::path::{Path, PathBuf};

pub struct Drive {
    root: PathBuf,
    current: Vec<String>,
}

pub struct Entry {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
}

impl Drive {
    pub fn mount(root: &Path) -> std::io::Result<Self> {
        let root = fs::canonicalize(root)?;
        let text = root.to_string_lossy().to_string();
        let root = match text.strip_prefix(r"\\?\") {
            Some(plain) if !plain.starts_with("UNC\\") => PathBuf::from(plain),
            _ => root,
        };
        if !root.is_dir() {
            return Err(std::io::Error::other(format!("{} is not a directory", root.display())));
        }
        Ok(Drive { root, current: Vec::new() })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn current_text(&self) -> String {
        format!("C:\\{}", self.current.join("\\"))
    }

    pub fn current_relative(&self) -> String {
        self.current.join("\\")
    }

    fn components(&self, dos_path: &str) -> Result<Vec<String>, u16> {
        let mut text = dos_path.trim().replace('/', "\\");
        if text.len() >= 2 && text.as_bytes()[1] == b':' {
            if !text[..1].eq_ignore_ascii_case("c") {
                return Err(super::ERROR_PATH_NOT_FOUND);
            }
            text = text[2..].to_string();
        }
        let mut parts = if text.starts_with('\\') { Vec::new() } else { self.current.clone() };
        for piece in text.split('\\') {
            match piece {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                _ => parts.push(piece.to_ascii_uppercase()),
            }
        }
        Ok(parts)
    }

    fn find_child(directory: &Path, name: &str) -> Option<PathBuf> {
        let exact = directory.join(name);
        if exact.exists() {
            return Some(exact);
        }
        fs::read_dir(directory).ok()?.flatten().find_map(|entry| {
            let candidate = entry.file_name().to_string_lossy().to_string();
            candidate.eq_ignore_ascii_case(name).then(|| entry.path())
        })
    }

    pub fn resolve(&self, dos_path: &str) -> Result<PathBuf, u16> {
        let parts = self.components(dos_path)?;
        let mut host = self.root.clone();
        for (index, part) in parts.iter().enumerate() {
            match Self::find_child(&host, part) {
                Some(found) => host = found,
                None if index + 1 == parts.len() => host = host.join(part),
                None => return Err(super::ERROR_PATH_NOT_FOUND),
            }
        }
        Ok(host)
    }

    pub fn change_directory(&mut self, dos_path: &str) -> Result<(), u16> {
        let parts = self.components(dos_path)?;
        let host = self.resolve(dos_path)?;
        if !host.is_dir() {
            return Err(super::ERROR_PATH_NOT_FOUND);
        }
        self.current = parts;
        Ok(())
    }

    pub fn list(&self, dos_directory: &str) -> Result<Vec<Entry>, u16> {
        let host = self.resolve(dos_directory)?;
        let reader = fs::read_dir(&host).map_err(|_| super::ERROR_PATH_NOT_FOUND)?;
        let mut entries: Vec<Entry> = reader
            .flatten()
            .filter_map(|entry| {
                let metadata = entry.metadata().ok()?;
                Some(Entry {
                    name: entry.file_name().to_string_lossy().to_ascii_uppercase(),
                    is_directory: metadata.is_dir(),
                    size: metadata.len(),
                    modified: metadata.modified().ok(),
                })
            })
            .collect();
        entries.sort_by(|left, right| right.is_directory.cmp(&left.is_directory).then(left.name.cmp(&right.name)));
        Ok(entries)
    }
}

pub fn wildcard_match(pattern: &str, name: &str) -> bool {
    let pattern = pattern.to_ascii_uppercase();
    let name = name.to_ascii_uppercase();
    let (pattern_base, pattern_ext) = pattern.split_once('.').unwrap_or((&pattern, ""));
    let (name_base, name_ext) = name.split_once('.').unwrap_or((&name, ""));
    let pattern_ext = if pattern.contains('.') { pattern_ext } else { "*" };
    part_match(pattern_base, name_base) && part_match(pattern_ext, name_ext)
}

fn part_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let mut index = 0;
    for (position, character) in pattern.iter().enumerate() {
        match character {
            '*' => return true,
            '?' => index += 1,
            _ => {
                if text.get(index) != Some(character) {
                    return false;
                }
                index += 1;
            }
        }
        if position + 1 == pattern.len() {
            return index >= text.len();
        }
    }
    text.is_empty()
}
