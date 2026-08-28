pub mod accessory;
pub mod armor;
pub mod bag;
pub mod boost;
pub mod consume;
pub mod exchange;
mod fellow_equip;
pub mod filters;
pub mod gem;
pub mod item_option;
pub mod item_quality;
pub mod item_res;
pub mod item_set;
pub mod locale;
pub mod material;
pub mod package;
pub mod product;
pub mod random_box;
pub mod random_box_group;
pub mod random_box_probability;
pub mod recipe;
pub mod sealed_fellow;
pub mod secondary_weapon;
pub mod skill_book;
pub mod style;
pub mod tempering;
pub mod weapon;

use anyhow::Result;

use encoding_rs::EUC_KR;
use enum_dispatch::enum_dispatch;
use gpui::{AsyncWindowContext, Entity, Hsla, Image, SharedString, hsla};

use image::{ImageReader, imageops::FilterType};
use indexmap::IndexMap;
use itertools::Itertools;
use rust_xlsxwriter::{DocProperties, Table, TableColumn, workbook::Workbook};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::File,
    hash::Hash,
    io::{Cursor, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::Arc,
};
use strum::{EnumIter, FromRepr, IntoEnumIterator};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt, BufReader};
use tracing::{debug, error, warn};
use zip::ZipArchive;

use crate::{
    game_data::{
        accessory::Accessory, armor::Armor, bag::Bag, boost::Boost, consume::Consume, exchange::Exchange, fellow_equip::FellowEquip, gem::Gem,
        item_option::ItemOption, item_quality::ItemQuality, item_res::ItemRes, item_set::ItemSet, locale::Locale, material::Material,
        package::Package, product::Product, random_box::RandomBox, random_box_group::RandomBoxGroup, random_box_probability::RandomBoxProbability,
        recipe::Recipe, sealed_fellow::SealedFellow, secondary_weapon::SecondaryWeapon, skill_book::SkillBook, style::Style, tempering::Tempering,
        weapon::Weapon,
    },
    game_data_view::GameDataLoadingStatus,
    language::{LanguageController, t, t_v},
};

#[derive(Default, Serialize, Clone)]
pub struct ItemEffect {
    pub effect: SharedString,
    pub parsed: Option<(SharedString, f32)>,
}

#[derive(Default, Serialize, Clone)]
pub struct ItemMinMaxStepEffect {
    pub effect: SharedString,
    pub parsed: Option<(SharedString, f32, f32, f32)>,
}
#[derive(Default, Serialize, Clone)]
pub struct ItemMinMaxNoStepEffect {
    pub effect: SharedString,
    pub parsed: Option<(SharedString, f32, f32)>,
}

#[derive(Default, Serialize, Clone)]
pub struct ItemMinMaxEffect {
    pub effect: SharedString,
    pub parsed: Option<(SharedString, f32, f32)>,
}

impl ItemMinMaxStepEffect {
    pub fn new(effect: &str) -> Self {
        let mut e = Self::default();
        e.effect = SharedString::new(effect);
        e.parse_effect();
        e
    }

    pub fn get_locale(&self, maximized: bool, tempering_effect: f32) -> SharedString {
        self.parsed
            .as_ref()
            .map(|(key, min, max, step)| {
                let (min, max) = if maximized {
                    (
                        (min + step) * (1.0 + tempering_effect / 100.0),
                        (max + step) * (1.0 + tempering_effect / 100.0),
                    )
                } else {
                    (min * (1.0 + tempering_effect / 100.0), max * (1.0 + tempering_effect / 100.0))
                };

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
    fn parse_key_min_max_step(input: &str) -> Option<(&str, f32, f32, f32)> {
        let parts: Vec<&str> = input.split(',').collect();
        if parts.len() != 4 {
            return None;
        }

        let key = parts[0];
        let min = parts[1].parse::<f32>().ok()?;
        let max = parts[2].parse::<f32>().ok()?;
        let step = parts[3].parse::<f32>().ok()?;

        Some((key, min, max, step))
    }

    fn parse_effect(&mut self) {
        if let Some((effect_key, min, max, step)) = Self::parse_key_min_max_step(&self.effect) {
            if let Some(effect_key) = ItemEffect::matching(effect_key) {
                self.parsed = Some((SharedString::new(effect_key), min, max, step));
            }
        }
    }
}

impl ItemMinMaxNoStepEffect {
    pub fn new(effect: &str) -> Self {
        let mut e = Self::default();
        e.effect = SharedString::new(effect);
        e.parse_effect();
        e
    }

    pub fn get_locale(&self, tempering_effect: f32) -> SharedString {
        self.parsed
            .as_ref()
            .map(|(key, min, max)| {
                let min = (min) * (1.0 + tempering_effect / 100.0);
                let max = (max) * (1.0 + tempering_effect / 100.0);

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
        let parts: Vec<&str> = input.split(',').collect();
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
        }
    }
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
            "회피율%" | "회피율+" => Some("item-effect-evasion-percent"), // хз, уклонение, проверить на Capital Guard Veiled Gloves
            "최대MP+" => Some("item-effect-mana"),
            "최대HP+" | "최대hp+" | "최대Hp+" => Some("item-effect-max-hp"),
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
            "INTDerest+" | "intderest+" => Some("item-effect-intelligence-break-limit"),
            "INTDerest%" | "intderest%" | "intDerest%" => Some("item-effect-intelligence-break-limit-percent"),
            "VTLDerest+" | "vtlderest+" => Some("item-effect-vitality-break-limit"),
            "VTLDerest%" | "vtlderest%" => Some("item-effect-vitality-break-limit-percent"),
            "STRDerest+" | "strderest+" => Some("item-effect-strength-break-limit"),
            "STRDerest%" | "strderest%" | "strDerest%" => Some("item-effect-strength-break-limit-percent"),
            "DEXDerest+" | "dexderest+" => Some("item-effect-dexterity-break-limit"),
            "DEXDerest%" | "dexderest%" => Some("item-effect-dexterity-break-limit-percent"),
            "MTLDerest+" | "mtlderest+" => Some("item-effect-mentality-break-limit"),
            "MTLDerest%" | "mtlderest%" => Some("item-effect-mentality-break-limit-percent"),
            "INT%" | "int%" => Some("item-effect-intelligence-percent"),
            "STR%" | "str%" => Some("item-effect-strength-percent"),
            "VTL%" | "vtl%" => Some("item-effect-vitality-percent"),
            "MTL%" | "mtl%" => Some("item-effect-mentality-percent"),
            "DEX%" | "dex%" => Some("item-effect-dexterity-percent"),
            "VTL+" | "vtl+" => Some("item-effect-vitality"),
            "MTL+" | "mtl+" => Some("item-effect-mentality"),
            "INT+" | "int+" | "Int+" => Some("item-effect-intelligence"),
            "STR+" | "str+" | "Str+" => Some("item-effect-strength"),
            "DEX+" | "dex+" => Some("item-effect-dexterity"),

            "PK공격력%" | "pk공격력%" => Some("item-effect-pvp-attack-percent"),
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
            "무기극대화데미지+" | "무기극대력%" => Some("item-effect-physical-critical-damage-percent"),
            "몬스터드랍율%" | "드랍율+" => Some("item-effect-drop-chance-percent"),
            "마법물리공격력%" => Some("item-effect-magic-attack-percent"),
            "무기물리공격력%" => Some("item-effect-physical-attack-percent"),
            "길들이기확률%" => Some("item-effect-taming-chance-percent"),
            "리버스강화확률%" => Some("item-effect-reverse-tempering-chance-percent"),
            "강화성공확률%" => Some("item-effect-tempering-chance-percent"),
            "제작성공확률%" => Some("item-effect-crafting-chance-percent"),
            "제작대성공확률%" => Some("item-effect-great-craft-chance-percent"),
            "판매대행등록비감소%" => Some("item-effect-auction-fee-percent"),
            "판매대행판매수수료감소%" => Some("item-effect-auction-sales-fee-percent"),
            "펠로우경험치%" | "접속중펠로우위탁경험치%" => Some("item-effect-mount-exp-percent"),
            "도트데미지감소+" => Some("item-effect-bleed-damage-reduction"), //idk
            "도트데미지감소%" => Some("item-effect-bleed-damage-reduction-percent"), //idk
            "길들이기포인트감소%" => Some("item-effect-taming-points-percent"), // проверить потом на бафе зелек
            "고도+" => Some("item-effect-mount-altitude"),
            "드랍Money변화율*" | "드랍money변화율*" => Some("item-effect-money-drop-increase-percent"),
            "Money추가획득율%" => Some("item-effect-money-drop-increase"),
            "공격자의치명타피해Plus효과감소%" | "공격자의치명타피해plus효과감소%" => {
                Some("item-effect-critical-defense-percent")
            }
            "최대MP%" => Some("item-effect-mana-percent"),
            "플레이어경험치%" => Some("item-effect-obtained-character-exp-percent"),

            "배후공격데미지%" => Some("item-effect-backstab-rate-percent"),
            "Hp힐량%" => Some("item-effect-health-regen-percent"),
            "어그로%" => Some("item-effect-threat-percent"),
            "hp회복력%" | "Hp회복력%" | "HP회복력%" => Some("item-effect-base-health-regen-percent"),
            "마법물리방어력+" => Some("item-effect-magic-and-physical-defense"),
            "낚시시간감소" => Some("item-effect-fishing-time-sec"),
            "펫포획확률%" => Some("item-effect-capturing-chance-percent"),
            "월척확률증가%" => Some("item-effect-fishing-very-rare-drop-percent"),
            "모든낚시확률증가%" => Some("item-effect-fishing-drop-percent"),
            "준척확률증가%" => Some("item-effect-fishing-rare-drop-percent"),
            "길드포인트%" => Some("item-effect-guild-points-percent"),
            _ => {
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
        }
    }
}

#[derive(Default, Clone)]
pub struct ItemNode {
    pub id: SharedString,
    pub item: Option<Weak<Item>>,
}

#[derive(Debug, Clone, Copy)]
pub enum TagType {
    String,
    Float,
}
#[derive(Copy, Clone, Default)]
pub enum DataFormat {
    #[default]
    String,
    WideString,
}

#[derive(Debug, EnumIter, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
pub enum ArmorClassKind {
    Magic(ArmorTypes),
    Physical(ArmorTypes),
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
pub enum ArmorTypes {
    Helmet,
    Pauldron,
    Armor,
    Gloves,
    Boots,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
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

#[derive(EnumIter, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
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

#[derive(EnumIter, Eq, PartialEq, Hash, Clone, Copy)]
pub enum ItemType {
    Armor,
    SecondaryWeapon,
    Weapon,
    Accessory,
    Material,
    Recipe,
    FellowEquip,
    Consume,
    Boost,
    Gem,
    SealedFellow,
    SkillBook,
    Exchange,
    RandomBox,
    Package,
    Style,
    Bag,
}

impl ItemType {
    pub fn locale(&self) -> SharedString {
        match self {
            ItemType::Armor => t("item-type-armor"),
            ItemType::SecondaryWeapon => t("item-type-secondary-weapon"),
            ItemType::Weapon => t("item-type-weapon"),
            ItemType::Accessory => t("item-type-accessory"),
            ItemType::Material => t("item-type-material"),
            ItemType::Recipe => t("item-type-recipe"),
            ItemType::FellowEquip => t("item-type-fellow-equip"),
            ItemType::Consume => t("item-type-consume"),
            ItemType::Boost => t("item-type-boost"),
            ItemType::Gem => t("item-type-gem"),
            ItemType::SealedFellow => t("item-type-sealed-fellow"),
            ItemType::SkillBook => t("item-type-skill-book"),
            ItemType::Exchange => t("item-type-exchange"),
            ItemType::RandomBox => t("item-type-random-box"),
            ItemType::Package => t("item-type-package"),
            ItemType::Style => t("item-type-style"),
            ItemType::Bag => t("item-type-bag"),
        }
    }
}

#[derive(Default, Clone, Copy)]
pub enum Quality {
    #[default]
    Simple,
    Good,
    Perfect,
}

#[derive(Clone, Copy, Serialize, Debug)]
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

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum DebugValue {
    String(SharedString),
    Float(f32),
}

type DebugItem = IndexMap<SharedString, DebugValue>;

#[derive(Default)]
pub struct Common {
    pub debug: Vec<u8>,
    pub linked_recipes: BTreeSet<SharedString>,
    pub item_set: Option<ItemSet>,
    pub locale: Option<Locale>,
    pub skill_locale: Option<Locale>,
    pub icon: Option<Arc<Image>>,

    pub id: SharedString,
    pub grade: Grade,
    pub no_trade: bool,
    pub no_sell: bool,
    pub no_destroy: bool,
    pub binding: Option<Binding>,
    pub required_level: u8,
    pub item_level: u16,
    pub usable_class: BTreeSet<GameClass>,

    pub effects: Vec<ItemEffect>,
}

impl Common {
    pub fn get_debug(&self) -> Result<String> {
        Ok(String::from_utf8(lz4_flex::block::decompress_size_prepended(&self.debug)?)?)
    }

    pub fn get_unique_effects(&self) -> HashSet<SharedString> {
        let mut effects = HashSet::new();
        effects.extend(self.effects.iter().filter_map(|f| f.parsed.as_ref().map(|(key, _)| key.clone())));

        if let Some(set) = &self.item_set {
            effects.extend(
                set.effects
                    .iter()
                    .flat_map(|f| f.seteffect_effects.iter())
                    .filter_map(|f| f.parsed.as_ref().map(|(key, _)| key.clone())),
            );
        }

        effects
    }

    pub fn get_localized_name(&self) -> SharedString {
        self.locale.as_ref().and_then(|f| f.locale()).unwrap_or_else(|| self.id.clone())
    }

    fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.locale = locales.get(&self.id).cloned();
    }

    fn set_item_set(&mut self, item_set: &HashMap<SharedString, ItemSet>) {
        self.item_set = item_set.get(&self.id).cloned();
    }

    fn set_linked_recipes(&mut self, products_by_recipe_id: &HashMap<SharedString, Rc<RefCell<Product>>>) {
        self.linked_recipes = products_by_recipe_id
            .iter()
            .filter_map(|(_, product)| {
                let p = product.borrow();
                if p.node.id == self.id
                    || p.materials
                        .values()
                        .any(|m| m.node.id == self.id || m.additional_node.as_ref().is_some_and(|f| f.id == self.id))
                {
                    Some(p.productid.clone())
                } else {
                    None
                }
            })
            .collect();
    }

    async fn set_icon<R: std::io::Read + std::io::Seek>(
        &mut self,
        res: &HashMap<SharedString, ItemRes>,
        zip: &mut zip::ZipArchive<R>,
        icon_cache: &mut HashMap<String, Arc<Image>>,
    ) -> Result<()> {
        if let Some(item_res) = res.get(&self.id) {
            let icon_key = item_res.icon.to_lowercase();
            if let Some(icon) = icon_cache.get(&icon_key) {
                self.icon = Some(icon.clone());
                return Ok(());
            }

            if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon.to_lowercase())) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match dds_to_jpeg(buf).await {
                    Ok(icon) => {
                        icon_cache.insert(icon_key, icon.clone());
                        self.icon = Some(icon);
                    }
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            } else if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon)) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match dds_to_jpeg(buf).await {
                    Ok(icon) => {
                        icon_cache.insert(icon_key, icon.clone());
                        self.icon = Some(icon);
                    }
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            }
        }

        Ok(())
    }

    pub async fn parse_id<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        self.id = SharedString::new(reader.read_string(format).await?.to_uppercase());
        Ok(())
    }

    pub async fn parse_usable_class<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        &mut self,
        reader: &mut R,
        format: DataFormat,
    ) -> Result<()> {
        let value = reader.read_string(format).await?;

        self.usable_class = value.split("_").filter_map(|c| GameClass::try_from(c).ok()).collect();
        Ok(())
    }

    pub async fn parse_effect<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        let effect = reader.read_string(format).await?;
        if effect != "*" && effect != "0" {
            self.effects.push(ItemEffect::new(effect));
        }
        Ok(())
    }

    pub async fn parse_binding<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        self.binding = Binding::try_from(reader.read_string(format).await?.as_str()).ok();
        Ok(())
    }

    pub async fn parse_no_trade<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.no_trade = reader.read_f32_le().await? != 0.0;
        Ok(())
    }

    pub async fn parse_no_sell<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.no_sell = reader.read_f32_le().await? != 0.0;
        Ok(())
    }

    pub async fn parse_no_destroy<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.no_destroy = reader.read_f32_le().await? != 0.0;
        Ok(())
    }

    pub async fn parse_grade<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.grade = Grade::from(reader.read_f32_le().await? as u8);
        Ok(())
    }

    pub async fn parse_item_level<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.item_level = reader.read_f32_le().await? as u16;
        Ok(())
    }

    pub async fn parse_required_level<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.required_level = reader.read_f32_le().await? as u8;
        Ok(())
    }
}

#[enum_dispatch]
pub trait ItemTrait {
    fn common(&self) -> &Common;
    fn get_id(&self) -> SharedString {
        self.common().id.clone()
    }
    fn get_locale(&self) -> Option<&Locale> {
        self.common().locale.as_ref()
    }

    fn get_localized_name(&self) -> SharedString {
        self.common().get_localized_name()
    }

    fn get_icon(&self) -> Option<Arc<Image>> {
        self.common().icon.clone()
    }
    fn get_grade(&self) -> Grade {
        self.common().grade
    }

    fn get_unique_effects(&self) -> HashSet<SharedString> {
        self.common().get_unique_effects()
    }

    fn get_type(&self) -> Option<SharedString> {
        None
    }

    fn get_full_type(&self) -> Option<SharedString> {
        None
    }
}

#[enum_dispatch(ItemTrait)]
pub enum Item {
    SecondaryWeapon(SecondaryWeapon),
    Weapon(Weapon),
    Armor(Armor),
    Accessory(Accessory),
    Material(Material),
    Recipe(Recipe),
    FellowEquip(FellowEquip),
    Consume(Consume),
    Boost(Boost),
    Gem(Gem),
    SealedFellow(SealedFellow),
    SkillBook(SkillBook),
    Exchange(Exchange),
    RandomBox(RandomBox),
    Package(Package),
    Style(Style),
    Bag(Bag),
}

impl Item {
    pub fn filter_effect(&self, filter: &Option<SharedString>) -> bool {
        if let Some(filter) = filter {
            let effects = self.get_unique_effects();
            return effects.contains(filter);
        }
        return true;
    }

    fn matches(&self, input: &str, types: &HashSet<ItemType>, grades: &HashSet<Grade>, effect: &Option<SharedString>) -> bool {
        let include = match self {
            Self::Weapon(_) => types.contains(&ItemType::Weapon),
            Self::Armor(_) => types.contains(&ItemType::Armor),
            Self::Accessory(_) => types.contains(&ItemType::Accessory),
            Self::SecondaryWeapon(_) => types.contains(&ItemType::SecondaryWeapon),
            Self::Material(_) => types.contains(&ItemType::Material),
            Self::Recipe(_) => types.contains(&ItemType::Recipe),
            Self::FellowEquip(_) => types.contains(&ItemType::FellowEquip),
            Self::Consume(_) => types.contains(&ItemType::Consume),
            Self::Boost(_) => types.contains(&ItemType::Boost),
            Self::Gem(_) => types.contains(&ItemType::Gem),
            Self::SealedFellow(_) => types.contains(&ItemType::SealedFellow),
            Self::SkillBook(_) => types.contains(&ItemType::SkillBook),
            Self::Exchange(_) => types.contains(&ItemType::Exchange),
            Self::RandomBox(_) => types.contains(&ItemType::RandomBox),
            Self::Package(_) => types.contains(&ItemType::Package),
            Self::Style(_) => types.contains(&ItemType::Style),
            Self::Bag(_) => types.contains(&ItemType::Bag),
        };

        if !include {
            return false;
        }

        if !self.filter_effect(effect) {
            return false;
        }

        let grade = self.get_grade();

        if !grades.contains(&grade) {
            return false;
        }

        if input.is_empty() {
            return true;
        }

        let id = self.get_id();
        let locale = self.get_locale();
        id.to_lowercase().contains(&input)
            || locale.is_some_and(|l| l.rus.to_lowercase().contains(&input))
            || locale.is_some_and(|l| l.eng.to_lowercase().contains(&input))
    }
}

#[derive(Default)]
pub struct GameData {
    pub items: IndexMap<SharedString, Rc<Item>>,
    pub effects_by_grade: HashMap<Grade, HashMap<u16, ItemOption>>,
    pub tempering_by_types: HashMap<SharedString, HashMap<u16, Tempering>>,
    pub quality_by_types: HashMap<SharedString, HashMap<u16, ItemQuality>>,
    random_box_groups: HashMap<SharedString, Rc<RefCell<RandomBoxGroup>>>,
    products_by_recipe_id: HashMap<SharedString, Rc<RefCell<Product>>>,
    products_by_result_id: HashMap<SharedString, Rc<RefCell<Product>>>,
    icon_cache: HashMap<String, Arc<Image>>,
}

impl GameData {
    pub fn get_all_effects(&self) -> BTreeSet<SharedString> {
        self.items.iter().flat_map(|(_, item)| item.get_unique_effects()).collect()
    }

    pub async fn load(game_path: &str, on_load: &Entity<GameDataLoadingStatus>, cx: &mut AsyncWindowContext) -> Result<Self> {
        let gamedatas = File::open(Path::new(game_path).join(r"Game\gamedatas.npk"))?;
        let gamelibs = File::open(Path::new(game_path).join(r"Game\gamelibs.npk"))?;
        let mut gamedatas_zip = ZipArchive::new(gamedatas)?;
        let mut gamelibs_zip = ZipArchive::new(gamelibs)?;
        let mut data = Self::default();

        let item_set = Self::load_itemset(&mut gamedatas_zip, on_load, cx).await?;
        data.load_product_materials(&mut gamedatas_zip, on_load, cx).await?;
        let random_box_probabilities = Self::load_random_box_probabilities(&mut gamedatas_zip, on_load, cx).await?;
        data.load_random_box_groups(&mut gamedatas_zip, on_load, cx).await?;
        data.load_boosts(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;
        let item_set_fellow = Self::load_itemset_fellow(&mut gamedatas_zip, on_load, cx).await?;

        data.load_consumes(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;
        data.load_recipes(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;
        data.load_fellow_equips(&mut gamedatas_zip, &mut gamelibs_zip, &item_set_fellow, on_load, cx)
            .await?;
        data.load_exchanges(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;
        data.load_gems(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;
        data.load_bags(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;
        data.load_skill_books(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;

        data.load_sealed_fellows(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;
        data.load_weapons(&mut gamedatas_zip, &mut gamelibs_zip, &item_set, on_load, cx).await?;
        data.load_accessory(&mut gamedatas_zip, &mut gamelibs_zip, &item_set, on_load, cx).await?;
        data.load_secondary_weapons(&mut gamedatas_zip, &mut gamelibs_zip, &item_set, on_load, cx)
            .await?;
        data.load_armors(&mut gamedatas_zip, &mut gamelibs_zip, &item_set, on_load, cx).await?;
        data.load_styles(&mut gamedatas_zip, &mut gamelibs_zip, &item_set, on_load, cx).await?;
        data.load_materials(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;

        data.load_temperings(
            data.items.iter().filter_map(|(_, item)| item.get_full_type()).collect(),
            &mut gamedatas_zip,
            on_load,
            cx,
        )
        .await?;
        data.load_options(&mut gamedatas_zip, on_load, cx).await?;

        data.load_qualites(
            data.items.iter().filter_map(|(_, item)| item.get_type()).collect(),
            &mut gamedatas_zip,
            on_load,
            cx,
        )
        .await?;
        data.load_random_boxes(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;
        // Should be last one
        data.load_packages(&mut gamedatas_zip, &mut gamelibs_zip, on_load, cx).await?;

        for (_, item) in &data.products_by_recipe_id {
            item.borrow_mut().set_materials(&data.items);
        }

        for (_, item) in &data.random_box_groups {
            item.borrow_mut().set_items(&data.items, &random_box_probabilities);
        }

        Ok(data)
    }

    async fn load_locales<R: std::io::Read + std::io::Seek>(
        gamedatas_zip: &mut ZipArchive<R>,
        locale_path: &str,
    ) -> Result<HashMap<SharedString, Locale>> {
        Locale::read_all(gamedatas_zip, locale_path).await
    }

    async fn load_itemres<R: Read + std::io::Seek>(gamedatas_zip: &mut ZipArchive<R>, itemres_path: &str) -> Result<HashMap<SharedString, ItemRes>> {
        ItemRes::read_all(gamedatas_zip, itemres_path).await
    }

    async fn load_itemset<R: Read + std::io::Seek>(
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<HashMap<SharedString, ItemSet>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::ItemSet;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_setitem.sxb").await?;
        let skill_locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_skill.sxb").await?;
        let mut items = ItemSet::read_all(gamedatas_zip, r"gamedata\adatabin\itemset_setcharacter.bin").await?;
        for (_, item) in items.iter_mut() {
            item.set_locale(&locales);
            item.set_effects_skill_locale(&skill_locales);
        }
        Ok(items)
    }

    async fn load_product_materials<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::ProductMaterial;
            cx.notify();
        });
        let items = Product::read_all(gamedatas_zip, r"gamedata\adatabin\productdata_productmaterial.bin").await?;
        let mut products_by_recipe_id = HashMap::with_capacity(items.len());
        let mut products_by_result_id = HashMap::with_capacity(items.len());
        for (key, item) in items {
            let productid = item.productid.clone();
            let item = Rc::new(RefCell::new(item));
            products_by_recipe_id.insert(productid, item.clone());
            products_by_result_id.insert(key, item.clone());
        }

        self.products_by_recipe_id = products_by_recipe_id;
        self.products_by_result_id = products_by_result_id;

        Ok(())
    }

    async fn load_random_box_probabilities<R: Read + Seek>(
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<HashMap<SharedString, RandomBoxProbability>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::RandomBoxProbability;
            cx.notify();
        });
        RandomBoxProbability::read_all(gamedatas_zip, r"gamedata\adatabin\randomboxtable_randomboxprobability.bin").await
    }

    async fn load_random_box_groups<R: Read + Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::RandomBoxGroup;
            cx.notify();
        });
        let items = RandomBoxGroup::read_all(gamedatas_zip, r"gamedata\adatabin\randomboxtable_randomboxgroup.bin").await?;
        for (key, mut item) in items {
            self.random_box_groups.insert(key, Rc::new(RefCell::new(item)));
        }

        Ok(())
    }

    async fn load_itemset_fellow<R: Read + std::io::Seek>(
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<HashMap<SharedString, ItemSet>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::ItemSet;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_setitem.sxb").await?;
        let skill_locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_skill.sxb").await?;
        let mut items = ItemSet::read_all(gamedatas_zip, r"gamedata\adatabin\itemset_setfellow.bin").await?;
        for (_, item) in items.iter_mut() {
            item.set_locale(&locales);
            item.set_effects_skill_locale(&skill_locales);
        }
        Ok(items)
    }

    async fn load_armors<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &HashMap<SharedString, ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Armor;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_armor.sxb").await?;
        let skill_locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_skill.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_armor.bin").await?;

        let items = Armor::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_armor.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_skill_locale(&skill_locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;
            item.common.set_item_set(item_set);
            self.items.insert(key, Rc::new(Item::Armor(item)));
        }

        Ok(())
    }

    async fn load_weapons<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &HashMap<SharedString, ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Weapon;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_weapon.sxb").await?;
        let skill_locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_skill.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_weapon.bin").await?;

        let items = Weapon::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_weapon.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_skill_locale(&skill_locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;
            item.common.set_item_set(item_set);
            self.items.insert(key, Rc::new(Item::Weapon(item)));
        }

        Ok(())
    }

    async fn load_accessory<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &HashMap<SharedString, ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Accessory;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_accessory.sxb").await?;
        let skill_locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_skill.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_accessory.bin").await?;

        let items = Accessory::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_accessory.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_skill_locale(&skill_locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;
            item.common.set_item_set(item_set);
            self.items.insert(key, Rc::new(Item::Accessory(item)));
        }

        Ok(())
    }

    async fn load_styles<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &HashMap<SharedString, ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Style;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_style.sxb").await?;
        let skill_locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_skill.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_style.bin").await?;

        let items = Style::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_style.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_skill_locale(&skill_locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;
            item.common.set_item_set(item_set);
            self.items.insert(key, Rc::new(Item::Style(item)));
        }

        Ok(())
    }

    async fn load_secondary_weapons<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &HashMap<SharedString, ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SecondaryWeapon;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_subitem.sxb").await?;
        let skill_locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_skill.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_sub.bin").await?;

        let items = SecondaryWeapon::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_sub.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_skill_locale(&skill_locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;
            item.common.set_item_set(item_set);
            self.items.insert(key, Rc::new(Item::SecondaryWeapon(item)));
        }

        Ok(())
    }

    async fn load_fellow_equips<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        item_set: &HashMap<SharedString, ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::FellowEquip;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_fellowequip.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_fellowequip.bin").await?;

        let items = FellowEquip::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_fellowequip.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);

            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;
            item.common.set_item_set(item_set);
            self.items.insert(key, Rc::new(Item::FellowEquip(item)));
        }

        Ok(())
    }

    async fn load_packages<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,

        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Package;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_package.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_package.bin").await?;

        let items = Package::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_package.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_package_contents(&self.items);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::Package(item)));
        }

        Ok(())
    }

    async fn load_random_boxes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,

        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::RandomBox;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_randombox.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_randombox.bin").await?;

        let items = RandomBox::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_randombox.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_random_box_group(&self.random_box_groups);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::RandomBox(item)));
        }

        Ok(())
    }

    async fn load_boosts<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Boost;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_boost.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_boost.bin").await?;

        let items = Boost::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_boost.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::Boost(item)));
        }

        Ok(())
    }

    async fn load_materials<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Material;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_material.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_material.bin").await?;

        let items = Material::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_material.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_description_locale(&locales);
            item.set_recipe_type(&res);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::Material(item)));
        }

        Ok(())
    }

    async fn load_consumes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Consume;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_consume.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_consume.bin").await?;

        let items = Consume::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_consume.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::Consume(item)));
        }

        Ok(())
    }

    async fn load_bags<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Bag;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_bag.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_bag.bin").await?;

        let items = Bag::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_bag.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::Bag(item)));
        }

        Ok(())
    }

    async fn load_exchanges<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Exchange;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_exchange.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_exchange.bin").await?;

        let items = Exchange::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_exchange.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::Exchange(item)));
        }

        Ok(())
    }

    async fn load_sealed_fellows<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SealedFellow;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_sealedfellow.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_sealedfellow.bin").await?;

        let items = SealedFellow::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_sealedfellow.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);

            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::SealedFellow(item)));
        }

        Ok(())
    }

    async fn load_gems<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Gem;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_enchantstone.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_enchantstone.bin").await?;

        let items = Gem::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_enchantstone.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::Gem(item)));
        }

        Ok(())
    }

    async fn load_skill_books<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SkillBook;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_skillbook.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_skillbook.bin").await?;

        let items = SkillBook::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_skillbook.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::SkillBook(item)));
        }

        Ok(())
    }

    async fn load_recipes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Recipe;
            cx.notify();
        });
        let locales = Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_recipe.sxb").await?;
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_recipe.bin").await?;

        let items = Recipe::read_all(gamedatas_zip, r"gamedata\adatabin\itemdata_recipe.bin").await?;
        for (key, mut item) in items {
            item.common.set_locale(&locales);
            item.set_product(&self.products_by_recipe_id, &self.products_by_result_id);
            item.common.set_icon(&res, gamelibs_zip, &mut self.icon_cache).await?;

            self.items.insert(key, Rc::new(Item::Recipe(item)));
        }

        Ok(())
    }

    async fn load_temperings<R: Read + Seek>(
        &mut self,
        item_types: HashSet<SharedString>,
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
            match Tempering::read_all(
                gamedatas_zip,
                &match item_type.as_str() {
                    "ne_01" => r"gamedata\adatabin\itemreinforcetable_am_01.bin".to_string(),
                    "sd_01" => r"gamedata\adatabin\itemreinforcetable_sh_01.bin".to_string(),
                    "ga_01" => r"gamedata\adatabin\itemreinforcetable_g1_01.bin".to_string(),
                    "at_01" => r"gamedata\adatabin\itemreinforcetable_ar_01.bin".to_string(),
                    _ => format!(r"gamedata\adatabin\itemreinforcetable_{}.bin", item_type),
                },
            )
            .await
            {
                Ok(tempering) => {
                    tempering_by_types.insert(SharedString::new(item_type), tempering);
                }
                Err(e) => {
                    error!(?e, ?item_type);
                }
            };
        }
        self.tempering_by_types = tempering_by_types;
        Ok(())
    }

    async fn load_options<R: Read + Seek>(
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
        for grade in Grade::iter().filter(|variant| !matches!(variant, Grade::Unknown(_))) {
            match ItemOption::read_all(
                gamedatas_zip,
                match grade {
                    Grade::Common => r"gamedata\adatabin\itemoption_basicstatnormal.bin",
                    Grade::Elite => r"gamedata\adatabin\itemoption_basicstatelite.bin",
                    Grade::Heroic => r"gamedata\adatabin\itemoption_basicstatrare.bin",
                    Grade::Legendary | Grade::LegendaryPlus => r"gamedata\adatabin\itemoption_basicstatlegend.bin",
                    Grade::Unique => r"gamedata\adatabin\itemoption_basicstatunique.bin",
                    Grade::Mythical => r"gamedata\adatabin\itemoption_basicstatancientmythic.bin",
                    Grade::Unknown(_) => unimplemented!(),
                },
            )
            .await
            {
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

    async fn load_qualites<R: Read + Seek>(
        &mut self,
        item_types: HashSet<SharedString>,
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
            match ItemQuality::read_all(gamedatas_zip, &format!(r"gamedata\adatabin\itemqualitytable_{}.bin", item_type)).await {
                Ok(quality) => {
                    quality_by_types.insert(SharedString::new(item_type), quality);
                }
                Err(e) => {
                    error!(?e, ?item_type);
                }
            };
        }

        self.quality_by_types = quality_by_types;
        Ok(())
    }

    pub fn export_xlsx(&self, path: PathBuf) -> Result<()> {
        let mut workbook = Workbook::new();
        let properties = DocProperties::new().set_author("picarus");
        workbook.set_properties(&properties);

        let worksheet = workbook.add_worksheet().set_name("111")?;

        let items = self
            .products_by_recipe_id
            .values()
            .filter_map(|f| {
                lz4_flex::block::decompress_size_prepended(&f.borrow().debug)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<IndexMap<SharedString, DebugValue>>(&b).ok())
            })
            .collect::<Vec<_>>();

        for (row, item) in items.iter().enumerate() {
            for (col, (_, value)) in item.iter().enumerate() {
                match value {
                    DebugValue::String(string) => worksheet.write(row as u32 + 1, col as u16, string.as_str()),
                    DebugValue::Float(float) => worksheet.write(row as u32 + 1, col as u16, *float),
                }?;
            }
        }

        if let Some(first) = items.first() {
            let table = Table::new().set_columns(&first.keys().map(|key| TableColumn::new().set_header(key.as_str())).collect::<Vec<_>>());

            // Add the table to the worksheet.
            worksheet.add_table(0, 0, items.len() as u32, first.len() as u16, &table)?;
            worksheet.set_freeze_panes(1, 0)?;
        }
        workbook.save(path)?;

        Ok(())
    }
}

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

pub trait AsyncBufReadExtReadString: AsyncBufReadExt + Unpin {
    async fn read_string(&mut self, format: DataFormat) -> Result<SharedString>
    where
        Self: Sized,
    {
        match format {
            DataFormat::String => read_c_string(self).await,
            DataFormat::WideString => read_wide_c_string(self).await,
        }
    }
}

// Implement for all types that satisfy the bounds
impl<T: AsyncBufReadExt + Unpin> AsyncBufReadExtReadString for T {}

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

async fn read_definitions(reader: &mut BufReader<Cursor<&[u8]>>) -> Result<IndexMap<SharedString, TagType>> {
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
        definitions.insert(SharedString::new(key), tag_type);
    }
    //debug!(?definitions);
    Ok(definitions)
}

async fn read_item_count(reader: &mut BufReader<Cursor<&[u8]>>) -> Result<usize> {
    let item_count = reader.read_u16_le().await? as usize;
    Ok(item_count)
}

async fn read_offsets(reader: &mut BufReader<Cursor<&[u8]>>, item_count: usize, tag_count: usize) -> Result<Vec<u32>> {
    let mut offsets = Vec::with_capacity(item_count * tag_count + 1);
    for _ in 0..=item_count * tag_count {
        let len = reader.read_u32_le().await?;
        offsets.push(len);
    }
    Ok(offsets)
}

pub trait ReadableItem: Sized + Default {
    const FORMAT: DataFormat;
    type Key: Eq + std::hash::Hash;

    fn key(item: &Self) -> Self::Key;

    async fn read_all<R: std::io::Read + std::io::Seek>(gamedatas_zip: &mut ZipArchive<R>, data_path: &str) -> Result<HashMap<Self::Key, Self>> {
        let mut file = gamedatas_zip.by_path(data_path)?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;

        let mut reader = BufReader::new(Cursor::new(data.as_slice()));

        let definitions = read_definitions(&mut reader).await?;
        let item_count = read_item_count(&mut reader).await?;
        let offsets = read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut items = HashMap::with_capacity(item_count);

        for idx in 0..item_count {
            let item = Self::default().read(&mut reader, &offsets, idx, &definitions, global_offset).await?;
            items.insert(Self::key(&item), item);
        }

        Ok(items)
    }

    async fn parse_debug<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Vec<u8>> {
        let tag_count = definitions.len();
        let mut debug = IndexMap::with_capacity(tag_count);
        for (tag_idx, (key, value_type)) in definitions.iter().enumerate() {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match Self::FORMAT {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset)).await?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2)).await?;
                }
            };

            match value_type {
                TagType::String => debug.insert(key.clone(), DebugValue::String(reader.read_string(Self::FORMAT).await?)),
                TagType::Float => debug.insert(key.clone(), DebugValue::Float(reader.read_f32_le().await?)),
            };
        }
        Ok(lz4_flex::block::compress_prepend_size(&serde_json::to_vec_pretty(&debug)?))
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self>;
}
