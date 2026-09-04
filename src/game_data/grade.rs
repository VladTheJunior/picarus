use gpui_kit::{Hsla, SharedString, hsla};
use strum::EnumIter;
use tracing::warn;

use crate::language::t;

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

    pub fn color(&self) -> Option<Hsla> {
        match self {
            Grade::Common => None,
            Grade::Elite => Some(hsla(210.0 / 360.0, 0.55, 0.67, 1.0)),
            Grade::Heroic => Some(hsla(25.0 / 360.0, 0.55, 0.67, 1.0)),
            Grade::Legendary | Grade::LegendaryPlus => Some(hsla(270.0 / 360.0, 0.55, 0.67, 1.0)),
            Grade::Unique => Some(hsla(8.0 / 360.0, 0.55, 0.67, 1.0)),
            Grade::Mythical => Some(hsla(8.0 / 360.0, 0.55, 0.45, 1.0)),
            Grade::Unknown(_) => None,
        }
    }
}
