pub mod item_option;
pub mod item_quality;
pub mod item_res;
pub mod item_set;
pub mod items;
pub mod locale;
pub mod product;
pub mod random_box_group;
pub mod random_box_probability;
pub mod tempering;

pub mod binding;
pub mod common;
pub mod effects;
pub mod evolution;
pub mod filters;
pub mod fishing;
pub mod game_class;
pub mod grade;
pub mod quality;
pub mod skill;
pub mod synthesis_fellows;
pub mod synthesis_parts;
use anyhow::Result;

use encoding_rs::EUC_KR;

use gpui_kit::{AsyncWindowContext, Entity, Image, SharedString};

use image::{ImageReader, imageops::FilterType};
use indexmap::IndexMap;

use itertools::Itertools;
use rust_xlsxwriter::{DocProperties, Table, TableColumn, workbook::Workbook};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::File,
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Duration,
};
use strum::IntoEnumIterator;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    time::Instant,
};
use tracing::{error, warn};
use zip::ZipArchive;

use crate::{
    game_data::{
        effects::{EffectKind, ItemMinMaxEffect},
        evolution::Evolution,
        fishing::Fishing,
        game_class::GameClass,
        grade::Grade,
        item_option::ItemOption,
        item_quality::ItemQuality,
        item_res::ItemRes,
        item_set::ItemSet,
        items::{
            Item, ItemTrait, ItemType, ReadableItem, accessory::Accessory, armor::Armor, bag::Bag, boost::Boost, bracelet::Bracelet,
            consume::Consume, elluns::Elluns, event::Event, exchange::Exchange, fellow::Fellow, fellow_book::FellowBook,
            fellow_consume::FellowConsume, fellow_equip::FellowEquip, fellow_style::FellowStyle, gem::Gem, material::Material, package::Package,
            quest::Quest, random_box::RandomBox, recipe::Recipe, relic::Relic, sealed_fellow::SealedFellow, secondary_weapon::SecondaryWeapon,
            skill_book::SkillBook, style::Style, weapon::Weapon,
        },
        locale::Locale,
        product::Product,
        quality::Quality,
        random_box_group::RandomBoxGroup,
        random_box_probability::RandomBoxProbability,
        skill::Skill,
        synthesis_fellows::SynthesisFellows,
        synthesis_parts::SynthesisParts,
        tempering::Tempering,
    },
    game_data_view::GameDataLoadingStatus,
};

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

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DebugValue {
    String(SharedString),
    Float(f32),
}

#[derive(Default, Clone)]
pub struct GameData {
    pub items: IndexMap<SharedString, Rc<RefCell<Item>>>,
    effects_by_grade: HashMap<Grade, HashMap<u16, ItemOption>>,
    tempering_by_types: HashMap<SharedString, HashMap<u16, Tempering>>,
    quality_by_types: HashMap<SharedString, HashMap<u16, ItemQuality>>,
    random_box_groups: HashMap<SharedString, Rc<RefCell<RandomBoxGroup>>>,
    products: Vec<Rc<RefCell<Product>>>,
    icon_cache: HashMap<String, Arc<Image>>,
    pub elapsed: Duration,
    unknown_ids: BTreeMap<SharedString, u32>,
    unknown_effects: BTreeMap<SharedString, BTreeSet<SharedString>>,
    unknown_icons: BTreeMap<SharedString, BTreeSet<SharedString>>,
    unknown_skills: BTreeSet<SharedString>,
    skills: HashMap<SharedString, Skill>,
    pub fishing: Vec<Fishing>,
}

impl GameData {
    pub fn get_all_effects(&self) -> BTreeSet<SharedString> {
        self.items
            .iter()
            .flat_map(|(_, item)| item.borrow().get_unique_effects())
            .into_iter()
            .filter_map(|e| match e {
                EffectKind::Common { id: _, effect } => effect.parsed.as_ref().map(|(key, _)| key.clone()),
                EffectKind::MinMaxNoStep { id: _, effect } => effect.parsed.as_ref().map(|(key, _, _)| key.clone()),
                EffectKind::MinMaxStep { id: _, effect } => effect.parsed.as_ref().map(|(key, _, _, _)| key.clone()),
            })
            .collect()
    }

    fn validate_effects(&mut self) {
        self.unknown_effects = self
            .items
            .iter()
            .flat_map(|(_, item)| item.borrow().get_unique_effects())
            .filter_map(|e| match e {
                EffectKind::Common { id, effect } => {
                    if effect.parsed.is_none() {
                        effect.intermediate_effect.as_ref().map(|e| (e.clone(), id.clone()))
                    } else {
                        None
                    }
                }
                EffectKind::MinMaxNoStep { id, effect } => {
                    if effect.parsed.is_none() {
                        effect.intermediate_effect.as_ref().map(|e| (e.clone(), id.clone()))
                    } else {
                        None
                    }
                }
                EffectKind::MinMaxStep { id, effect } => {
                    if effect.parsed.is_none() {
                        effect.intermediate_effect.as_ref().map(|e| (e.clone(), id.clone()))
                    } else {
                        None
                    }
                }
            })
            .fold(BTreeMap::new(), |mut acc, (key, id)| {
                acc.entry(key).or_insert_with(BTreeSet::new).insert(id);
                acc
            });
        if !self.unknown_effects.is_empty() {
            warn!(unknown_effects_len = ?&self.unknown_effects.len(), unknown_effects = ?self.unknown_effects);
        }
    }

    pub fn get_random_effects(
        &self,
        grade: Grade,
        item_level: u16,
        usable_class: &BTreeSet<GameClass>,
        item_sub_type: &str,
    ) -> Option<Vec<ItemMinMaxEffect>> {
        self.effects_by_grade
            .get(&grade)
            .and_then(|effects| effects.get(&item_level))
            .and_then(|e| e.get_random_effects(usable_class, &item_sub_type))
    }

    pub fn get_quality_effect(&self, item_type: SharedString, item_level: u16, quality: Option<Quality>) -> Option<f32> {
        let quality = quality?;
        self.quality_by_types
            .get(&item_type)
            .and_then(|quality| quality.get(&item_level))
            .and_then(|f| match quality {
                Quality::Simple => None,
                Quality::Good => f.intermediate_fixed_effect.as_ref().and_then(|f| f.parsed.as_ref()).map(|f| f.1),
                Quality::Perfect => f.advanced_fixed_effect.as_ref().and_then(|f| f.parsed.as_ref()).map(|f| f.1),
            })
    }

    pub fn get_tempering_effect(&self, item_full_type: SharedString, item_level: u16) -> Option<&Tempering> {
        self.tempering_by_types
            .get(&item_full_type)
            .and_then(|tempering| tempering.get(&item_level))
    }

    pub async fn load(game_path: &str, on_load: &Entity<GameDataLoadingStatus>, cx: &mut AsyncWindowContext) -> Result<Self> {
        let start = Instant::now();
        let gamedatas = File::open(Path::new(game_path).join(r"Game\gamedatas.npk"))?;
        let gamelibs = File::open(Path::new(game_path).join(r"Game\gamelibs.npk"))?;
        let mut gamedatas_zip = ZipArchive::new(gamedatas)?;
        let mut gamelibs_zip = ZipArchive::new(gamelibs)?;
        let mut data = Self::default();

        let locales = Self::load_all_locales(&mut gamedatas_zip, on_load, cx).await?;

        data.load_fishing(&mut gamedatas_zip, &locales, on_load, cx).await?;

        let icons = gamelibs_zip
            .file_names()
            .filter(|f| f.starts_with("libs/ui/resources/textures/slot_icons/"))
            .map(|f| (f.to_lowercase(), f.to_string()))
            .collect::<HashMap<_, _>>();
        data.load_skills(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        let item_set = Self::load_itemset(&mut gamedatas_zip, &locales, on_load, cx).await?;

        data.load_product_materials(&mut gamedatas_zip, on_load, cx).await?; // always first
        data.load_recipes(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?; // always right after products

        let random_box_probabilities = Self::load_random_box_probabilities(&mut gamedatas_zip, on_load, cx).await?;
        data.load_random_box_groups(&mut gamedatas_zip, on_load, cx).await?;
        data.load_boosts(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        let item_set_fellow = Self::load_itemset_fellow(&mut gamedatas_zip, &locales, on_load, cx).await?;

        data.load_consumes(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_fellow_consumes(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_fellow_equips(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, &item_set_fellow, on_load, cx)
            .await?;
        data.load_elluns(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_exchanges(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_quests(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_braceletes(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_gems(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_bags(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_fellow_books(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_skill_books(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_fellows(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_events(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_relics(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_sealed_fellows(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_weapons(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, &item_set, on_load, cx)
            .await?;
        data.load_accessory(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, &item_set, on_load, cx)
            .await?;
        data.load_secondary_weapons(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, &item_set, on_load, cx)
            .await?;
        data.load_armors(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, &item_set, on_load, cx)
            .await?;
        data.load_styles(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, &item_set, on_load, cx)
            .await?;
        data.load_fellow_styles(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, &item_set, on_load, cx)
            .await?;
        data.load_materials(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;

        data.load_temperings(
            data.items.iter().filter_map(|(_, item)| item.borrow().get_full_type()).collect(),
            &mut gamedatas_zip,
            on_load,
            cx,
        )
        .await?;
        data.load_options(&mut gamedatas_zip, on_load, cx).await?;

        data.load_qualites(
            data.items.iter().filter_map(|(_, item)| item.borrow().get_type()).collect(),
            &mut gamedatas_zip,
            on_load,
            cx,
        )
        .await?;
        data.load_random_boxes(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        data.load_packages(&mut gamedatas_zip, &mut gamelibs_zip, &locales, &icons, on_load, cx)
            .await?;
        let evolution = data.load_evolutuion(&mut gamedatas_zip, on_load, cx).await?; // always last
        let synthesis_parts = data.load_synthesis_parts(&mut gamedatas_zip, on_load, cx).await?; // always last
        let synthesis_fellows = data.load_synthesis_fellows(&mut gamedatas_zip, on_load, cx).await?;
        for (_, item) in data.items.iter() {
            if let Item::Package(package) = &mut *item.borrow_mut() {
                package.set_package_contents(&data.items, &mut data.unknown_ids);
            }
            item.borrow_mut().set_fishing_drop(&data.fishing);
            item.borrow_mut().set_evolution(&evolution);
            item.borrow_mut().set_synthesis_parts(&synthesis_parts);
            item.borrow_mut().set_synthesis_fellows(&synthesis_fellows);
        }
        for item in &data.products {
            item.borrow_mut().set_materials(&data.items, &mut data.unknown_ids);
        }

        for (_, item) in &data.random_box_groups {
            item.borrow_mut().set_items(&data.items, &random_box_probabilities, &mut data.unknown_ids);
        }

        if !data.unknown_ids.is_empty() {
            warn!(unknown_ids_len = data.unknown_ids.len(), unknown_ids = ?data.unknown_ids);
        }

        if !data.unknown_icons.is_empty() {
            warn!(unknown_icons_len = data.unknown_icons.len(), unknown_icons = ?data.unknown_icons);
        }

        if !data.unknown_skills.is_empty() {
            warn!(unknown_skills_len = data.unknown_skills.len(), unknown_skills = ?data.unknown_skills);
        }

        data.validate_effects();

        data.elapsed = start.elapsed();
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

    async fn load_all_locales<R: Read + std::io::Seek>(
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<HashMap<SharedString, Locale>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Locales;
            cx.notify();
        });
        let mut locales = HashMap::new();

        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_skill.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_skill_fellow.sxb").await?);

        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_map.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_setitem.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_armor.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_weapon.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_accessory.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_style.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_fellowstyle.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_subitem.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_fellowequip.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_package.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_randombox.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_boost.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_fellow.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_material.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_fellowconsume.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_consume.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_fellowbook.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_event.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_bag.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_exchange.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_relic.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_quest.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_sealedfellow.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_enchantstone.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_skillbook.sxb").await?);
        locales.extend(Self::load_locales(gamedatas_zip, r"gamedata\localized\localstringdata_item_recipe.sxb").await?);
        Ok(locales)
    }

    async fn load_skills<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Skill;
            cx.notify();
        });

        let paths = gamedatas_zip
            .file_names()
            .filter(|f| f.starts_with("gamedata/adataxml/skill/") && f.ends_with(".xml"))
            .map(|f| f.to_string())
            .collect::<Vec<_>>();

        for path in paths {
            match Skill::load(gamedatas_zip, &path).await {
                Ok(mut skill) => {
                    skill.set_locale(&locales);
                    skill.set_effects();
                    skill.set_icon(gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons).await?;
                    self.skills.insert(skill.recid.clone(), skill);
                }
                Err(e) => warn!(?e, ?path, "Failed to parse skill"),
            }
        }
        Ok(())
    }

    async fn load_itemset<R: Read + std::io::Seek>(
        gamedatas_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<Vec<ItemSet>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::ItemSet;
            cx.notify();
        });

        let mut items = ItemSet::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemset_setcharacter.bin").await?;
        for item in items.iter_mut() {
            item.set_locale(&locales);
            item.set_effects_skill_locale(&locales);
        }
        Ok(items)
    }

    async fn load_fishing<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Fishing;
            cx.notify();
        });

        let mut items = Fishing::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fishing_timegroup.bin").await?;
        for item in items.iter_mut() {
            item.set_map_locale(locales);
        }
        self.fishing = items;
        Ok(())
    }

    async fn load_evolutuion<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<Vec<Evolution>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Evolution;
            cx.notify();
        });
        let mut items = Evolution::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fellowcompose_evolvegroup.bin").await?;
        for item in items.iter_mut() {
            item.set_fellows(&self.items, &mut self.unknown_ids);
        }
        Ok(items)
    }

    async fn load_synthesis_parts<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<Vec<SynthesisParts>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Synthesis;
            cx.notify();
        });
        let mut items = SynthesisParts::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fellowcompose_hopecompose.bin").await?;
        for item in items.iter_mut() {
            item.set_parts(&self.items, &mut self.unknown_ids);
        }
        Ok(items)
    }

    async fn load_synthesis_fellows<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<Vec<SynthesisFellows>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Synthesis;
            cx.notify();
        });
        SynthesisFellows::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fellowcompose_graderesult.bin").await
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
        self.products = Product::read_all_vec(gamedatas_zip, r"gamedata\adatabin\productdata_productmaterial.bin").await?;

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
        self.random_box_groups = RandomBoxGroup::read_all(gamedatas_zip, r"gamedata\adatabin\randomboxtable_randomboxgroup.bin").await?;

        Ok(())
    }

    async fn load_itemset_fellow<R: Read + std::io::Seek>(
        gamedatas_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<Vec<ItemSet>> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::ItemSet;
            cx.notify();
        });

        let mut items = ItemSet::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemset_setfellow.bin").await?;
        for item in items.iter_mut() {
            item.set_locale(&locales);
            item.set_effects_skill_locale(&locales);
        }
        Ok(items)
    }

    async fn load_armors<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Armor;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_armor.bin").await?;

        let items = Armor::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_armor.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Armor(item))));
        }

        Ok(())
    }

    async fn load_weapons<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Weapon;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_weapon.bin").await?;

        let items = Weapon::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_weapon.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Weapon(item))));
        }

        Ok(())
    }

    async fn load_accessory<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Accessory;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_accessory.bin").await?;

        let items = Accessory::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_accessory.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Accessory(item))));
        }

        Ok(())
    }

    async fn load_styles<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Style;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_style.bin").await?;

        let items = Style::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_style.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Style(item))));
        }

        Ok(())
    }

    async fn load_fellow_styles<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::FellowStyle;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_fellowstyle.bin").await?;

        let items = FellowStyle::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_fellowstyle.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Rc::new(RefCell::new(Item::FellowStyle(item))));
        }

        Ok(())
    }

    async fn load_secondary_weapons<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SecondaryWeapon;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_sub.bin").await?;

        let items = SecondaryWeapon::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_sub.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Rc::new(RefCell::new(Item::SecondaryWeapon(item))));
        }

        Ok(())
    }

    async fn load_fellow_equips<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        item_set: &Vec<ItemSet>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::FellowEquip;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_fellowequip.bin").await?;

        let items = FellowEquip::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_fellowequip.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Rc::new(RefCell::new(Item::FellowEquip(item))));
        }

        Ok(())
    }

    async fn load_packages<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Package;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_package.bin").await?;

        let items = Package::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_package.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);

            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Package(item))));
        }

        Ok(())
    }

    async fn load_random_boxes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::RandomBox;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_randombox.bin").await?;

        let items = RandomBox::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_randombox.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_random_box_group(&self.random_box_groups);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::RandomBox(item))));
        }

        Ok(())
    }

    async fn load_boosts<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Boost;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_boost.bin").await?;

        let items = Boost::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_boost.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Boost(item))));
        }

        Ok(())
    }

    async fn load_fellows<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Fellow;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\fellow_res.bin").await?;

        let items = Fellow::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fellow_state.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;
            item.set_skills(&self.skills, locales, &mut self.unknown_skills)?;
            item.set_region_locale(&locales);
            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Fellow(item))));
        }

        Ok(())
    }

    async fn load_materials<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Material;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_material.bin").await?;

        let items = Material::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_material.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Material(item))));
        }

        Ok(())
    }

    async fn load_fellow_consumes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::FellowConsume;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_fellowconsume.bin").await?;

        let items = FellowConsume::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_fellowconsume.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::FellowConsume(item))));
        }

        Ok(())
    }

    async fn load_consumes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Consume;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_consume.bin").await?;

        let items = Consume::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_consume.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;
            item.set_skills(&self.skills, locales, &mut self.unknown_skills)?;
            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Consume(item))));
        }

        Ok(())
    }

    async fn load_fellow_books<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::FellowBook;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_fellowbook.bin").await?;

        let items = FellowBook::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_fellowbook.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::FellowBook(item))));
        }

        Ok(())
    }

    async fn load_events<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Event;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_event.bin").await?;

        let items = Event::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_event.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Event(item))));
        }

        Ok(())
    }

    async fn load_elluns<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Elluns;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_ruby.bin").await?;

        let items = Elluns::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_ruby.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Elluns(item))));
        }

        Ok(())
    }

    async fn load_bags<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Bag;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_bag.bin").await?;

        let items = Bag::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_bag.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Bag(item))));
        }

        Ok(())
    }

    async fn load_exchanges<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Exchange;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_exchange.bin").await?;

        let items = Exchange::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_exchange.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Exchange(item))));
        }

        Ok(())
    }

    async fn load_relics<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Relic;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_relic.bin").await?;

        let items = Relic::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_relic.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Relic(item))));
        }

        Ok(())
    }

    async fn load_braceletes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Bracelet;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_bracelet.bin").await?;

        let items = Bracelet::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_bracelet.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Bracelet(item))));
        }

        Ok(())
    }

    async fn load_quests<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Quest;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_quest.bin").await?;

        let items = Quest::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_quest.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Quest(item))));
        }

        Ok(())
    }

    async fn load_sealed_fellows<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SealedFellow;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_sealedfellow.bin").await?;

        let items = SealedFellow::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_sealedfellow.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::SealedFellow(item))));
        }

        Ok(())
    }

    async fn load_gems<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Gem;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_enchantstone.bin").await?;

        let items = Gem::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_enchantstone.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Gem(item))));
        }

        Ok(())
    }

    async fn load_skill_books<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::SkillBook;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_skillbook.bin").await?;

        let items = SkillBook::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_skillbook.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::SkillBook(item))));
        }

        Ok(())
    }

    async fn load_recipes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        gamelibs_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
        icons: &HashMap<String, String>,
        on_load: &Entity<GameDataLoadingStatus>,
        cx: &mut AsyncWindowContext,
    ) -> Result<()> {
        on_load.update(cx, |this, cx| {
            *this = GameDataLoadingStatus::Recipe;
            cx.notify();
        });

        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_recipe.bin").await?;

        let items = Recipe::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_recipe.bin").await?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.set_product(&self.products);
            item.common
                .set_icon(&res, gamelibs_zip, icons, &mut self.icon_cache, &mut self.unknown_icons)
                .await?;

            self.items.insert(item.key(), Rc::new(RefCell::new(Item::Recipe(item))));
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

    pub async fn export_xlsx(&self, path: PathBuf) -> Result<()> {
        let mut workbook = Workbook::new();
        let properties = DocProperties::new().set_author("picarus");
        workbook.set_properties(&properties);

        let mut items_data: Vec<(ItemType, Vec<Vec<u8>>)> = Vec::new();

        for (item_type, group) in self.items.iter().into_group_map_by(|(_, value)| value.borrow().item_type()) {
            let mut debug_data = Vec::with_capacity(group.len());
            for (_, item) in group {
                debug_data.push(item.borrow().debug().to_vec());
            }
            items_data.push((item_type, debug_data));
        }

        tokio::task::spawn_blocking(move || -> Result<()> {
            for (item_type, group) in items_data {
                let worksheet = workbook.add_worksheet().set_name(item_type.locale())?;
                let mut items = Vec::with_capacity(group.len());
                for item in group {
                    let decompressed = lz4_flex::block::decompress_size_prepended(&item)?;
                    items.push(serde_json::from_slice::<IndexMap<SharedString, DebugValue>>(&decompressed)?);
                }
                let first = items.first().cloned();
                let length = items.len();
                for (row, item) in items.into_iter().enumerate() {
                    for (col, (_, value)) in item.into_iter().enumerate() {
                        match value {
                            DebugValue::String(string) => worksheet.write(row as u32 + 1, col as u16, string.as_str()),
                            DebugValue::Float(float) => worksheet.write(row as u32 + 1, col as u16, float),
                        }?;
                    }
                }

                if let Some(first) = first {
                    let table = Table::new().set_columns(&first.keys().map(|key| TableColumn::new().set_header(key.as_str())).collect::<Vec<_>>());

                    // Add the table to the worksheet.
                    worksheet.add_table(0, 0, length as u32, first.len() as u16, &table)?;
                    worksheet.set_freeze_panes(1, 0)?;
                }
            }

            workbook.save(path)?;
            Ok(())
        })
        .await??;
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

    Ok(std::sync::Arc::new(Image::from_bytes(gpui_kit::ImageFormat::Jpeg, data)))
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
