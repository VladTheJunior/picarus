use std::{
    collections::HashMap,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use fluent::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use gpui_kit::SharedString;

use unic_langid::langid;

use crate::settings::Language;
static LANGUAGE: AtomicUsize = AtomicUsize::new(0);

static LANGUAGE_MANAGER: OnceLock<LanguageController> = OnceLock::new();

pub fn t(key: &str) -> SharedString {
    let fallback = SharedString::new(key);
    LANGUAGE_MANAGER.get().and_then(|l| l.value(key, None)).unwrap_or(fallback)
}

pub fn t_v<'args, V>(key: &str, kwargs: Vec<(&'static str, V)>) -> SharedString
where
    V: Into<FluentValue<'args>>,
{
    let fallback = SharedString::new(key);
    let mut args = FluentArgs::new();
    for (k, v) in kwargs.into_iter() {
        args.set(k, v);
    }

    LANGUAGE_MANAGER.get().and_then(|l| l.value(key, Some(&args))).unwrap_or(fallback)
}

#[derive(Default)]
pub struct LanguageController {
    bundles: HashMap<Language, FluentBundle<FluentResource>>,
}

impl LanguageController {
    fn add(mut self, language: Language, bundle: FluentBundle<FluentResource>) -> LanguageController {
        self.bundles.insert(language, bundle);
        self
    }

    pub fn get_current_language() -> Language {
        Language::from_repr(LANGUAGE.load(Ordering::Relaxed)).unwrap_or_default()
    }

    fn value(&self, id: &str, args: Option<&FluentArgs>) -> Option<SharedString> {
        let bundle = &self.bundles[&Self::get_current_language()];

        let msg = bundle.get_message(id)?;
        let mut errors = vec![];
        let pattern = msg.value()?;
        Some(SharedString::new(bundle.format_pattern(&pattern, args, &mut errors)))
    }

    pub fn init() {
        let en_res = FluentResource::try_new(include_str!("../locales/en.ftl").to_string()).expect("Failed to parse an FTL string.");
        let mut en_bundle = FluentBundle::new_concurrent(vec![langid!("en-US")]);
        en_bundle.add_resource(en_res).expect("Failed to add FTL resources to the bundle.");
        en_bundle.add_builtins().expect("Failed to add builtins to the bundle.");

        let ru_res = FluentResource::try_new(include_str!("../locales/ru.ftl").to_string()).expect("Failed to parse an FTL string.");
        let mut ru_bundle = FluentBundle::new_concurrent(vec![langid!("ru-RU")]);
        ru_bundle.add_resource(ru_res).expect("Failed to add FTL resources to the bundle.");
        ru_bundle.add_builtins().expect("Failed to add builtins to the bundle.");

        LANGUAGE_MANAGER.get_or_init(|| {
            LanguageController::default()
                .add(Language::English, en_bundle)
                .add(Language::Russian, ru_bundle)
        });
    }

    pub fn switch(language: Language) {
        LANGUAGE.store(language.into(), Ordering::Release);
    }
}

pub fn format_duration(seconds: i32) -> SharedString {
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    let remaining_seconds = seconds % 60;
    let remaining_minutes = minutes % 60;
    let remaining_hours = hours % 24;

    let mut parts = Vec::new();

    if days > 0 {
        parts.push(t_v("duration-days", vec![("value", days)]));
    }
    if remaining_hours > 0 {
        parts.push(t_v("duration-hours", vec![("value", remaining_hours)]));
    }
    if remaining_minutes > 0 {
        parts.push(t_v("duration-minutes", vec![("value", remaining_minutes)]));
    }
    if remaining_seconds > 0 || parts.is_empty() {
        parts.push(t_v("duration-seconds", vec![("value", remaining_seconds)]));
    }

    SharedString::new(parts.join(" "))
}
