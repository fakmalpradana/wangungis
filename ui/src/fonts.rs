//! Bundled Noto fonts (ADR-010). egui's own fonts stay first, so Latin text looks
//! as before; Noto only fills in what they lack. One CJK face is registered, the
//! one for the chosen language, so Han characters use that region's forms.

use eframe::egui::{FontData, FontDefinitions, FontFamily};
use wangungis_i18n::Lang;

static LATIN: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");
static ARABIC: &[u8] = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
static DEVANAGARI: &[u8] = include_bytes!("../fonts/NotoSansDevanagari-Regular.ttf");
static THAI: &[u8] = include_bytes!("../fonts/NotoSansThai-Regular.ttf");
static CJK: &[u8] = include_bytes!("../fonts/NotoSansCJK-Regular.ttc");

/// Face of the CJK collection for `lang`: 0 JP, 1 KR, 2 SC, 3 TC.
/// Languages without Han text get SC, which only has to cover the glyphs.
pub fn cjk_face(lang: Lang) -> u32 {
    match lang {
        Lang::Ja => 0,
        Lang::Ko => 1,
        Lang::ZhHant => 3,
        _ => 2,
    }
}

/// egui's default fonts, then the Noto fonts as fallbacks in both families.
/// `from_static` shares the bytes: nothing is copied, also not per language.
pub fn definitions(lang: Lang) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let noto = [
        ("NotoSans", FontData::from_static(LATIN)),
        ("NotoSansArabic", FontData::from_static(ARABIC)),
        ("NotoSansDevanagari", FontData::from_static(DEVANAGARI)),
        ("NotoSansThai", FontData::from_static(THAI)),
        (
            "NotoSansCJK",
            FontData::from_blob(std::sync::Arc::new(CJK), cjk_face(lang)),
        ),
    ];
    for (name, data) in noto {
        defs.font_data.insert(name.to_owned(), data.into());
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            defs.families
                .entry(family)
                .or_default()
                .push(name.to_owned());
        }
    }
    // egui splits text into runs by font. With a Latin font first, the spaces
    // of an Arabic sentence come from it, so every word is its own run and the
    // words are laid out left to right. Arabic first keeps a sentence one run,
    // which the shaper orders right to left. Mixed text can still swap (no bidi).
    if lang.rtl() {
        for list in defs.families.values_mut() {
            list.retain(|n| n != "NotoSansArabic");
            list.insert(0, "NotoSansArabic".to_owned());
        }
    }
    defs
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Context, FontId, RawInput};
    use std::collections::BTreeSet;

    #[test]
    fn embedded_files_have_the_expected_size() {
        assert_eq!(LATIN.len(), 431_364);
        assert_eq!(ARABIC.len(), 142_140);
        assert_eq!(DEVANAGARI.len(), 184_228);
        assert_eq!(THAI.len(), 20_960);
        assert_eq!(CJK.len(), 19_484_784);
    }

    #[test]
    fn cjk_face_per_language() {
        let want = |l| match l {
            Lang::Ja => 0,
            Lang::Ko => 1,
            Lang::ZhHans => 2,
            Lang::ZhHant => 3,
            _ => 2,
        };
        for lang in Lang::ALL {
            assert_eq!(cjk_face(lang), want(lang), "{lang:?}");
            let defs = definitions(lang);
            assert_eq!(defs.font_data["NotoSansCJK"].index, want(lang), "{lang:?}");
        }
    }

    #[test]
    fn egui_fonts_stay_first_and_noto_follows() {
        let defs = definitions(Lang::Id);
        let default = FontDefinitions::default();
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let list = &defs.families[&family];
            let base = &default.families[&family];
            assert_eq!(&list[..base.len()], base.as_slice());
            assert_eq!(
                &list[base.len()..],
                [
                    "NotoSans",
                    "NotoSansArabic",
                    "NotoSansDevanagari",
                    "NotoSansThai",
                    "NotoSansCJK"
                ]
            );
        }
    }

    #[test]
    fn arabic_comes_first_for_arabic_only() {
        for lang in Lang::ALL {
            let defs = definitions(lang);
            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                let list = &defs.families[&family];
                assert_eq!(list[0] == "NotoSansArabic", lang.rtl(), "{lang:?}");
                let arabic = list.iter().filter(|n| *n == "NotoSansArabic").count();
                assert_eq!(arabic, 1, "{lang:?}");
            }
        }
    }

    /// Every character the catalogs and the language menu can show.
    fn screen_chars() -> BTreeSet<char> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../i18n/locales");
        let mut chars = BTreeSet::new();
        let mut files = 0;
        for entry in std::fs::read_dir(dir).expect("locales folder") {
            let path = entry.expect("entry").path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            files += 1;
            let text = std::fs::read_to_string(&path).expect("catalog");
            let map: std::collections::BTreeMap<String, String> =
                serde_json::from_str(&text).expect("flat JSON object of strings");
            chars.extend(map.values().flat_map(|v| v.chars()));
        }
        assert!(files >= 2, "catalogs not found");
        for lang in Lang::ALL {
            chars.extend(lang.native_name().chars());
        }
        chars.retain(|c| !c.is_whitespace() && !c.is_control());
        // Not vacuous: the menu names alone bring in every script.
        for c in ['日', '한', '中', 'ع', 'ไ', 'ह', 'Р'] {
            assert!(chars.contains(&c), "{c}");
        }
        chars
    }

    #[test]
    fn every_screen_character_has_a_glyph() {
        let chars = screen_chars();
        for lang in Lang::ALL {
            let ctx = Context::default();
            ctx.set_fonts(definitions(lang));
            let mut out = ctx.run_ui(RawInput::default(), |_| {});
            out.textures_delta.clear();
            let id = FontId::proportional(14.0);
            let missing: String = ctx.fonts_mut(|f| {
                chars
                    .iter()
                    .copied()
                    .filter(|&c| !f.has_glyph(&id, c))
                    .collect()
            });
            assert!(missing.is_empty(), "{lang:?}: no glyph for {missing:?}");
        }
    }
}
