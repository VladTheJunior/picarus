use std::{
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    sync::Arc,
};

use crate::{
    game_data::{AbstractItem, DataFormat, GameClass, Grade, Item, ItemEffect, item_set::ItemSet, locale::Locale},
    language::LanguageController,
};
use anyhow::Result;

use gpui::{Image, SharedString};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tracing::warn;

#[derive(Debug, Default, Serialize)]
pub struct SecondaryWeapon {
    pub item_set: Option<ItemSet>,
    pub locale: Option<Locale>,
    pub skill_locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    pub id: SharedString,
    pub name: SharedString,
    for_development: f32,
    pub required_level: f32,
    level_restriction: f32,
    pub item_level: u16,
    pub usable_class: BTreeSet<GameClass>,
    randomly_applied_class: SharedString,
    usable_gender: SharedString,
    pub grade: Option<Grade>,
    equipment_slot: SharedString,
    pub weapon_type: SharedString,
    reuse_time: f32,
    attribute_type: SharedString,
    defense_attribute_gauge: f32,
    pub physical_defense: f32,
    pub magical_defense: f32,
    block_damage_reduction: f32,
    durability: f32,
    purchase_price: f32,
    disposal_price: f32,
    pub equip_effect_1: Option<ItemEffect>,
    pub equip_effect_2: Option<ItemEffect>,
    pub equip_effect_3: Option<ItemEffect>,
    pub equip_effect_4: Option<ItemEffect>,
    random_option_enabled: f32,
    pub random_option_count_min: u8,
    pub random_option_count_max: u8,
    option_apply_ratio: SharedString,
    variable_option: SharedString,
    variable_option_min: f32,
    variable_option_max: f32,
    pub enchant_limit: u8,
    enchant_probability: SharedString,
    no_drop: f32,
    no_trade: f32,
    no_disposal: f32,
    indestructible: f32,
    drop_level_check: SharedString,
    binding: SharedString,
    binding_target: SharedString,
    binding_release_count: f32,
    use_restriction: SharedString,
    dyeable: f32,
    consignment_category: SharedString,
    skill_effect: Option<SharedString>,
    ignore_drop_level_check: f32,
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
    integrated_channel_disabled: f32,
    pub reverse_enchant_limit: u8,
    level_up: f32,
    item_exp: f32,
    seal_overlap_enabled: f32,
    overrise_material_id1: SharedString,
    overrise_material_id2: SharedString,
}

impl AbstractItem for SecondaryWeapon {
    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        tag_count: usize,
        global_offset: u64,
        format: DataFormat,
    ) -> Result<Self> {
        // Read all fields sequentially
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

                13 => self.attribute_type = Self::read_string(format, reader).await?,
                21 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_1 = Some(ItemEffect::new(effect));
                    }
                }
                22 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_2 = Some(ItemEffect::new(effect));
                    }
                }
                23 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_3 = Some(ItemEffect::new(effect));
                    }
                }
                24 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_4 = Some(ItemEffect::new(effect));
                    }
                }
                28 => self.option_apply_ratio = Self::read_string(format, reader).await?,
                29 => self.variable_option = Self::read_string(format, reader).await?,
                33 => self.enchant_probability = Self::read_string(format, reader).await?,
                38 => self.drop_level_check = Self::read_string(format, reader).await?,

                39 => self.binding = Self::read_string(format, reader).await?,
                40 => self.binding_target = Self::read_string(format, reader).await?,
                42 => self.use_restriction = Self::read_string(format, reader).await?,
                44 => self.consignment_category = Self::read_string(format, reader).await?,
                45 => {
                    self.skill_effect = {
                        let effect_skill = Self::read_string(format, reader).await?.to_uppercase().replace(".1", "");
                        if effect_skill != "*" {
                            Some(SharedString::new(format!("{}_DESCRIPTION_1", effect_skill)))
                        } else {
                            None
                        }
                    }
                }
                48 => self.grade_up_code_key = Self::read_string(format, reader).await?,
                49 => self.grade_up_result = Self::read_string(format, reader).await?,
                50 => self.disassembly_id = Self::read_string(format, reader).await?,
                51 => self.extraction_id = Self::read_string(format, reader).await?,
                55 => self.overrise_id1 = Self::read_string(format, reader).await?,
                56 => self.overrise_id2 = Self::read_string(format, reader).await?,
                57 => self.overrise_id3 = Self::read_string(format, reader).await?,

                64 => self.overrise_material_id1 = Self::read_string(format, reader).await?,
                65 => self.overrise_material_id2 = Self::read_string(format, reader).await?,

                _ => {
                    let value = reader.read_f32_le().await?;
                    // Assign to the appropriate field based on index
                    match tag_idx {
                        2 => self.for_development = value,
                        3 => self.required_level = value,
                        4 => self.level_restriction = value,
                        5 => self.item_level = value as u16,
                        9 => self.grade = Grade::from_repr(value as u8),
                        12 => self.reuse_time = value,
                        14 => self.defense_attribute_gauge = value,
                        15 => self.physical_defense = value,
                        16 => self.magical_defense = value,
                        17 => self.block_damage_reduction = value,
                        18 => self.durability = value,
                        19 => self.purchase_price = value,
                        20 => self.disposal_price = value,

                        25 => self.random_option_enabled = value,
                        26 => self.random_option_count_min = value as u8,
                        27 => self.random_option_count_max = value as u8,
                        30 => self.variable_option_min = value,
                        31 => self.variable_option_max = value,
                        32 => self.enchant_limit = value as u8,
                        34 => self.no_drop = value,
                        35 => self.no_trade = value,
                        36 => self.no_disposal = value,
                        37 => self.indestructible = value,
                        41 => self.binding_release_count = value,
                        43 => self.dyeable = value,
                        46 => self.ignore_drop_level_check = value,
                        47 => self.usage_period = value,
                        52 => self.use_level_option = value,
                        53 => self.level_weight = value,
                        54 => self.overrise_max = value as u8,
                        58 => self.contents_level = value,
                        59 => self.integrated_channel_disabled = value,
                        60 => self.reverse_enchant_limit = value as u8,
                        61 => self.level_up = value,
                        62 => self.item_exp = value,
                        63 => self.seal_overlap_enabled = value,

                        _ => {} // Should not happen
                    }
                }
            }
        }

        Ok(self)
    }
}

impl SecondaryWeapon {
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

impl Item for SecondaryWeapon {
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
}
