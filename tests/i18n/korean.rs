// use gettextrs::{LocaleCategory, bindtextdomain, gettext, setlocale, textdomain};
// use xpodcasts::config;

// #[test]
// fn korean_translation_loads() {
//     let localedir = concat!(env!("CARGO_MANIFEST_DIR"), "/po/build-locale");

//     let locale_set = setlocale(LocaleCategory::LcAll, "ko_KR.UTF-8");
//     assert!(
//         locale_set.is_some(),
//         "ko_KR.UTF-8 not installed on this system"
//     );

//     bindtextdomain(config::GETTEXT_PACKAGE, localedir).expect("bindtextdomain failed");
//     textdomain(config::GETTEXT_PACKAGE).expect("textdomain failed");

//     let translated = gettext("_Subscribe");

//     assert_eq!(translated, "구독(_S)", "Korean translation did not apply");
// }
