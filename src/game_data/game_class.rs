use gpui::SharedString;
use strum::EnumIter;

use crate::language::t;

#[derive(EnumIter, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum GameClass {
    Assassin,
    Berserker,
    Guardian,
    Magician,
    Priest,
    Ranger,
    Trickster,
    Wizard,
}

impl GameClass {
    pub fn locale(&self) -> SharedString {
        match self {
            GameClass::Assassin => t("item-class-assassin"),
            GameClass::Berserker => t("item-class-berserker"),
            GameClass::Guardian => t("item-class-guardian"),
            GameClass::Magician => t("item-class-magician"),
            GameClass::Priest => t("item-class-priest"),
            GameClass::Ranger => t("item-class-ranger"),
            GameClass::Trickster => t("item-class-trickster"),
            GameClass::Wizard => t("item-class-wizard"),
        }
    }
}

impl TryFrom<&str> for GameClass {
    type Error = String;

    fn try_from(other: &str) -> Result<Self, Self::Error> {
        match other {
            "GD" => Ok(Self::Guardian),
            "MG" => Ok(Self::Magician),
            "WZ" => Ok(Self::Wizard),
            "TF" => Ok(Self::Assassin),
            "WR" => Ok(Self::Berserker),
            "PR" => Ok(Self::Priest),
            "AC" => Ok(Self::Ranger),
            "DO" => Ok(Self::Trickster),
            unk => Err(format!("Cannot convert {} class", unk)),
        }
    }
}
