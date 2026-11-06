//! Every `tr!` message in the workspace must have a translation with the same placeholders.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use kanami_i18n::zh_cn::ENTRIES;

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Reads the Rust string literal at the start of `source`, resolving escapes.
fn literal(source: &str) -> Option<String> {
    let mut chars = source.strip_prefix('"')?.chars().peekable();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                '\n' => while chars.next_if(|c| c.is_whitespace()).is_some() {},
                other => out.push(other),
            },
            _ => out.push(c),
        }
    }
    None
}

fn messages() -> Vec<(PathBuf, String)> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    let root = manifest.join("../..").canonicalize().unwrap();
    let mut files = Vec::new();
    rust_files(&root.join("crates"), &mut files);
    rust_files(&root.join("apps/settings/src-tauri/src"), &mut files);
    // This crate only mentions `tr!` in its docs and in this test.
    files.retain(|file| !file.starts_with(&manifest));
    let mut found = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file).unwrap();
        for (start, _) in source.match_indices("tr!(") {
            // Skip longer macro names such as `include_str!(`.
            let name_char = |c: char| c.is_alphanumeric() || c == '_';
            if source[..start].ends_with(name_char) {
                continue;
            }
            let rest = source[start + 4..].trim_start();
            if let Some(message) = literal(rest) {
                found.push((file.clone(), message));
            }
        }
    }
    found
}

fn placeholders(message: &str) -> BTreeSet<&str> {
    let is_name =
        |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    message
        .split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(name, _)| name))
        .filter(|name| is_name(name))
        .collect()
}

#[test]
fn every_message_is_translated() {
    let found = messages();
    assert!(!found.is_empty());
    let missing: Vec<_> = found
        .iter()
        .filter(|(_, message)| !ENTRIES.iter().any(|(en, _)| en == message))
        .map(|(file, message)| format!("{}: {message:?}", file.display()))
        .collect();
    assert!(
        missing.is_empty(),
        "missing translations:\n{}",
        missing.join("\n")
    );
}

#[test]
fn translations_keep_their_placeholders() {
    for (en, zh) in ENTRIES {
        assert_eq!(placeholders(en), placeholders(zh), "{en:?}");
    }
}

#[test]
fn catalog_has_no_duplicates() {
    let mut seen = BTreeSet::new();
    for (en, _) in ENTRIES {
        assert!(seen.insert(*en), "duplicate entry: {en:?}");
    }
}
