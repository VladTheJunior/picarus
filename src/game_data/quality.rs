use gpui::SharedString;

use crate::language::t;

#[derive(Default, Clone, Copy)]
pub enum Quality {
    #[default]
    Simple,
    Good,
    Perfect,
}

impl Quality {
    pub fn locale(&self) -> SharedString {
        match self {
            Quality::Simple => t("item-quality-simple"),
            Quality::Good => t("item-quality-good"),
            Quality::Perfect => t("item-quality-perfect"),
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Quality::Simple => Quality::Good,
            Quality::Good => Quality::Perfect,
            Quality::Perfect => Quality::Simple,
        }
    }
}
