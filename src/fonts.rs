//! CJK text, on a toolkit that bundles none.
//!
//! egui ships no CJK face and no single Windows font covers Simplified,
//! Traditional, Japanese and Korean at once. Every script is loaded regardless
//! of the active language, because the language picker shows all of the endonyms
//! at once -- on an en-US machine, loading only the active language's face turns
//! 简体中文 into tofu in its own menu entry.
//!
//! What the active language changes is the ORDER. Chinese, Japanese and Korean
//! share Han code points and draw them differently, so whichever face comes
//! first decides which regional glyph shape a reader gets.
//!
//! Lifted from Euterpe.Doctor, which solved this first.

use std::path::Path;
use std::sync::Arc;

use eframe::egui;

use crate::i18n::Lang;

pub fn install(ctx: &egui::Context, lang: Lang) {
    let Some(windir) = std::env::var_os("WINDIR") else {
        return;
    };
    let dir = Path::new(&windir).join("Fonts");

    let mut fonts = egui::FontDefinitions::default();
    let mut chain = Vec::new();

    for script in script_order(lang).iter().chain(std::iter::once(&LATIN)) {
        // Within a script the list is an availability fallback, not a coverage
        // chain: whichever face this Windows build ships is the one to use.
        for file in *script {
            if load(&dir, file, &mut fonts, &mut chain) {
                break;
            }
        }
    }

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .extend(chain.iter().cloned());
    }
    ctx.set_fonts(fonts);
}

type Script = &'static [&'static str];

const SIMPLIFIED: Script = &["msyh.ttc", "simhei.ttf", "simsun.ttc"];
const TRADITIONAL: Script = &["msjh.ttc", "mingliu.ttc"];
const JAPANESE: Script = &["YuGothM.ttc", "meiryo.ttc", "msgothic.ttc"];
const KOREAN: Script = &["malgun.ttf", "gulim.ttc"];
/// Latin and Cyrillic both. Segoe UI covers Russian, so the eleven European
/// locales and English need nothing further.
const LATIN: Script = &["segoeui.ttf"];

fn script_order(lang: Lang) -> [Script; 4] {
    match lang {
        Lang::ZhHant => [TRADITIONAL, SIMPLIFIED, JAPANESE, KOREAN],
        Lang::Ja => [JAPANESE, SIMPLIFIED, TRADITIONAL, KOREAN],
        Lang::Ko => [KOREAN, SIMPLIFIED, TRADITIONAL, JAPANESE],
        _ => [SIMPLIFIED, TRADITIONAL, JAPANESE, KOREAN],
    }
}

fn load(
    dir: &Path,
    file: &str,
    fonts: &mut egui::FontDefinitions,
    chain: &mut Vec<String>,
) -> bool {
    let Ok(bytes) = std::fs::read(dir.join(file)) else {
        return false;
    };
    fonts
        .font_data
        .insert(file.to_owned(), Arc::new(egui::FontData::from_owned(bytes)));
    chain.push(file.to_owned());
    true
}
