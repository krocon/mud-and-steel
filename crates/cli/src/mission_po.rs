//! Translations of mission texts (prompt 043): `<map>/missions/<id>.<lang>.po`
//! next to every mission file, one entry per text the player reads
//! (`sim::mission::mission_texts`), the English text as `msgid`. The game
//! loads them in `godot/scenes/menu/missions.gd`.
//!
//! `just mission-po` writes the files: new texts get an empty `msgstr`,
//! existing translations are kept, texts no longer in the mission are
//! dropped. `--check` (part of `just l10n-check`) changes nothing and fails
//! when a file is missing, out of date or has an untranslated text.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Languages besides English (the language of the mission files).
pub const LANGUAGES: [&str; 1] = ["de"];

/// Every mission file below `maps`, sorted.
pub fn mission_files(maps: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(dirs) = std::fs::read_dir(maps) else {
        return files;
    };
    for dir in dirs.flatten() {
        let Ok(missions) = std::fs::read_dir(dir.path().join("missions")) else {
            continue;
        };
        for f in missions.flatten() {
            let path = f.path();
            if path.extension().is_some_and(|e| e == "toml") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The `msgid` → `msgstr` pairs of a PO file (multi-line strings joined).
pub fn parse_po(text: &str) -> BTreeMap<String, String> {
    let mut entries = BTreeMap::new();
    let mut id: Option<String> = None;
    let mut current: Option<(bool, String)> = None;
    let mut finish = |current: &mut Option<(bool, String)>, id: &mut Option<String>| {
        if let Some((is_str, value)) = current.take() {
            if is_str {
                if let Some(key) = id.take()
                    && !key.is_empty()
                {
                    entries.insert(key, value);
                }
            } else {
                *id = Some(value);
            }
        }
    };
    for line in text.lines() {
        let line = line.trim();
        let quoted =
            |rest: &str| unescape(rest.trim().trim_start_matches('"').trim_end_matches('"'));
        if let Some(rest) = line.strip_prefix("msgid ") {
            finish(&mut current, &mut id);
            current = Some((false, quoted(rest)));
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            finish(&mut current, &mut id);
            current = Some((true, quoted(rest)));
        } else if line.starts_with('"') {
            if let Some((_, value)) = current.as_mut() {
                value.push_str(&quoted(line));
            }
        }
    }
    finish(&mut current, &mut id);
    entries
}

/// The PO file of `texts` in `lang`, translations taken from `old`.
pub fn render_po(
    mission: &str,
    lang: &str,
    texts: &[String],
    old: &BTreeMap<String, String>,
) -> String {
    let mut out = format!(
        "# Translation of the mission {mission} (prompt 043). msgid: the English text of the\n\
         # mission file; `just mission-po` adds new texts, see docs/MISSIONS.md.\n\
         msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\"Language: {lang}\\n\"\n"
    );
    for t in texts {
        let translated = old.get(t).cloned().unwrap_or_default();
        out.push_str(&format!(
            "\nmsgid \"{}\"\nmsgstr \"{}\"\n",
            escape(t),
            escape(&translated)
        ));
    }
    out
}

/// Writes (or with `check` only compares) the PO files; returns the problems.
pub fn run(maps: &Path, check: bool) -> Vec<String> {
    let mut problems = Vec::new();
    for file in mission_files(maps) {
        let texts = match std::fs::read_to_string(&file)
            .map_err(|e| e.to_string())
            .and_then(|t| sim::mission::mission_texts(&t).map_err(|e| e.0))
        {
            Ok(t) => t,
            Err(e) => {
                problems.push(format!("{}: {e}", file.display()));
                continue;
            }
        };
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        for lang in LANGUAGES {
            let po = file.with_file_name(format!("{stem}.{lang}.po"));
            let existing = std::fs::read_to_string(&po).unwrap_or_default();
            let old = parse_po(&existing);
            let new = render_po(&stem, lang, &texts, &old);
            let missing = texts
                .iter()
                .filter(|t| old.get(*t).is_none_or(String::is_empty))
                .count();
            if missing > 0 {
                problems.push(format!(
                    "{}: {missing} text(s) without translation",
                    po.display()
                ));
            }
            if new != existing {
                if check {
                    problems.push(format!(
                        "{} is out of date (run `just mission-po`)",
                        po.display()
                    ));
                } else if let Err(e) = std::fs::write(&po, new) {
                    problems.push(format!("cannot write {}: {e}", po.display()));
                } else {
                    println!("wrote {}", po.display());
                }
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn po_round_trip_keeps_translations() {
        let texts = vec!["Say \"hi\"\nnow".to_owned(), "New".to_owned()];
        let mut old = BTreeMap::new();
        old.insert(texts[0].clone(), "Sag \"hallo\"\njetzt".to_owned());
        let po = render_po("m", "de", &texts, &old);
        let back = parse_po(&po);
        assert_eq!(
            back.get(&texts[0]).map(String::as_str),
            Some("Sag \"hallo\"\njetzt")
        );
        assert_eq!(back.get("New").map(String::as_str), Some(""));
        assert_eq!(render_po("m", "de", &texts, &back), po);
    }
}
