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

pub mod achievement_title;
pub mod binding;
pub mod common;
pub mod effects;
pub mod evolution;
pub mod fellow_combination;
pub mod filters;
pub mod fishing;
pub mod game_class;
pub mod grade;
pub mod quality;
pub mod reverse_tempering_rate;
pub mod skill;
pub mod synthesis_fellows;
pub mod synthesis_parts;
pub mod test;
use anyhow::Result;

use encoding_rs::EUC_KR;

use gpui_kit::{Image, SharedString};
use image::{ImageReader, imageops::FilterType};
use indexmap::IndexMap;
use rayon::prelude::*;

use itertools::Itertools;
use parking_lot::Mutex;
use rust_xlsxwriter::{DocProperties, Table, TableColumn, workbook::Workbook};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::File,
    io::{BufRead, Cursor, Read, Seek},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use strum::IntoEnumIterator;
use tokio::time::Instant;
use tracing::{error, warn};
use zip::ZipArchive;

use crate::game_data::{
    achievement_title::AchievementTitle,
    effects::{EffectKind, ItemMinMaxEffect},
    evolution::Evolution,
    fellow_combination::FellowCombination,
    fishing::Fishing,
    game_class::GameClass,
    grade::Grade,
    item_option::ItemOption,
    item_quality::ItemQuality,
    item_res::ItemRes,
    item_set::ItemSet,
    items::{
        Item, ItemTrait, ItemType, ReadableItem, accessory::Accessory, armor::Armor, bag::Bag, boost::Boost, bracelet::Bracelet, consume::Consume,
        elluns::Elluns, event::Event, exchange::Exchange, fellow::Fellow, fellow_book::FellowBook, fellow_consume::FellowConsume,
        fellow_equip::FellowEquip, fellow_style::FellowStyle, gem::Gem, material::Material, monster::Monster, package::Package, quest::Quest,
        random_box::RandomBox, recipe::Recipe, relic::Relic, sealed_fellow::SealedFellow, secondary_weapon::SecondaryWeapon, skill_book::SkillBook,
        style::Style, weapon::Weapon,
    },
    locale::Locale,
    product::Product,
    quality::Quality,
    random_box_group::RandomBoxGroup,
    random_box_probability::RandomBoxProbability,
    reverse_tempering_rate::ReverseTemperingRate,
    skill::Skill,
    synthesis_fellows::SynthesisFellows,
    synthesis_parts::SynthesisParts,
    tempering::Tempering,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum DebugValue {
    String(SharedString),
    Float(f32),
}

#[derive(Default, Clone)]
pub struct GameData {
    items: IndexMap<SharedString, Item>,
    pub shared: IndexMap<SharedString, Arc<Item>>,
    effects_by_grade: HashMap<Grade, HashMap<u16, ItemOption>>,
    tempering_by_types: HashMap<SharedString, HashMap<u16, Tempering>>,
    quality_by_types: HashMap<SharedString, HashMap<u16, ItemQuality>>,
    random_box_groups: HashMap<SharedString, Arc<RandomBoxGroup>>,
    products: Vec<Arc<Mutex<Product>>>,

    pub elapsed: Duration,

    pub fishing: Vec<Fishing>,
}

impl GameData {
    pub fn get_all_effects(effects: &Vec<EffectKind>) -> BTreeSet<SharedString> {
        effects
            .iter()
            .filter_map(|e| match e {
                EffectKind::Common { id: _, effect } => effect.parsed.as_ref().map(|(key, _)| key.clone()),
                EffectKind::MinMaxNoStep { id: _, effect } => effect.parsed.as_ref().map(|(key, _, _)| key.clone()),
                EffectKind::MinMaxStep { id: _, effect } => effect.parsed.as_ref().map(|(key, _, _, _)| key.clone()),
            })
            .collect()
    }

    fn validate_effects(effects: &Vec<EffectKind>) {
        let unknown_effects = effects
            .iter()
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
        if !unknown_effects.is_empty() {
            warn!(unknown_effects_len = ?&unknown_effects.len(), unknown_effects = ?unknown_effects);
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

    pub fn load(game_path: String) -> Result<(Self, BTreeSet<SharedString>)> {
        let start = Instant::now();
        let gamedatas = File::open(Path::new(&game_path).join(r"Game\gamedatas.npk"))?;
        let gamelibs = File::open(Path::new(&game_path).join(r"Game\gamelibs.npk"))?;
        let mut gamedatas_zip = ZipArchive::new(gamedatas)?;
        let mut gamelibs_zip = ZipArchive::new(gamelibs)?;

        let mut unknown_icons: BTreeMap<SharedString, BTreeSet<SharedString>> = BTreeMap::new();
        let mut unknown_skills: BTreeSet<SharedString> = BTreeSet::new();

        let mut data = Self::default();

        let locales = Self::load_all_locales(&mut gamedatas_zip)?;

        data.load_fishing(&mut gamedatas_zip, &locales)?;

        let icons = data.load_icons(&mut gamelibs_zip)?;

        let skills: HashMap<SharedString, Skill> = data.load_skills(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        let item_set = Self::load_itemset(&mut gamedatas_zip, &locales)?;

        data.load_product_materials(&mut gamedatas_zip)?; // always first
        data.load_recipes(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?; // always right after products

        data.load_random_box_groups(&mut gamedatas_zip)?;
        data.load_boosts(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        let item_set_fellow = Self::load_itemset_fellow(&mut gamedatas_zip, &locales)?;

        data.load_consumes(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons, &skills, &mut unknown_skills)?;
        data.load_fellow_consumes(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_fellow_equips(&mut gamedatas_zip, &locales, &item_set_fellow, &icons, &mut unknown_icons)?;
        data.load_elluns(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_exchanges(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_quests(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_braceletes(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_gems(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_bags(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_fellow_books(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_skill_books(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_fellows(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons, &skills, &mut unknown_skills)?;
        data.load_monsters(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons, &skills, &mut unknown_skills)?;
        data.load_events(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_relics(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_sealed_fellows(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_weapons(&mut gamedatas_zip, &locales, &item_set, &icons, &mut unknown_icons)?;
        data.load_accessory(&mut gamedatas_zip, &locales, &item_set, &icons, &mut unknown_icons)?;
        data.load_secondary_weapons(&mut gamedatas_zip, &locales, &item_set, &icons, &mut unknown_icons)?;
        data.load_armors(&mut gamedatas_zip, &locales, &item_set, &icons, &mut unknown_icons)?;
        data.load_styles(&mut gamedatas_zip, &locales, &item_set, &icons, &mut unknown_icons)?;
        data.load_fellow_styles(&mut gamedatas_zip, &locales, &item_set, &icons, &mut unknown_icons)?;
        data.load_materials(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;

        data.load_temperings(
            data.items.iter().filter_map(|(_, item)| item.get_full_type()).collect(),
            &mut gamedatas_zip,
        )?;
        data.load_options(&mut gamedatas_zip)?;

        data.load_qualites(data.items.iter().filter_map(|(_, item)| item.get_type()).collect(), &mut gamedatas_zip)?;
        data.load_random_boxes(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        data.load_packages(&mut gamedatas_zip, &locales, &icons, &mut unknown_icons)?;
        let evolution = data.load_evolutuion(&mut gamedatas_zip)?; // always last
        let synthesis_parts = data.load_synthesis_parts(&mut gamedatas_zip)?; // always last
        let fellow_combinations = data.load_fellow_combination(&mut gamedatas_zip, &locales, &skills, &mut unknown_skills)?; // always last
        let synthesis_fellows = data.load_synthesis_fellows(&mut gamedatas_zip)?;
        for (_, item) in data.items.iter_mut() {
            item.set_fishing_drop(&data.fishing);
            item.set_evolution(&evolution);
            item.set_synthesis_parts(&synthesis_parts);
            item.set_synthesis_fellows(&synthesis_fellows);
            item.set_fellow_combinations(&fellow_combinations);
        }

        if !unknown_icons.is_empty() {
            warn!(unknown_icons_len = unknown_icons.len(), unknown_icons = ?unknown_icons);
        }

        if !unknown_skills.is_empty() {
            warn!(unknown_skills_len = unknown_skills.len(), unknown_skills = ?unknown_skills);
        }
        let effects = data.items.iter().flat_map(|(_, item)| item.get_unique_effects()).collect();
        //Self::validate_effects(&effects);
        let effects = Self::get_all_effects(&effects);
        data.shared = data.items.drain(..).map(|(k, v)| (k, Arc::new(v))).collect();
        data.elapsed = start.elapsed();
        Ok((data, effects))
    }

    fn load_locales<R: std::io::Read + std::io::Seek>(gamedatas_zip: &mut ZipArchive<R>, locale_path: &str) -> Result<HashMap<SharedString, Locale>> {
        Locale::read_all(gamedatas_zip, locale_path)
    }

    fn load_itemres<R: Read + std::io::Seek>(gamedatas_zip: &mut ZipArchive<R>, itemres_path: &str) -> Result<HashMap<SharedString, ItemRes>> {
        ItemRes::read_all(gamedatas_zip, itemres_path)
    }

    fn load_all_locales<R: Read + std::io::Seek>(gamedatas_zip: &mut ZipArchive<R>) -> Result<HashMap<SharedString, Locale>> {
        let mut locales = HashMap::new();

        let paths = gamedatas_zip
            .file_names()
            .flatten()
            .filter(|f| f.starts_with("gamedata/localized/") && f.ends_with(".sxb"))
            .map(|f| f.to_string())
            .collect::<Vec<_>>();

        for path in paths {
            locales.extend(Self::load_locales(gamedatas_zip, &path)?);
        }

        Ok(locales)
    }

    fn load_icons<R: Read + std::io::Seek>(&self, gamelibs_zip: &mut ZipArchive<R>) -> Result<HashMap<String, Arc<Image>>> {
        let paths = gamelibs_zip
            .file_names()
            .flatten()
            .filter(|f| f.starts_with("libs/ui/resources/textures/slot_icons/"))
            .map(|f| f.to_string())
            .collect::<Vec<_>>();
        let mut icons = HashMap::new();
        for chunk_paths in paths.chunks(512) {
            let mut raw = Vec::with_capacity(chunk_paths.len());
            for path in chunk_paths {
                let mut file = gamelibs_zip.by_path(path)?;
                let mut bytes = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut bytes)?;
                raw.push((path.clone(), bytes));
            }

            let parsed: HashMap<String, Arc<Image>> = raw
                .into_par_iter()
                .filter_map(|(path, bytes)| match dds_to_jpeg(bytes) {
                    Ok(icon) => Some((
                        Path::new(&path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(str::to_lowercase)
                            .unwrap_or_default(),
                        icon,
                    )),
                    Err(e) => {
                        error!(?e, ?path, "Failed to convert icon");
                        None
                    }
                })
                .collect();

            icons.extend(parsed);
        }

        Ok(icons)
    }

    fn load_skills<R: Read + std::io::Seek>(
        &self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<HashMap<SharedString, Skill>> {
        let paths = gamedatas_zip
            .file_names()
            .flatten()
            .filter(|f| f.starts_with("gamedata/adataxml/skill/") && f.ends_with(".xml"))
            .map(|f| f.to_string())
            .collect::<Vec<_>>();
        let mut skills = HashMap::new();
        for chunk_paths in paths.chunks(512) {
            let mut raw = Vec::with_capacity(chunk_paths.len());
            for path in chunk_paths {
                let mut file = gamedatas_zip.by_path(path)?;
                let mut bytes = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut bytes)?;
                raw.push((path.clone(), bytes));
            }

            let parsed: Vec<Skill> = raw
                .into_par_iter()
                .filter_map(|(path, bytes)| match Skill::load(&bytes) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        warn!(?e, ?path, "Failed to parse skill");
                        None
                    }
                })
                .collect();

            for mut skill in parsed {
                skill.set_locale(locales);
                skill.set_effects();
                skill.set_icon(icons, unknown_icons);
                skills.insert(skill.recid.clone(), skill);
            }
        }

        Ok(skills)
    }

    fn load_itemset<R: Read + std::io::Seek>(gamedatas_zip: &mut ZipArchive<R>, locales: &HashMap<SharedString, Locale>) -> Result<Vec<ItemSet>> {
        let mut items = ItemSet::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemset_setcharacter.bin")?;
        for item in items.iter_mut() {
            item.set_locale(&locales);
            item.set_effects_skill_locale(&locales);
        }
        Ok(items)
    }

    fn load_fishing<R: Read + std::io::Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>, locales: &HashMap<SharedString, Locale>) -> Result<()> {
        let mut items = Fishing::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fishing_timegroup.bin")?;
        for item in items.iter_mut() {
            item.set_map_locale(locales);
        }
        self.fishing = items;
        Ok(())
    }

    fn load_fellow_combination<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,

        skills: &HashMap<SharedString, Skill>,
        unknown_skills: &mut BTreeSet<SharedString>,
    ) -> Result<Vec<FellowCombination>> {
        let mut items = FellowCombination::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fellowfarm_fellowcombination.bin")?;
        for item in items.iter_mut() {
            //item.set_fellows(&self.items, unknown_ids);
            item.set_skills(skills, locales, unknown_skills)?;
        }
        Ok(items)
    }

    fn load_evolutuion<R: Read + std::io::Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<Vec<Evolution>> {
        let items = Evolution::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fellowcompose_evolvegroup.bin")?;
        Ok(items)
    }

    fn load_synthesis_parts<R: Read + std::io::Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<Vec<SynthesisParts>> {
        let items = SynthesisParts::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fellowcompose_hopecompose.bin")?;

        Ok(items)
    }

    fn load_synthesis_fellows<R: Read + std::io::Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<Vec<SynthesisFellows>> {
        SynthesisFellows::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fellowcompose_graderesult.bin")
    }

    fn load_product_materials<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<()> {
        self.products = Product::read_all_vec(gamedatas_zip, r"gamedata\adatabin\productdata_productmaterial.bin")?;

        Ok(())
    }

    fn load_random_box_groups<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<()> {
        let random_box_probabilities = RandomBoxProbability::read_all(gamedatas_zip, r"gamedata\adatabin\randomboxtable_randomboxprobability.bin")?;
        let random_box_groups = RandomBoxGroup::read_all(gamedatas_zip, r"gamedata\adatabin\randomboxtable_randomboxgroup.bin")?;
        let mut items = HashMap::new();
        for (id, mut item) in random_box_groups.into_iter() {
            item.set_probabilities(&random_box_probabilities);
            items.insert(id, Arc::new(item));
        }
        self.random_box_groups = items;
        Ok(())
    }

    fn load_itemset_fellow<R: Read + std::io::Seek>(
        gamedatas_zip: &mut ZipArchive<R>,
        locales: &HashMap<SharedString, Locale>,
    ) -> Result<Vec<ItemSet>> {
        let mut items = ItemSet::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemset_setfellow.bin")?;
        for item in items.iter_mut() {
            item.set_locale(&locales);
            item.set_effects_skill_locale(&locales);
        }
        Ok(items)
    }

    fn load_armors<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        item_set: &Vec<ItemSet>,
        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_armor.bin")?;

        let items = Armor::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_armor.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Item::Armor(item));
        }

        Ok(())
    }

    fn load_weapons<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        item_set: &Vec<ItemSet>,
        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_weapon.bin")?;

        let items = Weapon::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_weapon.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Item::Weapon(item));
        }

        Ok(())
    }

    fn load_accessory<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        item_set: &Vec<ItemSet>,
        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_accessory.bin")?;

        let items = Accessory::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_accessory.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Item::Accessory(item));
        }

        Ok(())
    }

    fn load_styles<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        item_set: &Vec<ItemSet>,
        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_style.bin")?;

        let items = Style::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_style.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Item::Style(item));
        }

        Ok(())
    }

    fn load_fellow_styles<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        item_set: &Vec<ItemSet>,
        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_fellowstyle.bin")?;

        let items = FellowStyle::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_fellowstyle.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common.set_icon(&res, icons, unknown_icons);
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Item::FellowStyle(item));
        }

        Ok(())
    }

    fn load_secondary_weapons<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        item_set: &Vec<ItemSet>,
        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_sub.bin")?;

        let items = SecondaryWeapon::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_sub.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_skill_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Item::SecondaryWeapon(item));
        }

        Ok(())
    }

    fn load_fellow_equips<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        item_set: &Vec<ItemSet>,
        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_fellowequip.bin")?;

        let items = FellowEquip::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_fellowequip.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common.set_icon(&res, icons, unknown_icons);
            item.common.set_item_set(item_set);
            self.items.insert(item.key(), Item::FellowEquip(item));
        }

        Ok(())
    }

    fn load_packages<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_package.bin")?;

        let items = Package::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_package.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);

            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Package(item));
        }

        Ok(())
    }

    fn load_random_boxes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_randombox.bin")?;

        let items = RandomBox::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_randombox.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_random_box_group(&self.random_box_groups);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::RandomBox(item));
        }

        Ok(())
    }

    fn load_boosts<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_boost.bin")?;

        let items = Boost::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_boost.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Boost(item));
        }

        Ok(())
    }

    fn load_fellows<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
        skills: &HashMap<SharedString, Skill>,
        unknown_skills: &mut BTreeSet<SharedString>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\fellow_res.bin")?;

        let items = Fellow::read_all_vec(gamedatas_zip, r"gamedata\adatabin\fellow_state.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common.set_icon(&res, icons, unknown_icons);
            item.set_skills(skills, locales, unknown_skills)?;
            item.set_region_locale(&locales);
            self.items.insert(item.key(), Item::Fellow(item));
        }

        Ok(())
    }

    fn load_materials<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_material.bin")?;

        let items = Material::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_material.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Material(item));
        }

        Ok(())
    }

    fn load_fellow_consumes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_fellowconsume.bin")?;

        let items = FellowConsume::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_fellowconsume.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::FellowConsume(item));
        }

        Ok(())
    }

    fn load_consumes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
        skills: &HashMap<SharedString, Skill>,
        unknown_skills: &mut BTreeSet<SharedString>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_consume.bin")?;
        let titles: HashMap<SharedString, AchievementTitle> = AchievementTitle::read_all(gamedatas_zip, r"gamedata\adatabin\achievement_title.bin")?;
        let reverse_rates: HashMap<SharedString, ReverseTemperingRate> =
            ReverseTemperingRate::read_all(gamedatas_zip, r"gamedata\adatabin\itemreinforceratetable_reversereinforcerate.bin")?;

        let items = Consume::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_consume.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);
            item.set_skills(skills, locales, unknown_skills)?;
            if let Some(title) = titles.get(&item.get_id()) {
                item.common.effects = title.effects.clone();
            }
            if let Some(rate) = reverse_rates.get(&item.get_id()) {
                item.reverse_tempering_rates = Some(rate.clone());
            }

            self.items.insert(item.key(), Item::Consume(item));
        }

        Ok(())
    }

    fn load_fellow_books<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_fellowbook.bin")?;

        let items = FellowBook::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_fellowbook.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::FellowBook(item));
        }

        Ok(())
    }

    fn load_events<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_event.bin")?;

        let items = Event::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_event.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Event(item));
        }

        Ok(())
    }

    fn load_elluns<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_ruby.bin")?;

        let items = Elluns::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_ruby.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Elluns(item));
        }

        Ok(())
    }

    fn load_bags<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_bag.bin")?;

        let items = Bag::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_bag.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Bag(item));
        }

        Ok(())
    }

    fn load_exchanges<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_exchange.bin")?;

        let items = Exchange::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_exchange.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Exchange(item));
        }

        Ok(())
    }

    fn load_relics<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_relic.bin")?;

        let items = Relic::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_relic.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Relic(item));
        }

        Ok(())
    }

    fn load_braceletes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_bracelet.bin")?;

        let items = Bracelet::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_bracelet.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Bracelet(item));
        }

        Ok(())
    }

    fn load_quests<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_quest.bin")?;

        let items = Quest::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_quest.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.set_description_locale(&locales);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Quest(item));
        }

        Ok(())
    }

    fn load_sealed_fellows<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_sealedfellow.bin")?;

        let items = SealedFellow::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_sealedfellow.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::SealedFellow(item));
        }

        Ok(())
    }

    fn load_gems<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_enchantstone.bin")?;

        let items = Gem::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_enchantstone.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Gem(item));
        }

        Ok(())
    }

    fn load_skill_books<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_skillbook.bin")?;

        let items = SkillBook::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_skillbook.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::SkillBook(item));
        }

        Ok(())
    }

    fn load_monsters<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
        skills: &HashMap<SharedString, Skill>,
        unknown_skills: &mut BTreeSet<SharedString>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\monster2_res.bin")?;
        let mut items = Monster::read_all_vec(gamedatas_zip, r"gamedata\adatabin\monster2_state.bin")?;
        items.extend(Monster::read_all_vec(gamedatas_zip, r"gamedata\adatabin\monster1_state.bin")?);
        items.extend(Monster::read_all_vec(gamedatas_zip, r"gamedata\adatabin\monster_state.bin")?);
        for mut item in items {
            item.common.set_locale(&locales);
            item.common.set_linked_recipes(&self.products);
            item.common.set_icon(&res, icons, unknown_icons);
            item.set_skills(skills, locales, unknown_skills)?;
            self.items.insert(item.key(), Item::Monster(item));
        }

        Ok(())
    }

    fn load_recipes<R: Read + std::io::Seek>(
        &mut self,
        gamedatas_zip: &mut ZipArchive<R>,

        locales: &HashMap<SharedString, Locale>,

        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let res = Self::load_itemres(gamedatas_zip, r"gamedata\adatabin\itemres_recipe.bin")?;

        let items = Recipe::read_all_vec(gamedatas_zip, r"gamedata\adatabin\itemdata_recipe.bin")?;
        for mut item in items {
            item.common.set_locale(&locales);
            item.set_product(&self.products);
            item.common.set_icon(&res, icons, unknown_icons);

            self.items.insert(item.key(), Item::Recipe(item));
        }

        Ok(())
    }

    fn load_temperings<R: Read + Seek>(&mut self, item_types: HashSet<SharedString>, gamedatas_zip: &mut ZipArchive<R>) -> Result<()> {
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
            ) {
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

    fn load_options<R: Read + Seek>(&mut self, gamedatas_zip: &mut ZipArchive<R>) -> Result<()> {
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
            ) {
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

    fn load_qualites<R: Read + Seek>(&mut self, item_types: HashSet<SharedString>, gamedatas_zip: &mut ZipArchive<R>) -> Result<()> {
        let mut quality_by_types = HashMap::new();
        for item_type in &item_types {
            match ItemQuality::read_all(gamedatas_zip, &format!(r"gamedata\adatabin\itemqualitytable_{}.bin", item_type)) {
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

        let mut items_data: Vec<(ItemType, Vec<Vec<u8>>)> = Vec::new();

        for (item_type, group) in self.shared.iter().into_group_map_by(|(_, value)| value.item_type()) {
            let mut debug_data = Vec::with_capacity(group.len());
            for (_, item) in group {
                debug_data.push(item.debug().to_vec());
            }
            items_data.push((item_type, debug_data));
        }

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
    }
}

fn dds_to_jpeg(bytes: Vec<u8>) -> Result<std::sync::Arc<Image>> {
    let cursor = Cursor::new(bytes);
    let img = ImageReader::new(cursor).with_guessed_format()?.decode()?;

    let img = if img.width() > 128 || img.height() > 128 {
        img.resize(128, 128, FilterType::Triangle)
    } else {
        img
    };

    let mut data = Vec::new();
    img.write_to(&mut Cursor::new(&mut data), image::ImageFormat::Jpeg)?;

    Ok(std::sync::Arc::new(Image::from_bytes(gpui_kit::ImageFormat::Jpeg, data)))
}

pub trait ReadExt: BufRead {
    fn read_u8(&mut self) -> Result<u8> {
        let mut b = [0u8; 1];
        self.read_exact(&mut b)?;
        Ok(b[0])
    }

    fn read_u16_le(&mut self) -> Result<u16> {
        let mut b = [0u8; 2];
        self.read_exact(&mut b)?;
        Ok(u16::from_le_bytes(b))
    }

    fn read_f32_le(&mut self) -> Result<f32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b)?;
        Ok(f32::from_le_bytes(b))
    }

    fn read_u32_le(&mut self) -> Result<u32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    fn read_string(&mut self, format: DataFormat) -> Result<SharedString> {
        match format {
            DataFormat::String => read_c_string(self),
            DataFormat::WideString => read_wide_c_string(self),
        }
    }
}

// Blanket impl: anything that is `Read` gets these methods.
impl<R: BufRead + ?Sized> ReadExt for R {}

thread_local! {
    static SCRATCH_U8: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(256));
    static SCRATCH_U16: RefCell<Vec<u16>> = RefCell::new(Vec::with_capacity(256));
}

fn read_c_string<R: BufRead + ?Sized>(reader: &mut R) -> Result<SharedString> {
    SCRATCH_U8.with(|scratch| -> Result<SharedString> {
        let mut buffer = scratch.borrow_mut();
        buffer.clear();

        reader.read_until(0, &mut buffer)?;

        if buffer.last() == Some(&0) {
            buffer.pop();
        }
        let (value, _, _) = EUC_KR.decode(&buffer);
        Ok(SharedString::new(value))
    })
}

fn read_wide_c_string<R: BufRead + ?Sized>(reader: &mut R) -> Result<SharedString> {
    SCRATCH_U16.with(|scratch| -> Result<SharedString> {
        let mut byte_buffer = scratch.borrow_mut();
        byte_buffer.clear();

        loop {
            let byte = reader.read_u16_le()?;
            if byte == 0 {
                break;
            }
            byte_buffer.push(byte);
        }

        Ok(SharedString::from(String::from_utf16_lossy(&byte_buffer)))
    })
}

fn read_definitions<R: BufRead + ?Sized>(reader: &mut R) -> Result<IndexMap<SharedString, TagType>> {
    let tag_count = reader.read_u16_le()? as usize;

    let mut definitions = IndexMap::with_capacity(tag_count);
    for _ in 0..tag_count {
        let type_id = reader.read_u8()?;
        let tag_type = match type_id {
            1 => TagType::String,
            0 => TagType::Float,
            _ => return Err(anyhow::anyhow!("Unknown tag type")),
        };
        let len = reader.read_u8()?;
        let mut value = vec![0; len as usize];
        reader.read_exact(&mut value)?;
        let (key, _, _) = EUC_KR.decode(&value);
        definitions.insert(SharedString::new(key), tag_type);
    }
    //tracing::debug!(?definitions);
    Ok(definitions)
}

fn read_item_count<R: BufRead + ?Sized>(reader: &mut R) -> Result<usize> {
    let item_count = reader.read_u16_le()? as usize;
    Ok(item_count)
}

fn read_offsets<R: BufRead + ?Sized>(reader: &mut R, item_count: usize, tag_count: usize) -> Result<Vec<u32>> {
    let mut offsets = Vec::with_capacity(item_count * tag_count + 1);
    for _ in 0..=item_count * tag_count {
        let len = reader.read_u32_le()?;
        offsets.push(len);
    }
    Ok(offsets)
}
