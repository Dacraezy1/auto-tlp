// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dacraezy1
//! Small helpers for reading sysfs/procfs and running commands.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Read a file, trimmed. `None` if missing, unreadable or empty.
pub fn read<P: AsRef<Path>>(p: P) -> Option<String> {
    let s = fs::read_to_string(p).ok()?;
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

pub fn read_u64<P: AsRef<Path>>(p: P) -> Option<u64> {
    read(p)?.split_whitespace().next()?.parse().ok()
}

pub fn exists<P: AsRef<Path>>(p: P) -> bool {
    p.as_ref().exists()
}

/// Sorted directory entries (empty vec if the dir does not exist).
pub fn list_dir<P: AsRef<Path>>(p: P) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(p)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    v.sort();
    v
}

pub fn base(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Final path component of a symlink target (e.g. `device/driver` -> `ahci`).
pub fn link_base<P: AsRef<Path>>(p: P) -> Option<String> {
    fs::read_link(p)
        .ok()
        .and_then(|t| t.file_name().map(|s| s.to_string_lossy().into_owned()))
}

/// Run a command and return its trimmed stdout (even on non-zero exit,
/// because e.g. `systemctl is-active` exits 3 when inactive).
pub fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn command_exists(name: &str) -> bool {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    for extra in ["/usr/sbin", "/sbin", "/usr/bin", "/bin"] {
        dirs.push(PathBuf::from(extra));
    }
    dirs.iter().any(|d| d.join(name).is_file())
}

pub fn module_loaded(name: &str) -> bool {
    exists(format!("/sys/module/{}", name))
}

/// Parse kernel style choice lists: `"s2idle [deep]"` -> (["s2idle","deep"], Some("deep")).
pub fn parse_choices(s: &str) -> (Vec<String>, Option<String>) {
    let mut cur = None;
    let mut v = Vec::new();
    for tok in s.split_whitespace() {
        if tok.starts_with('[') && tok.ends_with(']') && tok.len() > 2 {
            let t = tok[1..tok.len() - 1].to_string();
            cur = Some(t.clone());
            v.push(t);
        } else {
            v.push(tok.to_string());
        }
    }
    (v, cur)
}

pub fn human_bytes(b: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1000.0 && i < U.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 {
        format!("{} B", b)
    } else {
        format!("{:.1} {}", v, U[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_with_current() {
        let (v, cur) = parse_choices("s2idle [deep]");
        assert_eq!(v, ["s2idle", "deep"]);
        assert_eq!(cur.as_deref(), Some("deep"));
    }

    #[test]
    fn choices_without_current() {
        let (v, cur) = parse_choices("performance powersave");
        assert_eq!(v, ["performance", "powersave"]);
        assert!(cur.is_none());
        assert_eq!(parse_choices(""), (vec![], None));
    }

    #[test]
    fn bytes_are_human_readable() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1_000_000_000), "1.0 GB");
        assert_eq!(human_bytes(512_110_190_592), "512.1 GB");
    }

    #[test]
    fn missing_files_are_none() {
        assert!(read("/definitely/not/here").is_none());
        assert!(read_u64("/definitely/not/here").is_none());
        assert!(list_dir("/definitely/not/here").is_empty());
        assert!(!command_exists("surely-not-a-real-command-xyz"));
    }
}
