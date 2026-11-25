//! Text for the screen, in the language the user picked. Keys are dotted English
//! names; the catalogs in `locales/` hold the text. See `SPEC.md`.

use std::collections::HashMap;
use std::fmt::{Display, Write};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

/// Languages the menu offers, in menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Id,
    En,
    ZhHans,
    ZhHant,
    Ja,
    Ko,
    Ar,
    Es,
    PtBr,
    Fr,
    De,
    Ru,
    Tr,
    Vi,
    Th,
    Hi,
}

impl Lang {
    pub const ALL: [Lang; 16] = [
        Lang::Id,
        Lang::En,
        Lang::ZhHans,
        Lang::ZhHant,
        Lang::Ja,
        Lang::Ko,
        Lang::Ar,
        Lang::Es,
        Lang::PtBr,
        Lang::Fr,
        Lang::De,
        Lang::Ru,
        Lang::Tr,
        Lang::Vi,
        Lang::Th,
        Lang::Hi,
    ];

    /// Code in file names and in the saved preference.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Id => "id",
            Lang::En => "en",
            Lang::ZhHans => "zh-Hans",
            Lang::ZhHant => "zh-Hant",
            Lang::Ja => "ja",
            Lang::Ko => "ko",
            Lang::Ar => "ar",
            Lang::Es => "es",
            Lang::PtBr => "pt-BR",
            Lang::Fr => "fr",
            Lang::De => "de",
            Lang::Ru => "ru",
            Lang::Tr => "tr",
            Lang::Vi => "vi",
            Lang::Th => "th",
            Lang::Hi => "hi",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == code)
    }

    /// The language's name in itself, as the menu shows it.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Id => "Bahasa Indonesia",
            Lang::En => "English",
            Lang::ZhHans => "简体中文",
            Lang::ZhHant => "繁體中文",
            Lang::Ja => "日本語",
            Lang::Ko => "한국어",
            Lang::Ar => "العربية",
            Lang::Es => "Español",
            Lang::PtBr => "Português (Brasil)",
            Lang::Fr => "Français",
            Lang::De => "Deutsch",
            Lang::Ru => "Русский",
            Lang::Tr => "Türkçe",
            Lang::Vi => "Tiếng Việt",
            Lang::Th => "ไทย",
            Lang::Hi => "हिन्दी",
        }
    }

    /// Written right to left. Layout does not follow yet (see SPEC.md).
    pub fn rtl(self) -> bool {
        self == Lang::Ar
    }

    /// A machine draft not yet checked by a native speaker (see `GLOSSARY.md`).
    pub fn machine_draft(self) -> bool {
        !matches!(self, Lang::Id | Lang::En)
    }

    /// The catalog file. Every language has one.
    fn source(self) -> Option<&'static str> {
        Some(match self {
            Lang::Id => include_str!("../locales/id.json"),
            Lang::En => include_str!("../locales/en.json"),
            Lang::ZhHans => include_str!("../locales/zh-Hans.json"),
            Lang::ZhHant => include_str!("../locales/zh-Hant.json"),
            Lang::Ja => include_str!("../locales/ja.json"),
            Lang::Ko => include_str!("../locales/ko.json"),
            Lang::Ar => include_str!("../locales/ar.json"),
            Lang::Es => include_str!("../locales/es.json"),
            Lang::PtBr => include_str!("../locales/pt-BR.json"),
            Lang::Fr => include_str!("../locales/fr.json"),
            Lang::De => include_str!("../locales/de.json"),
            Lang::Ru => include_str!("../locales/ru.json"),
            Lang::Tr => include_str!("../locales/tr.json"),
            Lang::Vi => include_str!("../locales/vi.json"),
            Lang::Th => include_str!("../locales/th.json"),
            Lang::Hi => include_str!("../locales/hi.json"),
        })
    }
}

type Catalog = HashMap<&'static str, &'static str>;

static CURRENT: AtomicU8 = AtomicU8::new(Lang::Id as u8);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    Lang::ALL[usize::from(CURRENT.load(Ordering::Relaxed))]
}

/// Parsed once per language. The text is leaked: the catalogs live as long as
/// the program.
fn catalog(lang: Lang) -> Option<&'static Catalog> {
    static CATALOGS: [OnceLock<Option<Catalog>>; 16] = [const { OnceLock::new() }; 16];
    CATALOGS[lang as usize]
        .get_or_init(|| {
            let text = lang.source()?;
            let raw: HashMap<String, String> =
                serde_json::from_str(text).expect("a catalog is a flat JSON object of strings");
            let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
            Some(raw.into_iter().map(|(k, v)| (leak(k), leak(v))).collect())
        })
        .as_ref()
}

/// The text of `key` in the current language, else English, else Indonesian,
/// else the key itself.
pub fn t(key: &str) -> &'static str {
    for lang in [current(), Lang::En, Lang::Id] {
        if let Some(text) = catalog(lang).and_then(|c| c.get(key)) {
            return text;
        }
    }
    // A missing key is a bug the tests catch. Kept so it still shows, once.
    static MISSING: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut seen = MISSING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(known) = seen.iter().find(|k| **k == key) {
        return known;
    }
    let leaked: &'static str = Box::leak(key.to_owned().into_boxed_str());
    seen.push(leaked);
    leaked
}

/// [`t`] with `{0}`, `{1}`, ... replaced by `args`. An argument's own text is
/// never read as a placeholder.
pub fn tf(key: &str, args: &[&dyn Display]) -> String {
    let mut rest = t(key);
    let mut out = String::with_capacity(rest.len() + 16);
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let arg = after.find('}').and_then(|close| {
            let n: usize = after[..close].parse().ok()?;
            Some((args.get(n)?, close))
        });
        match arg {
            Some((arg, close)) => {
                let _ = write!(out, "{arg}");
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests;
