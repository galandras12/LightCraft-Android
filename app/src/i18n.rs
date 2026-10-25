//! Translation of the strings of the phone / tablet interface.
//!
//! The upstream widgets (sliders, masking panel, presets...) are translated by the upstream catalogs
//! (`crates/ui-egui/locales`, extended by the patches for Croatian, Romanian, Hungarian, Slovak and
//! Norwegian). The strings written in this crate are translated here, from `locales/host.json`: a map
//! from the English text to the translation in each shipped language, keyed by language code. A text
//! with no entry (or a language with none) stays English, so a missing translation never breaks anything.
//!
//! The shipped languages are English, Spanish, German, French, Croatian, Romanian, Hungarian, Slovak,
//! Polish, Norwegian, Ukrainian, Russian and Italian. A system language outside that list uses English.

use lightcraft_ui_egui::i18n::{self, Locale};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The languages the interface is translated into, besides English, in the order of the language list.
pub const LANGUAGES: [Locale; 12] =
    [Locale::Es, Locale::De, Locale::Fr, Locale::Hr, Locale::Ro, Locale::Hu, Locale::Sk, Locale::Pl, Locale::No, Locale::Uk, Locale::Ru, Locale::It];

/// The translations of the strings of this crate: English text -> language code -> translation.
fn catalog() -> &'static HashMap<String, HashMap<String, String>> {
    static CATALOG: OnceLock<HashMap<String, HashMap<String, String>>> = OnceLock::new();
    CATALOG.get_or_init(|| match serde_json::from_str(include_str!("../locales/host.json")) {
        Ok(map) => map,
        Err(e) => {
            // a broken catalog degrades to English, it never takes the app down
            log::error!("host.json: {e}");
            HashMap::new()
        }
    })
}

/// `text` in the interface language: this crate's catalog first, then the upstream one, else `text` itself.
pub fn tr(text: &str) -> String {
    let lang = i18n::language();
    if lang != Locale::En
        && let Some(found) = catalog().get(text).and_then(|per_language| per_language.get(lang.code()))
    {
        return found.clone();
    }
    i18n::tr(text).to_string()
}

/// [`tr`] for a text with `{name}` placeholders, filled from `args`.
pub fn tf(text: &str, args: &[(&str, String)]) -> String {
    let mut out = tr(text);
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// The interface language for a system language tag (`hu-HU`, `nb_NO`, `en`...): one of the shipped
/// languages, English for anything else.
pub fn locale_for_system(tag: &str) -> Locale {
    match Locale::parse_tag(tag) {
        Some(l) if LANGUAGES.contains(&l) => l,
        _ => Locale::En,
    }
}
