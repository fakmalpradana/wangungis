use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Tests that touch the global language take this, and put `Id` back.
static LANG_LOCK: Mutex<()> = Mutex::new(());

fn lang_lock() -> std::sync::MutexGuard<'static, ()> {
    LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every catalog on disk, by file stem.
fn catalogs() -> BTreeMap<String, BTreeMap<String, String>> {
    let mut found = BTreeMap::new();
    for entry in std::fs::read_dir(root().join("locales")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(&path).unwrap();
        let map = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{stem}: {e}"));
        found.insert(stem, map);
    }
    found
}

fn placeholders(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        if let Some(close) = rest.find('}')
            && !rest[..close].is_empty()
            && rest[..close].bytes().all(|b| b.is_ascii_digit())
        {
            found.insert(rest[..close].to_owned());
        }
    }
    found
}

fn rust_files(dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        // Hidden folders hold worktrees and tool state, not our code.
        if name.starts_with('.') || name == "target" || name == "i18n" {
            continue;
        }
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(std::fs::read_to_string(&path).unwrap());
        }
    }
}

fn workspace_sources() -> Vec<String> {
    let mut out = Vec::new();
    rust_files(&root().join(".."), &mut out);
    assert!(out.len() > 5, "the scan found no workspace sources");
    out
}

#[test]
fn every_catalog_has_the_key_set_of_indonesian() {
    let all = catalogs();
    let id: BTreeSet<_> = all["id"].keys().collect();
    assert!(all.contains_key("en"));
    for (code, map) in &all {
        assert!(Lang::from_code(code).is_some(), "{code} is not a language");
        let keys: BTreeSet<_> = map.keys().collect();
        assert_eq!(keys, id, "{code}: keys differ from id.json");
    }
}

#[test]
fn placeholders_agree_across_catalogs() {
    let all = catalogs();
    for (key, text) in &all["id"] {
        let want = placeholders(text);
        for (code, map) in &all {
            assert_eq!(placeholders(&map[key]), want, "{code}: {key}");
        }
    }
}

#[test]
fn no_value_is_empty() {
    for (code, map) in catalogs() {
        for (key, text) in map {
            assert!(!text.trim().is_empty(), "{code}: {key} is empty");
        }
    }
}

#[test]
fn every_key_in_the_code_exists_and_every_key_is_used() {
    let all = catalogs();
    let id = &all["id"];
    let sources = workspace_sources();
    // `t("...")` and `tf("...", ...)` calls.
    let mut called = BTreeSet::new();
    // Any literal that looks like a key: keys also travel through `needs(..)`,
    // `Unsupported(..)`, label tables and the like.
    let mut literals = BTreeSet::new();
    let groups: BTreeSet<_> = id.keys().filter_map(|k| k.split('.').next()).collect();
    for src in &sources {
        for call in ["t(\"", "tf(\""] {
            for (at, _) in src.match_indices(call) {
                let before = src[..at].chars().next_back();
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let tail = &src[at + call.len()..];
                called.insert(tail[..tail.find('"').unwrap()].to_owned());
            }
        }
        for group in &groups {
            let start = format!("\"{group}.");
            for (at, _) in src.match_indices(&start) {
                let tail = &src[at + 1..];
                let end = tail
                    .find(|c: char| {
                        !(c.is_ascii_lowercase() || c.is_ascii_digit() || "_.".contains(c))
                    })
                    .unwrap_or(tail.len());
                if tail[end..].starts_with('"') {
                    literals.insert(tail[..end].to_owned());
                }
            }
        }
    }
    for key in called.iter().chain(&literals) {
        assert!(id.contains_key(key), "{key} is used but not in id.json");
    }
    for key in id.keys() {
        assert!(literals.contains(key), "{key} is in id.json but unused");
    }
}

#[test]
fn language_codes_round_trip() {
    let mut seen = BTreeSet::new();
    for lang in Lang::ALL {
        assert_eq!(Lang::from_code(lang.code()), Some(lang));
        assert!(seen.insert(lang.code()), "{} twice", lang.code());
        assert!(!lang.native_name().is_empty());
        assert_eq!(lang.rtl(), lang == Lang::Ar);
    }
    assert_eq!(Lang::from_code("xx"), None);
    assert_eq!(Lang::ALL[0], Lang::Id);
}

#[test]
fn t_and_tf_follow_the_language() {
    let _l = lang_lock();
    set(Lang::Id);
    assert_eq!(current(), Lang::Id);
    assert_eq!(t("menu.file"), "Berkas");
    set(Lang::En);
    assert_eq!(current(), Lang::En);
    assert_eq!(t("menu.file"), "File");
    assert_eq!(
        tf("status.editing", &[&"Grup > Objek"]),
        "Editing: Grup > Objek"
    );
    // Several, out of order, repeated, and an argument that looks like one.
    assert_eq!(
        tf("status.stats", &[&1, &"{0}", &3]),
        "Tiles: 1 · Objects: {0} · Triangles: 3"
    );
    set(Lang::Id);
}

#[test]
fn every_language_reads_its_own_catalog_and_an_unknown_key_shows_the_key() {
    // Every language has a full catalog now (checked above), so the English
    // fallback is only reachable through a key that no catalog has.
    let _l = lang_lock();
    set(Lang::Ja);
    assert_eq!(current(), Lang::Ja);
    assert_ne!(t("menu.file"), "File");
    for lang in Lang::ALL {
        assert!(lang.source().is_some(), "{lang:?}");
    }
    assert_eq!(t("no.such.key"), "no.such.key");
    assert_eq!(tf("no.such.key", &[&1]), "no.such.key");
    set(Lang::Id);
    assert_eq!(t("no.such.key"), "no.such.key");
}

#[test]
fn a_missing_argument_leaves_its_placeholder() {
    let _l = lang_lock();
    set(Lang::Id);
    assert_eq!(tf("status.distance", &[]), "Jarak: {0}");
}
