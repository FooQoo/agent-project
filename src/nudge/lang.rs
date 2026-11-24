//! The language of the sentences the nudge rules write. It follows the
//! project's locale (the `locale` of the progress table): `ja` gives Japanese,
//! anything else, and no locale, gives English. Rule ids, kinds, commands,
//! options, IDs, and JSON keys are never translated.

/// A language the hook's sentences are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Ja,
}

impl Lang {
    /// The language for a saved locale; English when there is none.
    pub fn from_locale(locale: Option<&str>) -> Lang {
        match locale {
            Some("ja") => Lang::Ja,
            _ => Lang::En,
        }
    }

    /// `en` or `ja`, by language.
    pub fn pick<'a>(self, en: &'a str, ja: &'a str) -> &'a str {
        match self {
            Lang::En => en,
            Lang::Ja => ja,
        }
    }
}
