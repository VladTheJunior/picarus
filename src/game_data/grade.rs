use gpui_kit::{Hsla, SharedString};
use strum::EnumIter;
use tracing::warn;

use crate::{
    colors::{BLUE, ORANGE, PURPLE, RED, YELLOW},
    language::t,
};

#[derive(Debug, EnumIter, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Grade {
    Common,
    Elite,
    Heroic,
    Legendary,
    LegendaryPlus,
    Unique,
    Mythical,
    Unknown(u8),
}

impl Default for Grade {
    fn default() -> Self {
        Self::Unknown(111)
    }
}

impl From<u8> for Grade {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Common,
            2 => Self::Elite,
            3 => Self::Heroic,
            4 => Self::Legendary,
            5 => Self::LegendaryPlus,
            6 => Self::Unique,
            7 => Self::Mythical,
            unk => {
                warn!("Cannot convert {} grade", unk);
                Self::Unknown(unk)
            }
        }
    }
}

impl From<SharedString> for Grade {
    fn from(value: SharedString) -> Self {
        match value.as_str() {
            "no" => Self::Common,
            "el" => Self::Elite,
            "he" => Self::Heroic,
            "ld" => Self::Legendary,
            "mt" => Self::LegendaryPlus,
            // 6 => Self::Unique,
            // 7 => Self::Mythical,
            unk => {
                warn!("Cannot convert {} grade", unk);
                Self::Unknown(99)
            }
        }
    }
}

impl Grade {
    pub fn locale(&self) -> SharedString {
        match self {
            Grade::Common => t("item-common-grade"),
            Grade::Elite => t("item-elite-grade"),
            Grade::Heroic => t("item-heroic-grade"),
            Grade::Legendary => t("item-legendary-grade"),
            Grade::LegendaryPlus => t("item-legendary-plus-grade"),
            Grade::Unique => t("item-unique-grade"),
            Grade::Mythical => t("item-mythical-grade"),
            Grade::Unknown(_) => t("item-unknown-grade"),
        }
    }

    pub fn locale_fishing(&self) -> SharedString {
        match self {
            Grade::Common => t("item-common-fishing-grade"),
            Grade::Elite => t("item-rare-fishing-grade"),
            Grade::Heroic => t("item-very-rare-fishing-grade"),
            _ => t("item-unknown-grade"),
        }
    }

    pub fn color(&self) -> Option<Hsla> {
        match self {
            Grade::Common => None,
            Grade::Elite => Some(BLUE),
            Grade::Heroic => Some(YELLOW),
            Grade::Legendary | Grade::LegendaryPlus => Some(PURPLE),
            Grade::Unique => Some(ORANGE),
            Grade::Mythical => Some(RED),
            Grade::Unknown(_) => None,
        }
    }
}
