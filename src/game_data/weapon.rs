use std::{
    cell::RefCell, collections::{BTreeSet, HashMap}, io::{Read, SeekFrom}, rc::Rc, sync::Arc,
};

use crate::{
    game_data::{AbstractItem, Binding, DataFormat, GameClass, Grade, Item, ItemEffect, TagType, item_set::ItemSet, locale::Locale, product::Product}, language::LanguageController,
};
use anyhow::Result;

use gpui::{Image, SharedString};
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tracing::warn;

#[derive(Default, Serialize)]
pub struct Weapon {
    pub linked_recipes: BTreeSet<SharedString>,
    pub item_set: Option<ItemSet>,
    pub locale: Option<Locale>,
    pub skill_locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    pub id: SharedString,
    pub name: SharedString,
    for_development: f32,
    pub required_level: u8,
    level_restriction: f32,
    pub item_level: u16,
    pub usable_class: BTreeSet<GameClass>,
    randomly_applied_class: SharedString,
    usable_gender: SharedString,
    pub grade: Option<Grade>,
    pub equipment_slot: SharedString,
    pub weapon_type: SharedString,
    pub attack_range_type: SharedString,
    pub attribute_type: SharedString,
    gauge_increase_amount: f32,
    range: f32,
    air_range: f32,
    pub min_attack: f32,
    pub max_attack: f32,
    projectile_speed: f32,
    pub attack_speed: f32,
    durability: f32,
    purchase_price: f32,
    sell_price: f32,
    phys_accuracy: f32,
    magic_accuracy: f32,
    phys_crit_rate: f32,
    magic_crit_rate: f32,
    phys_crit_damage: f32,
    magic_crit_damage: f32,
    block_rate: f32,
    pub equip_effect_1: Option<ItemEffect>,
    pub equip_effect_2: Option<ItemEffect>,
    pub equip_effect_3: Option<ItemEffect>,
    pub equip_effect_4: Option<ItemEffect>,
    random_option_applied: f32,
    pub min_random_options: u8,
    pub max_random_options: u8,
    application_rate: f32,
    apply_slots_on_drop: f32,
    min_sealed_slots: f32,
    max_sealed_slots: f32,
    pub min_crafting_seal_slots: u8,
    pub max_crafting_seal_slots: u8,
    seal_slots_on_great_crafting_success: SharedString,
    min_variable_stat: f32,
    max_variable_stat: f32,
    sealed_slots_count: f32,
    soul_stone1: SharedString,
    soul_stone2: SharedString,
    soul_stone3: SharedString,
    pub enhancement_limit: u8,
    enhancement_chance: SharedString,
    cannot_drop: f32,
    pub cannot_trade: bool,
    pub cannot_sell: bool,
    pub indestructible: bool,
    level_check_on_drop: SharedString,
    pub binding: Option<Binding>,
    bind_target: SharedString,
    bind_removal_count: f32,
    usage_restriction: SharedString,
    dyeable: f32,
    auction_category: SharedString,
    skill_effect: Option<SharedString>,
    ignore_level_check_on_drop: f32,
    usage_period: f32,
    grade_up_code_key: SharedString,
    grade_up_result: SharedString,
    disassembly_id: SharedString,
    extraction_id: SharedString,
    use_level_option: f32,
    level_weight: f32,
    pub overrise_max: u8,
    overrise_id1: SharedString,
    overrise_id2: SharedString,
    overrise_id3: SharedString,
    contents_level: f32,
    not_usable_in_unified_channel: f32,
    pub reverse_enhancement_limit: u8,
    awaken_check: f32,
    awaken_bead1: SharedString,
    awaken_nead2: SharedString,
    level_up: f32,
    item_exp: f32,
    seal_stone_reequip_allowed: f32,
    weapon_charge: f32,
    max_charge: f32,
    overrise_material_id1: SharedString,
    overrise_material_id2: SharedString,
}

impl AbstractItem for Weapon {
    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<String, TagType>,
        global_offset: u64,
        format: DataFormat,
    ) -> Result<Self> {
        let tag_count = definitions.len();
        for tag_idx in 0..tag_count {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match format {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset)).await?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2)).await?;
                }
            };

            match tag_idx {
                // String fields
                0 => self.id = SharedString::new(Self::read_string(format, reader).await?.to_uppercase()),
                1 => self.name = Self::read_string(format, reader).await?,
                6 => {
                    let value = Self::read_string(format, reader).await?;

                    self.usable_class = value.split("_").filter_map(|c| GameClass::try_from(c).ok()).collect();
                }
                7 => self.randomly_applied_class = Self::read_string(format, reader).await?,
                8 => self.usable_gender = Self::read_string(format, reader).await?,
                10 => self.equipment_slot = Self::read_string(format, reader).await?,
                11 => self.weapon_type = SharedString::new(Self::read_string(format, reader).await?.to_lowercase()),
                12 => self.attack_range_type = Self::read_string(format, reader).await?,
                13 => self.attribute_type = Self::read_string(format, reader).await?,
                31 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_1 = Some(ItemEffect::new(effect));
                    }
                }
                32 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_2 = Some(ItemEffect::new(effect));
                    }
                }
                33 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_3 = Some(ItemEffect::new(effect));
                    }
                }
                34 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_4 = Some(ItemEffect::new(effect));
                    }
                }
                44 => self.seal_slots_on_great_crafting_success = Self::read_string(format, reader).await?,
                48 => self.soul_stone1 = Self::read_string(format, reader).await?,
                49 => self.soul_stone2 = Self::read_string(format, reader).await?,
                50 => self.soul_stone3 = Self::read_string(format, reader).await?,
                52 => self.enhancement_chance = Self::read_string(format, reader).await?,
                57 => self.level_check_on_drop = Self::read_string(format, reader).await?,
                58 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
                59 => self.bind_target = Self::read_string(format, reader).await?,
                61 => self.usage_restriction = Self::read_string(format, reader).await?,
                63 => self.auction_category = Self::read_string(format, reader).await?,
                64 => {
                    self.skill_effect = {
                        let effect_skill = Self::read_string(format, reader).await?.to_uppercase().replace(".", "_DESCRIPTION_");
                        if effect_skill != "*" {
                            Some(SharedString::new(effect_skill))
                        } else {
                            None
                        }
                    }
                }
                67 => self.grade_up_code_key = Self::read_string(format, reader).await?,
                68 => self.grade_up_result = Self::read_string(format, reader).await?,
                69 => self.disassembly_id = Self::read_string(format, reader).await?,
                70 => self.extraction_id = Self::read_string(format, reader).await?,
                74 => self.overrise_id1 = Self::read_string(format, reader).await?,
                75 => self.overrise_id2 = Self::read_string(format, reader).await?,
                76 => self.overrise_id3 = Self::read_string(format, reader).await?,
                81 => self.awaken_bead1 = Self::read_string(format, reader).await?,
                82 => self.awaken_nead2 = Self::read_string(format, reader).await?,
                88 => self.overrise_material_id1 = Self::read_string(format, reader).await?,
                89 => self.overrise_material_id2 = Self::read_string(format, reader).await?,

                _ => {
                    let value = reader.read_f32_le().await?;
                    // Assign to the appropriate field based on index
                    match tag_idx {
                        2 => self.for_development = value,
                        3 => self.required_level = value as u8,
                        4 => self.level_restriction = value,
                        5 => self.item_level = value as u16,
                        9 => {
                            self.grade = Grade::from_repr(value as u8);
                        }
                        14 => self.gauge_increase_amount = value,
                        15 => self.range = value,
                        16 => self.air_range = value,
                        17 => self.min_attack = value,
                        18 => self.max_attack = value,
                        19 => self.projectile_speed = value,
                        20 => self.attack_speed = value,
                        21 => self.durability = value,
                        22 => self.purchase_price = value,
                        23 => self.sell_price = value,
                        24 => self.phys_accuracy = value,
                        25 => self.magic_accuracy = value,
                        26 => self.phys_crit_rate = value,
                        27 => self.magic_crit_rate = value,
                        28 => self.phys_crit_damage = value,
                        29 => self.magic_crit_damage = value,
                        30 => self.block_rate = value,
                        35 => self.random_option_applied = value,
                        36 => self.min_random_options = value as u8,
                        37 => self.max_random_options = value as u8,
                        38 => self.application_rate = value,
                        39 => self.apply_slots_on_drop = value,
                        40 => self.min_sealed_slots = value,
                        41 => self.max_sealed_slots = value,
                        42 => self.min_crafting_seal_slots = value as u8,
                        43 => self.max_crafting_seal_slots = value as u8,
                        45 => self.min_variable_stat = value,
                        46 => self.max_variable_stat = value,
                        47 => self.sealed_slots_count = value,
                        51 => self.enhancement_limit = value as u8,
                        53 => self.cannot_drop = value,
                        54 => self.cannot_trade = value != 0.0,
                        55 => self.cannot_sell = value != 0.0,
                        56 => self.indestructible = value != 0.0,
                        60 => self.bind_removal_count = value,
                        62 => self.dyeable = value,
                        65 => self.ignore_level_check_on_drop = value,
                        66 => self.usage_period = value,
                        71 => self.use_level_option = value,
                        72 => self.level_weight = value,
                        73 => self.overrise_max = value as u8,
                        77 => self.contents_level = value,
                        78 => self.not_usable_in_unified_channel = value,
                        79 => self.reverse_enhancement_limit = value as u8,
                        80 => self.awaken_check = value,
                        83 => self.level_up = value,
                        84 => self.item_exp = value,
                        85 => self.seal_stone_reequip_allowed = value,
                        86 => self.weapon_charge = value,
                        87 => self.max_charge = value,
                        _ => {} // Should not happen
                    }
                }
            }
        }

        Ok(self)
    }
}

impl Weapon {
    pub fn get_skill_locale(&self) -> Option<SharedString> {
        let language = LanguageController::get_current_language();

        self.skill_locale
            .as_ref()
            .map(|f| match language {
                crate::settings::Language::English => f.eng.clone(),
                crate::settings::Language::Russian => f.rus.clone(),
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .or_else(|| self.skill_effect.clone())
    }
}

impl Item for Weapon {
    fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>, skill_locales: &HashMap<SharedString, Locale>) {
        self.locale = locales.get(&self.id).cloned();
        if let Some(skill) = self.skill_effect.as_ref() {
            self.skill_locale = skill_locales.get(skill).cloned();
            if self.skill_locale.is_none() {
                warn!(?skill, "Can not find locale for skill");
            }
        }
    }

    async fn set_icon<R: std::io::Read + std::io::Seek>(
        &mut self,
        res: &HashMap<SharedString, super::item_res::ItemRes>,
        zip: &mut zip::ZipArchive<R>,
    ) -> Result<()> {
        if let Some(item_res) = res.get(&self.id) {
            if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon.to_lowercase())) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match Self::dds_to_jpeg(buf).await {
                    Ok(icon) => self.icon = Some(icon),
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            } else if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon)) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match Self::dds_to_jpeg(buf).await {
                    Ok(icon) => self.icon = Some(icon),
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            }
        }
        Ok(())
    }
    fn get_full_type(&self) -> SharedString {
        SharedString::new(format!("{}_01", self.weapon_type))
    }

    fn get_type(&self) -> SharedString {
        self.weapon_type.clone()
    }

    fn set_item_set(&mut self, item_set: &Vec<super::item_set::ItemSet>) {
        self.item_set = item_set.iter().find(|f| f.items.contains(&self.id)).cloned();
    }
    
    fn set_product(&mut self, products_by_recipe_id: &HashMap<SharedString, Rc<RefCell<Product>>>, products_by_result_id: &HashMap<SharedString,  Rc<RefCell<Product>>>) {
    }
}
