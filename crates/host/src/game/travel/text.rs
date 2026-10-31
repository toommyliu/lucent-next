use icu_collator::{
    Collator,
    options::{CollatorOptions, Strength},
};
use icu_locale_core::locale;

pub(super) fn same_text(left: &str, right: &str) -> bool {
    let mut options = CollatorOptions::default();
    options.strength = Some(Strength::Secondary);
    let collator =
        Collator::try_new(locale!("en").into(), options).expect("compiled collation data");
    collator.compare(left, right).is_eq()
}

pub(super) fn trim(text: &str) -> &str {
    text.trim_matches(|ch| matches!(ch, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'))
}
