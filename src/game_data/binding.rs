use gpui::SharedString;

use crate::language::t;

#[derive(Clone, Copy, Debug)]
pub enum Binding {
    None,
    Obtain,
    Equip,
}

impl TryFrom<&str> for Binding {
    type Error = String;

    fn try_from(other: &str) -> Result<Self, Self::Error> {
        match other {
            "get" => Ok(Self::Obtain),
            "equip" => Ok(Self::Equip),
            "none" => Ok(Self::None),
            unk => Err(format!("Cannot convert {} binding", unk)),
        }
    }
}

impl Binding {
    pub fn locale(&self) -> Option<SharedString> {
        match self {
            Binding::Obtain => Some(t("item-binding-obtain")),
            Binding::Equip => Some(t("item-binding-equip")),
            Binding::None => None,
        }
    }
}
