//! Every user-visible string, with all of its translations kept together.
//!
//! [`Text`] is a plain struct rather than a key-value map, the same choice
//! Euterpe.Doctor made and for the same reason: a struct literal must name every
//! field, so a missing translation is a compile error rather than a runtime
//! fallback to English that nobody notices.
//!
//! # The enum only lists languages that are actually finished
//!
//! Fourteen, the same set as Euterpe.App and Euterpe.Doctor, so terminology
//! cannot drift between products a charter uses in the same afternoon -- and so
//! that somebody running the App in Korean is not handed an English Press.
//!
//! Adding a locale means adding a field, which makes the compiler name every
//! string that does not have it yet and keeps the build red until they all do.
//! A locale is therefore complete or absent, and there is no state in which a
//! charter reads half a translated interface.
//!
//! # Press says what it will DO, not what the charter must do
//!
//! The web's `errors:codes.VIDEO_*` strings are worded for a page that can only
//! refuse: "re-export it without audio (ffmpeg -an)". Press *is* the thing that
//! does that, so its wording is what it found and what it is about to fix.
//! Reusing those sentences verbatim would tell a charter to go and do by hand
//! the exact job they just opened a tool for.
//!
//! The FACTS still match, because they come from the same gate -- the same
//! limits, the same numbers, in the same order.

pub mod text;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    ZhHans,
    ZhHant,
    Ja,
    Ko,
    De,
    Es,
    Fr,
    Hr,
    Hu,
    Id,
    Nl,
    Pt,
    Ru,
}

impl Lang {
    pub const ALL: [Self; 14] = [
        Self::En,
        Self::ZhHans,
        Self::ZhHant,
        Self::Ja,
        Self::Ko,
        Self::De,
        Self::Es,
        Self::Fr,
        Self::Hr,
        Self::Hu,
        Self::Id,
        Self::Nl,
        Self::Pt,
        Self::Ru,
    ];

    /// The OS language, falling back to English for anything not carried.
    pub fn from_system() -> Self {
        let Some(tag) = system_locale() else {
            return Self::En;
        };
        let tag = tag.to_ascii_lowercase();

        // Traditional Chinese is spelled several ways across Windows versions;
        // any Chinese tag that is not one of them is Simplified.
        if tag.starts_with("zh") {
            let traditional = ["hant", "tw", "hk", "mo"];
            return if traditional.iter().any(|marker| tag.contains(marker)) {
                Self::ZhHant
            } else {
                Self::ZhHans
            };
        }

        match tag.split(['-', '_']).next().unwrap_or_default() {
            "ja" => Self::Ja,
            "ko" => Self::Ko,
            "de" => Self::De,
            "es" => Self::Es,
            "fr" => Self::Fr,
            "hr" => Self::Hr,
            "hu" => Self::Hu,
            // Windows still reports the retired ISO code for Indonesian.
            "id" | "in" => Self::Id,
            "nl" => Self::Nl,
            "pt" => Self::Pt,
            "ru" => Self::Ru,
            _ => Self::En,
        }
    }

    pub const fn endonym(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::ZhHans => "简体中文",
            Self::ZhHant => "繁體中文",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
            Self::De => "Deutsch",
            Self::Es => "Español",
            Self::Fr => "Français",
            Self::Hr => "Hrvatski",
            Self::Hu => "Magyar",
            Self::Id => "Bahasa Indonesia",
            Self::Nl => "Nederlands",
            Self::Pt => "Português",
            Self::Ru => "Русский",
        }
    }
}

#[cfg(windows)]
fn system_locale() -> Option<String> {
    // LOCALE_NAME_MAX_LENGTH; the returned length counts the null terminator.
    let mut buffer = [0u16; 85];
    let written = unsafe {
        windows_sys::Win32::Globalization::GetUserDefaultLocaleName(buffer.as_mut_ptr(), 85)
    };
    let len = usize::try_from(written).ok()?.checked_sub(1)?;
    Some(String::from_utf16_lossy(&buffer[..len]))
}

#[cfg(not(windows))]
fn system_locale() -> Option<String> {
    std::env::var("LANG").ok()
}

/// One string in every language Press carries.
pub struct Text {
    pub en: &'static str,
    pub zh_hans: &'static str,
    pub zh_hant: &'static str,
    pub ja: &'static str,
    pub ko: &'static str,
    pub de: &'static str,
    pub es: &'static str,
    pub fr: &'static str,
    pub hr: &'static str,
    pub hu: &'static str,
    pub id: &'static str,
    pub nl: &'static str,
    pub pt: &'static str,
    pub ru: &'static str,
}

impl Text {
    pub const fn get(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => self.en,
            Lang::ZhHans => self.zh_hans,
            Lang::ZhHant => self.zh_hant,
            Lang::Ja => self.ja,
            Lang::Ko => self.ko,
            Lang::De => self.de,
            Lang::Es => self.es,
            Lang::Fr => self.fr,
            Lang::Hr => self.hr,
            Lang::Hu => self.hu,
            Lang::Id => self.id,
            Lang::Nl => self.nl,
            Lang::Pt => self.pt,
            Lang::Ru => self.ru,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A field left empty is a translation that was skipped rather than written,
    /// and the struct's whole purpose is that skipping is impossible. The
    /// compiler catches a MISSING field; this catches a present but blank one.
    #[test]
    fn no_string_is_blank_in_any_language() {
        for (name, text) in text::EVERY_STRING {
            for lang in Lang::ALL {
                assert!(
                    !text.get(lang).trim().is_empty(),
                    "{name} has nothing for {}",
                    lang.endonym()
                );
            }
        }
    }

    /// A placeholder that exists in one language and not another renders as
    /// literal braces for whoever reads that language -- the kind of bug that
    /// only ever shows up in the locale nobody on the team reads.
    #[test]
    fn every_language_interpolates_the_same_placeholders() {
        for (name, text) in text::EVERY_STRING {
            let expected = placeholders(text.en);
            for lang in Lang::ALL {
                assert_eq!(
                    placeholders(text.get(lang)),
                    expected,
                    "{name}: {} has different placeholders from English",
                    lang.endonym()
                );
            }
        }
    }

    fn placeholders(s: &str) -> Vec<String> {
        let mut found: Vec<String> = s
            .split('{')
            .skip(1)
            .filter_map(|rest| rest.split_once('}').map(|(name, _)| name.to_owned()))
            .collect();
        found.sort();
        found
    }
}
