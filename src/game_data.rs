pub mod accessory;
pub mod armor;
pub mod filters;
pub mod item_option;
pub mod item_quality;
pub mod item_res;
pub mod item_set;
pub mod locale;
pub mod secondary_weapon;
pub mod tempering;
pub mod weapon;

use anyhow::Result;

use encoding_rs::EUC_KR;
use gpui::{AsyncWindowContext, Entity, Hsla, Image, SharedString, hsla};

use image::{ImageReader, imageops::FilterType};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs::File,
    io::{Cursor, Read, Seek},
    path::Path,
    rc::Rc,
    sync::Arc,
};
use strum::{EnumIter, FromRepr, IntoEnumIterator};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt, BufReader};
use tracing::{debug, error, warn};
use zip::ZipArchive;

use crate::{
    game_data::{
        accessory::Accessory, armor::Armor, item_option::ItemOption, item_quality::ItemQuality, item_res::ItemRes,
        item_set::ItemSet, locale::Locale, secondary_weapon::SecondaryWeapon, tempering::Tempering, weapon::Weapon,
    },
    game_data_view::GameDataLoadingStatus,
    language::{LanguageController, t, t_v},
};

#[derive(Debug, EnumIter, Eq, PartialEq, Hash, Clone, Copy)]
pub enum ItemType {
    Armor,
    SecondaryWeapon,
    Weapon,
    Accessory,
}

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

impl ItemType {
    pub fn locale(&self) -> SharedString {
        match self {
            ItemType::Armor => t("item-type-armor"),
            ItemType::SecondaryWeapon => t("item-type-secondary-weapon"),
            ItemType::Weapon => t("item-type-weapon"),
            ItemType::Accessory => t("item-type-accessory"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TagType {
    String,
    Float,
}
#[derive(Debug, Copy, Clone, Default)]
pub enum DataFormat {
    #[default]
    String,
    WideString,
}
#[derive(Debug, Serialize)]
pub enum DataType {
    SecondaryWeapon(SecondaryWeapon),
    Weapon(Weapon),
    Armor(Armor),
    Accessory(Accessory),
}
#[derive(Debug, EnumIter, Copy, Clone, PartialEq, Eq, Hash, FromRepr, Serialize)]
#[repr(u8)]
pub enum Grade {
    Common = 1,
    Elite = 2,
    Heroic = 3,
    Legendary = 4,
    Unique = 6,
    Mythical = 7,
}

#[derive(Debug, EnumIter, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
pub enum _ArmorKind {
    HeavyArmor(ArmorTypes),
    LightArmorMagic(ArmorTypes),
    LightArmorPhysical(ArmorTypes),
    RobeArmor(ArmorTypes),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
pub enum ArmorClassKind {
    Magic(ArmorTypes),
    Physical(ArmorTypes),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
pub enum ArmorTypes {
    Helmet,
    Pauldron,
    Armor,
    Gloves,
    Boots,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
pub enum ItemSubType {
    Necklage,
    Ring,
    Armor(ArmorClassKind),
    Dagger,
    Sword,
    Greatsword,
    Scepter,
    Staff,
    Lance,
    Crossbow,
    Bow,
    Wand,
    Shield,
    Crest,
    Vambrace,
    TeddyBear,
}

impl TryFrom<&str> for ItemSubType {
    type Error = String;

    fn try_from(other: &str) -> Result<Self, Self::Error> {
        match other {
            "ne" => Ok(Self::Necklage),
            "ri" => Ok(Self::Ring),
            "pl_ha" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Helmet))),
            "pl_sh" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Pauldron))),
            "pl_ja" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Armor))),
            "pl_gl" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Gloves))),
            "pl_bo" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Boots))),

            "le_ha" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Helmet))),
            "le_sh" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Pauldron))),
            "le_ja" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Armor))),
            "le_gl" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Gloves))),
            "le_bo" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Boots))),

            "ch_ha" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Helmet))),
            "ch_sh" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Pauldron))),
            "ch_ja" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Armor))),
            "ch_gl" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Gloves))),
            "ch_bo" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Boots))),

            "cl_ha" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Helmet))),
            "cl_sh" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Pauldron))),
            "cl_ja" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Armor))),
            "cl_gl" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Gloves))),
            "cl_bo" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Boots))),

            "d1" => Ok(Self::Dagger),
            "s1" => Ok(Self::Sword),
            "s2" => Ok(Self::Greatsword),
            "m1" => Ok(Self::Scepter),
            "m2" => Ok(Self::Staff),
            "l2" => Ok(Self::Lance),
            "c2" => Ok(Self::Crossbow),
            "b1" => Ok(Self::Bow),
            "w1" => Ok(Self::Wand),
            "sd" => Ok(Self::Shield),
            "at" => Ok(Self::Crest),
            "ga" => Ok(Self::Vambrace),
            "tb" => Ok(Self::TeddyBear),
            unk => Err(format!("Cannot convert {} item subtype", unk)),
        }
    }
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

    pub fn check_item_option(option: &ItemOption, usable_class: &BTreeSet<GameClass>, item_sub_type: &str) -> Option<Vec<ItemMinMaxEffect>> {
        let item_sub_type = ItemSubType::try_from(item_sub_type).ok()?;
        let mut effects = vec![];
        let classes: Vec<&GameClass> = usable_class.iter().collect();
        let oe = match classes.as_slice() {
            [GameClass::Berserker] => Some((option.wr_effect1.clone(), option.wr_effect2.clone())),
            [GameClass::Guardian] => Some((option.gd_effect1.clone(), option.gd_effect2.clone())),
            [GameClass::Wizard, GameClass::Magician] | [GameClass::Magician, GameClass::Wizard] => {
                Some((option.wz_effect1.clone(), option.wz_effect2.clone()))
            }
            [GameClass::Trickster] => Some((option.do_effect1.clone(), option.do_effect2.clone())),
            [GameClass::Assassin] => Some((option.tf_effect1.clone(), option.tf_effect2.clone())),
            [GameClass::Priest] => Some((option.pr_effect1.clone(), option.pr_effect2.clone())),
            [GameClass::Ranger] => Some((option.ac_effect1.clone(), option.ac_effect2.clone())),
            _ => None,
        };

        match item_sub_type {
            // +
            ItemSubType::Necklage => {
                effects.push(option.effect2.get(0).cloned());
                effects.push(option.effect2.get(1).cloned());
                effects.push(option.effect2.get(9).cloned());
                effects.push(option.effect1.get(0).cloned());
                effects.push(option.effect1.get(4).cloned());
                effects.push(option.effect2.get(4).cloned());
                effects.push(option.effect2.get(10).cloned());
                effects.push(option.effect2.get(3).cloned());
                effects.push(option.effect2.get(2).cloned());
                effects.push(option.effect2.get(5).cloned());
            }
            // +
            ItemSubType::Ring => {
                effects.push(option.effect1.get(10).cloned());
                effects.push(option.effect1.get(1).cloned());
                effects.push(option.effect1.get(3).cloned());
                effects.push(option.effect1.get(7).cloned());
                effects.push(option.effect1.get(11).cloned());
                effects.push(option.effect2.get(6).cloned());
                effects.push(option.effect2.get(8).cloned());
                effects.push(option.effect1.get(9).cloned());
                effects.push(option.effect2.get(7).cloned());
                effects.push(option.effect1.get(8).cloned());
            }
            // +
            ItemSubType::Dagger => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(0).cloned());
                effects.push(oe1.get(1).cloned());
                effects.push(oe1.get(2).cloned());
                effects.push(oe2.get(0).cloned());
                effects.push(oe2.get(1).cloned());
                effects.push(oe1.get(5).cloned());
            }
            // +
            ItemSubType::Sword | ItemSubType::Greatsword => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(0).cloned());
                effects.push(oe1.get(2).cloned());
                effects.push(oe2.get(0).cloned());
                effects.push(oe2.get(1).cloned());
                effects.push(oe1.get(5).cloned());
            }
            // +
            ItemSubType::Lance => {
                effects.push(option.effect1.get(0).cloned());
                effects.push(option.effect1.get(1).cloned());
                effects.push(option.effect1.get(2).cloned());
                effects.push(option.effect1.get(3).cloned());
                effects.push(option.effect1.get(4).cloned());
                effects.push(option.effect2.get(0).cloned());
                effects.push(option.effect2.get(1).cloned());
                effects.push(option.effect1.get(5).cloned());
                effects.push(option.effect2.get(10).cloned());
                effects.push(option.effect1.get(6).cloned());
                effects.push(option.effect2.get(2).cloned());
                effects.push(option.effect2.get(3).cloned());
            }
            // +
            ItemSubType::Crossbow => {
                effects.push(option.effect1.get(0).cloned());
                effects.push(option.effect1.get(1).cloned());
                effects.push(option.effect1.get(2).cloned());
                effects.push(option.effect1.get(3).cloned());
                effects.push(option.effect1.get(4).cloned());
                effects.push(option.effect2.get(0).cloned());
                effects.push(option.effect2.get(1).cloned());
                effects.push(option.effect1.get(5).cloned());
                effects.push(option.effect1.get(11).cloned());
                effects.push(option.effect1.get(6).cloned());
                effects.push(option.effect2.get(2).cloned());
                effects.push(option.effect2.get(3).cloned());
            }
            // +
            ItemSubType::Scepter | ItemSubType::Bow | ItemSubType::Staff | ItemSubType::Wand => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(2).cloned());
                effects.push(oe1.get(3).cloned());
                effects.push(oe1.get(4).cloned());
                effects.push(oe2.get(2).cloned());
                effects.push(oe2.get(3).cloned());
                effects.push(oe1.get(6).cloned());
            }
            // +
            ItemSubType::Shield => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(0).cloned());
                effects.push(oe1.get(2).cloned());
                effects.push(oe1.get(9).cloned());
                effects.push(oe2.get(6).cloned());
                effects.push(oe2.get(9).cloned());
                effects.push(oe1.get(8).cloned());
            }
            // +
            ItemSubType::Vambrace => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(2).cloned());
                effects.push(oe1.get(3).cloned());
                effects.push(oe1.get(4).cloned());
                effects.push(oe1.get(8).cloned());
                effects.push(oe2.get(5).cloned());
                effects.push(oe2.get(9).cloned());
                effects.push(oe1.get(10).cloned());
            }
            // +
            ItemSubType::TeddyBear | ItemSubType::Crest => {
                let (oe1, oe2) = oe?;
                effects.push(oe1.get(2).cloned());
                effects.push(oe1.get(3).cloned());
                effects.push(oe1.get(4).cloned());
                effects.push(oe1.get(9).cloned());
                effects.push(oe2.get(5).cloned());
                effects.push(oe2.get(8).cloned());
            }
            ItemSubType::Armor(armor_kind) => match armor_kind {
                ArmorClassKind::Physical(armor_types) => match armor_types {
                    ArmorTypes::Helmet => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(9).cloned());
                        effects.push(oe1.get(9).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                    ArmorTypes::Pauldron => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(10).cloned());
                        effects.push(oe1.get(11).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                    ArmorTypes::Armor => {
                        let (oe1, _) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe1.get(9).cloned());
                        effects.push(oe1.get(10).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                    ArmorTypes::Gloves => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(1).cloned());
                        effects.push(oe2.get(0).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                    ArmorTypes::Boots => {
                        let (oe1, _) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe1.get(10).cloned());
                        effects.push(oe1.get(8).cloned());
                    }
                },
                ArmorClassKind::Magic(armor_types) => match armor_types {
                    ArmorTypes::Helmet => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(9).cloned());
                        effects.push(oe1.get(8).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                    ArmorTypes::Pauldron => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(10).cloned());
                        effects.push(oe1.get(11).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                    ArmorTypes::Armor => {
                        let (oe1, _) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe1.get(8).cloned());
                        effects.push(oe1.get(9).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                    ArmorTypes::Gloves => {
                        let (oe1, oe2) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe2.get(3).cloned());
                        effects.push(oe2.get(2).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                    ArmorTypes::Boots => {
                        let (oe1, _) = oe?;
                        effects.push(oe1.get(0).cloned());
                        effects.push(oe1.get(1).cloned());
                        effects.push(oe1.get(2).cloned());
                        effects.push(oe1.get(3).cloned());
                        effects.push(oe1.get(4).cloned());
                        effects.push(oe1.get(7).cloned());
                        effects.push(oe1.get(9).cloned());
                        effects.push(oe1.get(10).cloned());
                    }
                },
            },
        };

        let collection: Vec<_> = effects.into_iter().filter_map(|f| f).collect();
        (!collection.is_empty()).then_some(collection)
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

impl Grade {
    pub fn locale(&self) -> SharedString {
        match self {
            Grade::Common => t("item-common-grade"),
            Grade::Elite => t("item-elite-grade"),
            Grade::Heroic => t("item-heroic-grade"),
            Grade::Legendary => t("item-legendary-grade"),
            Grade::Unique => t("item-unique-grade"),
            Grade::Mythical => t("item-mythical-grade"),
        }
    }

    pub fn color(&self) -> Option<Hsla> {
        match self {
            Grade::Common => None,
            Grade::Elite => Some(hsla(210.0 / 360.0, 0.55, 0.67, 1.0)),
            Grade::Heroic => Some(hsla(25.0 / 360.0, 0.55, 0.67, 1.0)),
            Grade::Legendary => Some(hsla(270.0 / 360.0, 0.55, 0.67, 1.0)),
            Grade::Unique => Some(hsla(8.0 / 360.0, 0.55, 0.67, 1.0)),
            Grade::Mythical => Some(hsla(8.0 / 360.0, 0.55, 0.45, 1.0)),
        }
    }
}

impl DataType {
    pub fn test(&self) {
        let id = self.get_id();
        let name = self.get_locale_name();
        let mut effects = Vec::new();
        match self {
            DataType::Weapon(weapon) => {
                [
                    &weapon.equip_effect_1,
                    &weapon.equip_effect_2,
                    &weapon.equip_effect_3,
                    &weapon.equip_effect_4,
                ]
                .iter()
                .filter_map(|opt| opt.as_ref())
                .for_each(|effect| effects.push(effect));
                if let Some(set) = weapon.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
            DataType::Armor(armor) => {
                [&armor.equip_effect_1, &armor.equip_effect_2, &armor.equip_effect_3, &armor.equip_effect_4]
                    .iter()
                    .filter_map(|opt| opt.as_ref())
                    .for_each(|effect| effects.push(effect));
                if let Some(set) = armor.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
            DataType::Accessory(accessory) => {
                [
                    &accessory.equip_effect_1,
                    &accessory.equip_effect_2,
                    &accessory.equip_effect_3,
                    &accessory.equip_effect_4,
                ]
                .iter()
                .filter_map(|opt| opt.as_ref())
                .for_each(|effect| effects.push(effect));
                if let Some(set) = accessory.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
            DataType::SecondaryWeapon(secondary_weapon) => {
                [
                    &secondary_weapon.equip_effect_1,
                    &secondary_weapon.equip_effect_2,
                    &secondary_weapon.equip_effect_3,
                    &secondary_weapon.equip_effect_4,
                ]
                .iter()
                .filter_map(|opt| opt.as_ref())
                .for_each(|effect| effects.push(effect));
                if let Some(set) = secondary_weapon.item_set.as_ref() {
                    for e in &set.effects {
                        effects.extend(e.seteffect_effects.iter());
                    }
                }
            }
        };

        for e in effects.iter().filter(|f| f.parsed.is_none()) {
            warn!(?id, ?name, ?e.effect, "Failed to detect effect");
        }
    }

    pub fn get_full_type(&self) -> SharedString {
        match self {
            DataType::Weapon(weapon) => weapon.get_full_type(),
            DataType::Armor(armor) => armor.get_full_type(),
            DataType::Accessory(accessory) => accessory.get_full_type(),
            DataType::SecondaryWeapon(secondary_weapon) => secondary_weapon.get_full_type(),
        }
    }

    pub fn get_type(&self) -> SharedString {
        match self {
            DataType::Weapon(weapon) => weapon.get_type(),
            DataType::Armor(armor) => armor.get_type(),
            DataType::Accessory(accessory) => accessory.get_type(),
            DataType::SecondaryWeapon(secondary_weapon) => secondary_weapon.get_type(),
        }
    }

    pub fn get_id(&self) -> SharedString {
        match self {
            DataType::Weapon(weapon) => weapon.id.clone(),
            DataType::Armor(armor) => armor.id.clone(),
            DataType::Accessory(accessory) => accessory.id.clone(),
            DataType::SecondaryWeapon(secondary_weapon) => secondary_weapon.id.clone(),
        }
    }

    pub fn get_icon(&self) -> Option<Arc<Image>> {
        match self {
            DataType::Weapon(weapon) => weapon.icon.clone(),
            DataType::Armor(armor) => armor.icon.clone(),
            DataType::Accessory(accessory) => accessory.icon.clone(),
            DataType::SecondaryWeapon(secondary_weapon) => secondary_weapon.icon.clone(),
        }
    }

    fn matches(&self, input: &str, types: &HashSet<ItemType>, grades: &HashSet<Grade>, effect: &Option<SharedString>) -> bool {
        let include = match self {
            DataType::Weapon(_) => types.contains(&ItemType::Weapon),
            DataType::Armor(_) => types.contains(&ItemType::Armor),
            DataType::Accessory(_) => types.contains(&ItemType::Accessory),
            DataType::SecondaryWeapon(_) => types.contains(&ItemType::SecondaryWeapon),
        };

        if !include {
            return false;
        }

        if !self.filter_effect(effect) {
            return false;
        }

        let grade = self.get_grade();

        if grade.is_none_or(|g| !grades.contains(&g)) {
            return false;
        }

        if input.is_empty() {
            return true;
        }

        let id = self.get_id();
        let locale = self.get_locale();
        id.to_lowercase().contains(&input)
            || locale.as_ref().is_some_and(|l| l.rus.to_lowercase().contains(&input))
            || locale.as_ref().is_some_and(|l| l.eng.to_lowercase().contains(&input))
    }

    pub fn get_effects(&self) -> HashSet<SharedString> {
        let mut effects = HashSet::new();
        match self {
            DataType::Weapon(weapon) => {
                effects.insert(weapon.equip_effect_1.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                effects.insert(weapon.equip_effect_2.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                effects.insert(weapon.equip_effect_3.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                effects.insert(weapon.equip_effect_4.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                if let Some(set) = &weapon.item_set {
                    effects.extend(
                        set.effects
                            .iter()
                            .map(|f| f.seteffect_effects.iter())
                            .flatten()
                            .filter_map(|f| f.parsed.clone())
                            .map(|(key, _)| Some(key)),
                    );
                }
            }
            DataType::Armor(armor) => {
                effects.insert(armor.equip_effect_1.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                effects.insert(armor.equip_effect_2.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                effects.insert(armor.equip_effect_3.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                effects.insert(armor.equip_effect_4.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                if let Some(set) = &armor.item_set {
                    effects.extend(
                        set.effects
                            .iter()
                            .map(|f| f.seteffect_effects.iter())
                            .flatten()
                            .filter_map(|f| f.parsed.clone())
                            .map(|(key, _)| Some(key)),
                    );
                }
            }
            DataType::Accessory(accessory) => {
                effects.insert(accessory.equip_effect_1.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                effects.insert(accessory.equip_effect_2.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                effects.insert(accessory.equip_effect_3.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                effects.insert(accessory.equip_effect_4.as_ref().and_then(|f| f.parsed.clone()).map(|(key, _)| key));
                if let Some(set) = &accessory.item_set {
                    effects.extend(
                        set.effects
                            .iter()
                            .map(|f| f.seteffect_effects.iter())
                            .flatten()
                            .filter_map(|f| f.parsed.clone())
                            .map(|(key, _)| Some(key)),
                    );
                }
            }
            DataType::SecondaryWeapon(secondary_weapon) => {
                effects.insert(
                    secondary_weapon
                        .equip_effect_1
                        .as_ref()
                        .and_then(|f| f.parsed.clone())
                        .map(|(key, _)| key),
                );
                effects.insert(
                    secondary_weapon
                        .equip_effect_2
                        .as_ref()
                        .and_then(|f| f.parsed.clone())
                        .map(|(key, _)| key),
                );
                effects.insert(
                    secondary_weapon
                        .equip_effect_3
                        .as_ref()
                        .and_then(|f| f.parsed.clone())
                        .map(|(key, _)| key),
                );
                effects.insert(
                    secondary_weapon
                        .equip_effect_4
                        .as_ref()
                        .and_then(|f| f.parsed.clone())
                        .map(|(key, _)| key),
                );
                if let Some(set) = &secondary_weapon.item_set {
                    effects.extend(
                        set.effects
                            .iter()
                            .map(|f| f.seteffect_effects.iter())
                            .flatten()
                            .filter_map(|f| f.parsed.clone())
                            .map(|(key, _)| Some(key)),
                    );
                }
            }
        };
        effects.into_iter().filter_map(|f| f).collect()
    }

    pub fn filter_effect(&self, filter: &Option<SharedString>) -> bool {
        if let Some(filter) = filter {
            let effects = self.get_effects();
            return effects.contains(filter);
        }
        return true;
    }

    pub fn get_grade(&self) -> Option<Grade> {
        match self {
            DataType::Weapon(weapon) => weapon.grade,
            DataType::Armor(armor) => armor.grade,
            DataType::Accessory(accessory) => accessory.grade,
            DataType::SecondaryWeapon(secondary_weapon) => secondary_weapon.grade,
        }
    }

    pub fn get_locale(&self) -> Option<Locale> {
        match self {
            DataType::Weapon(weapon) => weapon.locale.clone(),
            DataType::Armor(armor) => armor.locale.clone(),
            DataType::Accessory(accessory) => accessory.locale.clone(),
            DataType::SecondaryWeapon(secondary_weapon) => secondary_weapon.locale.clone(),
        }
    }

    pub fn get_locale_name(&self) -> SharedString {
        let language = LanguageController::get_current_language();
        let locale = self.get_locale();

        locale
            .as_ref()
            .map(|f| match language {
                crate::settings::Language::English => f.eng.clone(),
                crate::settings::Language::Russian => f.rus.clone(),
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or_else(|| self.get_id())
    }
}

pub struct GameData {
    pub items: IndexMap<SharedString, Rc<DataType>>,
    pub effects_by_grade: HashMap<Grade, HashMap<u16, ItemOption>>,
    pub tempering_by_types: HashMap<SharedString, HashMap<u16, Tempering>>,
    pub quality_by_types: HashMap<SharedString, HashMap<u16, ItemQuality>>,
}

impl GameData {
    pub fn new() -> Self {
        Self {
            items: IndexMap::new(),
            tempering_by_types: HashMap::new(),
            effects_by_grade: HashMap::new(),
            quality_by_types: HashMap::new(),
        }
    }

    pub fn get_all_effects(&self) -> HashSet<SharedString> {
        self.items.iter().map(|(_, value)| value.get_effects()).flatten().collect()
    }

    pub async fn load(game_path: &str, on_load: &Entity<GameDataLoadingStatus>, cx: &mut AsyncWindowContext) -> Result<Self> {
        let gamedatas = File::open(Path::new(game_path).join(r"Game\gamedatas.npk"))?;
        let gamelibs = File::open(Path::new(game_path).join(r"Game\gamelibs.npk"))?;
        let mut gamedatas_zip = ZipArchive::new(gamedatas)?;
        let mut gamelibs_zip = ZipArchive::new(gamelibs)?;
        let mut data = Self::new();
        /**/
        let item_set = data.read_itemset(&mut gamedatas_zip, on_load, cx).await?;
        data.read_weapons(&mut gamedatas_zip, &mut gamelibs_zip, &item_set, on_load, cx).await?;
        data.read_accessory(&mut gamedatas_zip, &mut gamelibs_zip, &item_set, on_load, cx).await?;

        data.read_secondary_weapons(&mut gamedatas_zip, &mut gamelibs_zip, &item_set, on_load, cx)
            .await?;

        data.read_armors(&mut gamedatas_zip, &mut gamelibs_zip, &item_set, on_load, cx).await?;

        data.read_temperings(
            data.items.iter().map(|(_, item)| item.get_full_type()).unique().collect(),
            &mut gamedatas_zip,
            on_load,
            cx,
        )
        .await?;
        data.read_options(&mut gamedatas_zip, on_load, cx).await?;

        data.read_qualites(
            data.items.iter().map(|(_, item)| item.get_type()).unique().collect(),
            &mut gamedatas_zip,
            on_load,
            cx,
        )
        .await?;

        Ok(data)
    }
    async fn read_temperings<R: Read + Seek>(
        &mut self,
        item_types: Vec<SharedString>,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Tempering;
            cx.notify();
        });
        let mut tempering_by_types = HashMap::new();
        for item_type in &item_types {
            match self.read_tempering_by_type(item_type, gamedatas_zip).await {
                Ok(tempering) => {
                    tempering_by_types.insert(SharedString::new(item_type), tempering);
                }
                Err(e) => {
                    error!(?e, ?item_type);
                }
            };
        }
        debug!(tempering_keys = ?tempering_by_types.keys());
        self.tempering_by_types = tempering_by_types;
        Ok(())
    }

    async fn read_options<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Effects;
            cx.notify();
        });
        let mut by_grade = HashMap::new();
        for grade in Grade::iter() {
            match self.read_effects_by_type(grade, gamedatas_zip).await {
                Ok(effects) => {
                    by_grade.insert(grade, effects);
                }
                Err(e) => {
                    error!(?e, ?grade);
                }
            };
        }
        self.effects_by_grade = by_grade;
        Ok(())
    }

    async fn read_effects_by_type<R: Read + Seek>(&mut self, grade: Grade, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<u16, ItemOption>> {
        let mut file = match grade {
            Grade::Common => gamedatas_zip.by_path(r"gamedata\adatabin\itemoption_basicstatnormal.bin")?,
            Grade::Elite => gamedatas_zip.by_path(r"gamedata\adatabin\itemoption_basicstatelite.bin")?,
            Grade::Heroic => gamedatas_zip.by_path(r"gamedata\adatabin\itemoption_basicstatrare.bin")?,
            Grade::Legendary => gamedatas_zip.by_path(r"gamedata\adatabin\itemoption_basicstatlegend.bin")?,
            Grade::Unique => gamedatas_zip.by_path(r"gamedata\adatabin\itemoption_basicstatunique.bin")?,
            Grade::Mythical => gamedatas_zip.by_path(r"gamedata\adatabin\itemoption_basicstatancientmythic.bin")?,
        };

        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_effects(&data, DataFormat::String).await
    }

    async fn read_tempering_by_type<R: Read + Seek>(
        &mut self,
        item_type: &str,
        gamedatas_zip: &mut ZipArchive<R>,
    ) -> Result<HashMap<u16, Tempering>> {
        let mut file = if item_type == "ne_01" {
            gamedatas_zip.by_path(r"gamedata\adatabin\itemreinforcetable_am_01.bin")?
        } else if item_type == "sd_01" {
            gamedatas_zip.by_path(r"gamedata\adatabin\itemreinforcetable_sh_01.bin")?
        } else if item_type == "ga_01" {
            gamedatas_zip.by_path(r"gamedata\adatabin\itemreinforcetable_g1_01.bin")?
        } else if item_type == "at_01" {
            gamedatas_zip.by_path(r"gamedata\adatabin\itemreinforcetable_ar_01.bin")?
        } else {
            gamedatas_zip.by_path(format!(r"gamedata\adatabin\itemreinforcetable_{}.bin", item_type))?
        };

        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_tempering(&data, DataFormat::String).await
    }

    async fn read_qualites<R: Read + Seek>(
        &mut self,
        item_types: Vec<SharedString>,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Quality;
            cx.notify();
        });
        let mut quality_by_types = HashMap::new();
        for item_type in &item_types {
            match self.read_quality_by_type(item_type, gamedatas_zip).await {
                Ok(quality) => {
                    quality_by_types.insert(SharedString::new(item_type), quality);
                }
                Err(e) => {
                    error!(?e, ?item_type);
                }
            };
        }
        debug!(quality_keys = ?quality_by_types.keys());

        let keys = quality_by_types
            .iter()
            .map(|f| {
                f.1.iter()
                    .map(|f| f.1.advanced_fixed_effect.as_ref().and_then(|f| f.parsed.as_ref().map(|f| f.0.clone())))
            })
            .flatten()
            .filter_map(|f| f)
            .collect::<HashSet<_>>();
        debug!(?keys);
        self.quality_by_types = quality_by_types;
        Ok(())
    }

    async fn read_quality_by_type<R: Read + Seek>(
        &mut self,
        item_type: &str,
        gamedatas_zip: &mut ZipArchive<R>,
    ) -> Result<HashMap<u16, ItemQuality>> {
        let mut file = gamedatas_zip.by_path(format!(r"gamedata\adatabin\itemqualitytable_{}.bin", item_type))?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_quality(&data, DataFormat::String).await
    }

    async fn read_itemset<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<Vec<ItemSet>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::ItemSet;
            cx.notify();
        });
        let locales = self.read_itemset_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemset_setcharacter.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_itemset(&data, DataFormat::String, &locales, &skill_locales).await
    }

    async fn read_accessory<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Accessory;
            cx.notify();
        });

        let locales = self.read_accessory_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_accessory_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_accessory.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Accessory,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_armors<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Armor;
            cx.notify();
        });
        let locales = self.read_armor_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_armor_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_armor.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Armor,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_secondary_weapons<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SecondaryWeapon;
            cx.notify();
        });
        let locales = self.read_secondary_weapon_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_secondary_weapon_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_sub.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::SecondaryWeapon,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_weapons<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Weapon;
            cx.notify();
        });
        let locales = self.read_weapon_locales(gamedatas_zip).await?;
        let skill_locales = self.read_skill_locales(gamedatas_zip).await?;
        let res = self.read_weapon_itemres(gamedatas_zip).await?;
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemdata_weapon.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items(
            &data,
            DataFormat::String,
            DataType::Weapon,
            &locales,
            &skill_locales,
            &res,
            &item_set,
            gamelibs_zip,
        )
        .await
    }

    async fn read_secondary_weapon_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_subitem.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_weapon_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_weapon.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_skill_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_skill.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_accessory_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_accessory.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_accessory_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_accessory.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_armor_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_armor.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_weapon_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_weapon.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_secondary_weapon_itemres<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, ItemRes>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\adatabin\itemres_sub.bin")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_res(&data, DataFormat::String).await
    }

    async fn read_armor_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_armor.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_itemset_locales<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut file = gamedatas_zip.by_path(r"gamedata\localized\localstringdata_item_setitem.sxb")?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;
        self.read_items_locale(&data, DataFormat::WideString).await
    }

    async fn read_items<T: AbstractItem + Item, R: Read + Seek>(
        &mut self,
        data: &[u8],
        format: DataFormat,
        constructor: fn(T) -> DataType,
        locales: &HashMap<SharedString, Locale>,
        skill_locales: &HashMap<SharedString, Locale>,
        res: &HashMap<SharedString, ItemRes>,
        item_set: &Vec<ItemSet>,
        gamelibs_zip: &mut ZipArchive<R>,
    ) -> Result<()> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;

        for item_idx in 0..item_count {
            let mut item = T::default()
                .read(&mut reader, &offsets, item_idx, definitions.len(), global_offset, format)
                .await?;
            item.set_locale(locales, skill_locales);
            item.set_item_set(item_set);
            item.set_icon(res, gamelibs_zip).await?;
            let c = constructor(item);
            c.test();
            self.items.insert(c.get_id(), Rc::new(c));
        }

        Ok(())
    }

    async fn read_items_quality(&mut self, data: &[u8], format: DataFormat) -> Result<HashMap<u16, ItemQuality>> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut item_quality = HashMap::with_capacity(item_count);
        for item_idx in 0..item_count {
            let item = ItemQuality::default()
                .read(&mut reader, &offsets, item_idx, definitions.len(), global_offset, format)
                .await?;
            item_quality.insert(item.level, item);
        }

        Ok(item_quality)
    }

    async fn read_items_itemset(
        &mut self,
        data: &[u8],
        format: DataFormat,
        locales: &HashMap<SharedString, Locale>,
        skill_locales: &HashMap<SharedString, Locale>,
    ) -> Result<Vec<ItemSet>> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut item_set = Vec::with_capacity(item_count);
        for item_idx in 0..item_count {
            let mut item = ItemSet::default()
                .read(&mut reader, &offsets, item_idx, definitions.len(), global_offset, format)
                .await?;
            item.locale = locales.get(&item.setid).cloned();
            item.set_skill_effects_locale(skill_locales);
            item_set.push(item);
        }

        Ok(item_set)
    }

    async fn read_items_locale(&mut self, data: &[u8], format: DataFormat) -> Result<HashMap<SharedString, Locale>> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut locales = HashMap::with_capacity(item_count);
        for item_idx in 0..item_count {
            let item = Locale::default()
                .read(&mut reader, &offsets, item_idx, definitions.len(), global_offset, format)
                .await?;
            locales.insert(item.key.clone(), item);
        }

        Ok(locales)
    }

    async fn read_items_res(&mut self, data: &[u8], format: DataFormat) -> Result<HashMap<SharedString, ItemRes>> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut res = HashMap::with_capacity(item_count);
        for item_idx in 0..item_count {
            let item = ItemRes::default()
                .read(&mut reader, &offsets, item_idx, definitions.len(), global_offset, format)
                .await?;
            res.insert(item.id.clone(), item);
        }

        Ok(res)
    }

    async fn read_effects(&mut self, data: &[u8], format: DataFormat) -> Result<HashMap<u16, ItemOption>> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut effects = HashMap::with_capacity(item_count);
        for item_idx in 0..item_count {
            let item = ItemOption::default()
                .read(&mut reader, &offsets, item_idx, definitions.len(), global_offset, format)
                .await?;
            effects.insert(item.level, item);
        }

        Ok(effects)
    }

    async fn read_tempering(&mut self, data: &[u8], format: DataFormat) -> Result<HashMap<u16, Tempering>> {
        let cursor = Cursor::new(data);
        let mut reader = BufReader::new(cursor);

        let definitions = self.read_definitions(&mut reader).await?;
        if definitions.len() != 106 && definitions.len() != 61 && definitions.len() != 101 && definitions.len() != 121 && definitions.len() != 111 {
            warn!(?definitions, "Unknown tag_count in tempering");
        }
        let item_count = self.read_item_count(&mut reader).await?;
        let offsets = self.read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut tempering = HashMap::with_capacity(item_count);
        for item_idx in 0..item_count {
            let item = Tempering::default()
                .read(&mut reader, &offsets, item_idx, definitions.len(), global_offset, format)
                .await?;
            tempering.insert(item.level, item);
        }

        Ok(tempering)
    }

    async fn read_definitions(&self, reader: &mut BufReader<Cursor<&[u8]>>) -> Result<IndexMap<String, TagType>> {
        let tag_count = reader.read_u16_le().await? as usize;

        let mut definitions = IndexMap::with_capacity(tag_count);
        for _ in 0..tag_count {
            let type_id = reader.read_u8().await?;
            let tag_type = match type_id {
                1 => TagType::String,
                0 => TagType::Float,
                _ => return Err(anyhow::anyhow!("Unknown tag type")),
            };
            let len = reader.read_u8().await?;
            let mut value = vec![0; len as usize];
            reader.read_exact(&mut value).await?;
            let (key, _, _) = EUC_KR.decode(&value);
            definitions.insert(key.to_string(), tag_type);
        }
        //  debug!(?definitions);
        Ok(definitions)
    }

    async fn read_item_count(&self, reader: &mut BufReader<Cursor<&[u8]>>) -> Result<usize> {
        let item_count = reader.read_u16_le().await? as usize;
        Ok(item_count)
    }

    async fn read_offsets(&self, reader: &mut BufReader<Cursor<&[u8]>>, item_count: usize, tag_count: usize) -> Result<Vec<u32>> {
        let mut offsets = Vec::with_capacity(item_count * tag_count + 1);
        for _ in 0..=item_count * tag_count {
            let len = reader.read_u32_le().await?;
            offsets.push(len);
        }
        Ok(offsets)
    }
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct ItemEffect {
    pub effect: SharedString,
    pub parsed: Option<(SharedString, f32)>,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct ItemMinMaxEffect {
    pub effect: SharedString,
    pub parsed: Option<(SharedString, f32, f32)>,
}

impl ItemMinMaxEffect {
    pub fn new(effect: &str) -> Self {
        let mut e = Self::default();
        e.effect = SharedString::new(effect);
        e.parse_effect();
        e
    }

    pub fn get_locale(&self) -> SharedString {
        self.parsed
            .as_ref()
            .map(|(key, min, max)| {
                if key.ends_with("-minus-percent") {
                    t_v(key, vec![("value", format!("{:.2}% ~ -{:.2}", min, max))])
                } else if key.ends_with("-percent") {
                    t_v(key, vec![("value", format!("{:.2}% ~ {:.2}", min, max))])
                } else {
                    t_v(key, vec![("value", format!("{:.0} ~ {:.0}", min, max))])
                }
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or_else(|| self.effect.clone())
    }
    fn parse_key_min_max(input: &str) -> Option<(&str, f32, f32)> {
        let parts: Vec<&str> = input.split('_').collect();
        if parts.len() != 3 {
            return None;
        }

        let key = parts[0];
        let min = parts[1].parse::<f32>().ok()?;
        let max = parts[2].parse::<f32>().ok()?;

        Some((key, min, max))
    }

    fn parse_effect(&mut self) {
        if let Some((effect_key, min, max)) = Self::parse_key_min_max(&self.effect) {
            if let Some(effect_key) = ItemEffect::matching(effect_key) {
                self.parsed = Some((SharedString::new(effect_key), min, max));
            }
        } else {
            warn!(?self.effect,  "Can not parse effect");
        }
    }
}

impl ItemEffect {
    pub fn matching(key: &str) -> Option<&str> {
        match key {
            "최대ep%" | "최대EP%" => Some("item-effect-max-ep-percent"),
            "생명력흡수성공확률+" | "생명력흡수성공확률%" => Some("item-effect-health-absorption-chance-percent"),
            "생명력흡수량+" => Some("item-effect-health-absorption-amount-percent"),
            "데미지감소%" => Some("item-effect-damage-reduction-percent"),
            "석궁피격데미지%" => Some("item-effect-crossbow-damage-percent"),
            "창피격데미지%" => Some("item-effect-lance-damage-percent"),
            "창피격데미지%-" => Some("item-effect-lance-damage-minus-percent"),
            "배후공격극대화확률+" => Some("item-effect-backstab-damage"),
            "회피력+" => Some("item-effect-evasion-power"),
            "회피율%" => Some("item-effect-evasion-percent"), // хз, уклонение, проверить на Capital Guard Veiled Gloves
            "최대MP+" => Some("item-effect-mana"),
            "최대HP+" | "최대hp+" => Some("item-effect-max-hp"),
            "최대HP%" => Some("item-effect-max-hp-percent"),
            "무기물리방어력%" => Some("item-effect-physical-defense-percent"),
            "쿨타임%" => Some("item-effect-cooldown-percent"),
            "PK방어력%" | "pk방어력%" => Some("item-effect-pvp-defense-percent"),
            "모든공격력+" => Some("item-effect-attack"),
            "모든공격력%" => Some("item-effect-attack-percent"),
            "allstatderest+" | "AllStatDerest+" => Some("item-effect-stat-limit-break"),
            "allstatderest%" | "AllStatDerest%" => Some("item-effect-stat-limit-break-percent"),
            "allstat+" | "AllStat+" => Some("item-effect-allstats"),
            "allstat%" | "AllStat%" => Some("item-effect-allstats-percent"),
            "모든극대화확률+" => Some("item-effect-crit-damage-chance-percent"),
            "PK육체계저항율+" | "pk육체계저항율+" => Some("item-effect-pvp-resist-percent"),
            "이동속도%" => Some("item-effect-speed-percent"),
            "탈것속도%" => Some("item-effect-mount-speed-percent"),
            "치명타피해감소+" => Some("item-effect-crit-defense"),
            "마법방어력%" => Some("item-effect-magic-defense-percent"),
            "INTDerest+" => Some("item-effect-intelligence-break-limit"),
            "VTLDerest%" => Some("item-effect-vitality-break-limit-percent"),
            "STRDerest+" => Some("item-effect-strength-break-limit"),
            "INT%" => Some("item-effect-intelligence-percent"),
            "STR%" => Some("item-effect-strength-percent"),
            "VTL+" => Some("item-effect-vitality"),
            "MTL+" => Some("item-effect-mentality"),
            "INT+" | "int+" => Some("item-effect-intelligence"),
            "STR+" | "str+" => Some("item-effect-strength"),
            "DEX+" | "dex+" => Some("item-effect-dexterity"),

            "PK공격력%" => Some("item-effect-pvp-attack-percent"),
            "출혈관통률" => Some("item-effect-bleed-chance-percent"),
            "모든방어력%" => Some("item-effect-defense-percent"),
            "모든방어력+" => Some("item-effect-defense"),
            "무기물리방어력+" => Some("item-effect-physical-defense"),
            "무기물리공격력+" => Some("item-effect-physical-attack"),
            "마법방어력+" => Some("item-effect-magic-defense"),
            "캐스팅속도%" => Some("item-effect-cast-time-percent"),
            "마법물리공격력+" => Some("item-effect-magic-attack"),
            /* idk about 2 */
            "출혈방어율" | "출혈방어율%" => Some("item-effect-bleed-defense-percent"),
            "모든극대력+" => Some("item-effect-critical-damage"),
            "마법극대력+" => Some("item-effect-magic-critical-damage"),
            "마법극대화데미지+" => Some("item-effect-magic-critical-damage-percent"),
            /* idk about 2, this one is uniq [Lazards Priest set effect] */
            "마법극대화확률+" | "마법극대화확률%" => Some("item-effect-magic-critical-damage-chance-percent"),
            "무기극대화확률+" => Some("item-effect-physical-critical-damage-chance-percent"),
            "치명타피해관통율%" => Some("item-effect-critical-damage-penetration-percent"),
            "무기극대력+" => Some("item-effect-physical-critical-damage"),
            "무기극대화데미지+" => Some("item-effect-physical-critical-damage-percent"),
            "몬스터드랍율%" => Some("item-effect-drop-chance-percent"),
            "마법물리공격력%" => Some("item-effect-magic-attack-percent"),
            "무기물리공격력%" => Some("item-effect-physical-attack-percent"),
            "길들이기확률%" => Some("item-effect-taming-chance-percent"),
            "리버스강화확률%" => Some("item-effect-reverse-tempering-chance-percent"),
            "강화성공확률%" => Some("item-effect-tempering-chance-percent"),
            "제작성공확률%" => Some("item-effect-crafting-chance-percent"),
            "제작대성공확률%" => Some("item-effect-great-craft-chance-percent"),
            "판매대행등록비감소%" => Some("item-effect-auction-fee-percent"),
            "펠로우경험치%" => Some("item-effect-mount-exp-percent"),
            "도트데미지감소+" => Some("item-effect-bleed-damage-reduction"), //idk
            "도트데미지감소%" => Some("item-effect-bleed-damage-reduction-percent"), //idk
            "길들이기포인트감소%" => Some("item-effect-taming-points-percent"), // проверить потом на бафе зелек

            "드랍Money변화율*" => Some("item-effect-money-drop-increase-percent"),
            "Money추가획득율%" => Some("item-effect-money-drop-increase"),
            "공격자의치명타피해Plus효과감소%" => Some("item-effect-critical-defense-percent"),
            "최대MP%" => Some("item-effect-mana-percent"),
            "플레이어경험치%" => Some("item-effect-obtained-character-exp-percent"),

            "배후공격데미지%" => Some("item-effect-backstab-rate-percent"),
            "Hp힐량%" => Some("item-effect-health-regen-percent"),
            "어그로%" => Some("item-effect-threat-percent"),
            "hp회복력%" => Some("item-effect-base-health-regen-percent"),
            "마법물리방어력+" => Some("item-effect-magic-and-physical-defense"),
            _ => {
                warn!(key, "Can not detect effect");
                return None;
            }
        }
    }

    pub fn new(effect: SharedString) -> Self {
        let mut e = Self::default();
        e.effect = effect;
        e.parse_effect();
        e
    }

    fn parse_key_value(s: &str) -> Option<(&str, f32)> {
        let s = s.trim_start_matches("(").trim_end_matches(")");

        let mut parts = s.splitn(2, ',');
        let key = parts.next()?.trim();
        let value_str = parts.next()?.trim();
        let value = value_str.trim_end_matches("%").parse::<f32>().ok()?;

        Some((key, value))
    }

    pub fn get_locale(&self) -> SharedString {
        self.parsed
            .as_ref()
            .map(|(key, value)| {
                if key.ends_with("-minus-percent") {
                    t_v(key, vec![("value", format!("{:.2}", value))])
                } else if key.ends_with("-percent") {
                    t_v(key, vec![("value", format!("{:+.2}", value))])
                } else {
                    t_v(key, vec![("value", format!("{:+.0}", value))])
                }
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or_else(|| self.effect.clone())
    }

    fn parse_effect(&mut self) {
        if let Some((effect_key, value)) = Self::parse_key_value(&self.effect) {
            if let Some(effect_key) = Self::matching(effect_key) {
                self.parsed = Some((SharedString::new(effect_key), value));
            }
        } else {
            warn!(?self.effect,  "Can not parse effect");
        }
    }
}

pub trait AbstractItem: Sized + Default {
    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        tag_count: usize,
        global_offset: u64,
        format: DataFormat,
    ) -> Result<Self>;

    async fn dds_to_jpeg(bytes: Vec<u8>) -> Result<std::sync::Arc<Image>> {
        let data = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
            let cursor = Cursor::new(bytes);
            let img = ImageReader::new(cursor)
                .with_guessed_format()?
                .decode()?
                .resize(128, 128, FilterType::Triangle);

            let mut data = Vec::new();
            img.write_to(&mut Cursor::new(&mut data), image::ImageFormat::Jpeg)?;

            Ok(data)
        })
        .await??;

        Ok(std::sync::Arc::new(Image::from_bytes(gpui::ImageFormat::Jpeg, data)))
    }

    async fn read_string<R: AsyncBufReadExt + std::marker::Unpin>(format: DataFormat, reader: &mut R) -> Result<SharedString> {
        let s = match format {
            DataFormat::String => Self::read_c_string(reader).await?,
            DataFormat::WideString => Self::read_wide_c_string(reader).await?,
        };
        Ok(s)
    }

    async fn read_c_string<R: AsyncBufReadExt + std::marker::Unpin>(reader: &mut R) -> Result<SharedString> {
        let mut buffer = Vec::with_capacity(256);

        // 0 is the null terminator byte ('\0')
        reader.read_until(0, &mut buffer).await?;

        // Optional: Remove the trailing null byte if you don't want it in your vector
        if buffer.last() == Some(&0) {
            buffer.pop();
        }
        let (value, _, _) = EUC_KR.decode(&buffer);
        Ok(SharedString::new(value))
    }

    async fn read_wide_c_string<R: AsyncBufReadExt + std::marker::Unpin>(reader: &mut R) -> Result<SharedString> {
        let mut byte_buffer = Vec::with_capacity(256);

        let mut null_terminated = false;
        while !null_terminated {
            let byte = reader.read_u16_le().await?;

            if byte == 0 {
                null_terminated = true;
            } else {
                byte_buffer.push(byte);
            }
        }

        Ok(SharedString::from(String::from_utf16_lossy(&byte_buffer)))
    }
}

pub trait Item: Sized + Default {
    fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>, skill_locales: &HashMap<SharedString, Locale>);

    fn set_item_set(&mut self, item_set: &Vec<ItemSet>);

    fn get_full_type(&self) -> SharedString;
    fn get_type(&self) -> SharedString;

    async fn set_icon<R: Read + Seek>(&mut self, res: &HashMap<SharedString, ItemRes>, gamelibs_zip: &mut ZipArchive<R>) -> Result<()>;
}
