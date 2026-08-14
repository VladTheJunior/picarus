pub mod config;

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use strum::{EnumIter, FromRepr};
use tracing::info;

use crate::{extensions::EnumNameExt, settings::config::Config};

#[derive(Debug, Default, EnumIter, Copy, Clone, PartialEq, Eq, Hash, FromRepr, Serialize, Deserialize)]
#[repr(usize)]
pub enum Language {
    #[default]
    English,
    Russian,
}

impl From<Language> for usize {
    fn from(value: Language) -> Self {
        value as usize
    }
}

impl From<String> for Language {
    fn from(value: String) -> Self {
        match value.as_str() {
            "en-US" => Self::English,
            "ru-RU" => Self::Russian,
            _ => Self::default(),
        }
    }
}

impl EnumNameExt for Language {
    fn title(&self) -> SharedString {
        match self {
            Language::English => "English".into(),
            Language::Russian => "Русский".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub language: Language,
    pub game_path: String
}

impl Default for Settings {
    fn default() -> Self {
        let locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));
        info!(locale, "Detected language:");
        Self {
            language: locale.into(),
            game_path: Default::default(),
        }
    }
}

impl Config for Settings {
    const FILE: &'static str = "settings.toml";
}
