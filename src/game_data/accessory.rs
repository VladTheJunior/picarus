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
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Debug, Default, Serialize)]
pub struct Accessory {
    pub item_set: Option<ItemSet>,
    pub locale: Option<Locale>,
    pub skill_locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    pub id: SharedString,
    pub name: SharedString,
    pub development_use: f32,
    pub required_level: f32,
    pub restricted_level: f32,
    pub item_level: u16,
    pub usable_class: BTreeSet<GameClass>,
    pub random_apply_class: SharedString,
    pub usable_gender: SharedString,
    pub grade: Option<Grade>,
    pub accessory_type: SharedString,
    pub durability: f32,
    pub purchase_price: f32,
    pub disposal_price: f32,
    pub physical_min_attack: f32,
    pub physical_max_attack: f32,
    pub magic_defense: f32,
    pub equip_effect_1: Option<ItemEffect>,
    pub equip_effect_2: Option<ItemEffect>,
    pub equip_effect_3: Option<ItemEffect>,
    pub equip_effect_4: Option<ItemEffect>,
    pub random_option_enabled: f32,
    pub random_option_count_min: u8,
    pub random_option_count_max: u8,
    pub option_apply_ratio: SharedString,
    pub variable_attribute: SharedString,
    pub variable_attribute_min: f32,
    pub variable_attribute_max: f32,
    pub enhancement_limit: u8,
    pub enhancement_probability: SharedString,
    pub no_drop: f32,
    pub no_trade: f32,
    pub no_disposal: f32,
    pub no_destroy: f32,
    pub drop_level_check: SharedString,
    pub binding: SharedString,
    pub binding_target: SharedString,
    pub binding_release_count: f32,
    pub usage_restriction: SharedString,
    pub dyeable: f32,
    pub sales_agency_category: SharedString,
    pub skill_effect: Option<SharedString>,
    pub ignore_drop_level_check: f32,
    pub usage_period: f32,
    pub grade_up_code_key: SharedString,
    pub grade_up_result: SharedString,
    pub disassembly_id: SharedString,
    pub extraction_id: SharedString,
    pub sealed_fellow_equip_slot_max: f32,
    pub use_level_option: f32,
    pub level_weight: f32,
    pub overrise_max: u8,
    pub overrise_id: SharedString,
    pub overrise_id_2: SharedString,
    pub overrise_id_3: SharedString,
    pub contents_level: f32,
    pub integrated_channel_disabled: f32,
    pub reverse_enhancement_limit: u8,
    pub level_up: f32,
    pub item_exp: f32,
    pub overrise_material_id1: SharedString,
    pub overrise_material_id2: SharedString,
}

impl AbstractItem for Accessory {
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
                7 => self.random_apply_class = Self::read_string(format, reader).await?,
                8 => self.usable_gender = Self::read_string(format, reader).await?,
                10 => self.accessory_type = SharedString::new(Self::read_string(format, reader).await?.to_lowercase()),
                17 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_1 = Some(ItemEffect::new(effect));
                    }
                }
                18 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_2 = Some(ItemEffect::new(effect));
                    }
                }
                19 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_3 = Some(ItemEffect::new(effect));
                    }
                }
                20 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_4 = Some(ItemEffect::new(effect));
                    }
                }
                24 => self.option_apply_ratio = Self::read_string(format, reader).await?,
                25 => self.variable_attribute = Self::read_string(format, reader).await?,
                29 => self.enhancement_probability = Self::read_string(format, reader).await?,
                34 => self.drop_level_check = Self::read_string(format, reader).await?,
                35 => self.binding = Self::read_string(format, reader).await?,
                36 => self.binding_target = Self::read_string(format, reader).await?,
                38 => self.usage_restriction = Self::read_string(format, reader).await?,
                40 => self.sales_agency_category = Self::read_string(format, reader).await?,
                41 => {
                    self.skill_effect = {
                        let effect_skill = Self::read_string(format, reader).await?.to_uppercase().replace(".1", "");
                        if effect_skill != "*" {
                            Some(SharedString::new(format!("{}_DESCRIPTION_1", effect_skill)))
                        } else {
                            None
                        }
                    }
                }
                44 => self.grade_up_code_key = Self::read_string(format, reader).await?,
                45 => self.grade_up_result = Self::read_string(format, reader).await?,
                46 => self.disassembly_id = Self::read_string(format, reader).await?,
                47 => self.extraction_id = Self::read_string(format, reader).await?,
                52 => self.overrise_id = Self::read_string(format, reader).await?,
                53 => self.overrise_id_2 = Self::read_string(format, reader).await?,
                54 => self.overrise_id_3 = Self::read_string(format, reader).await?,
                60 => self.overrise_material_id1 = Self::read_string(format, reader).await?,
                61 => self.overrise_material_id2 = Self::read_string(format, reader).await?,
                // f32 fields
                2 => self.development_use = reader.read_f32_le().await?,
                3 => self.required_level = reader.read_f32_le().await?,
                4 => self.restricted_level = reader.read_f32_le().await?,
                5 => self.item_level = reader.read_f32_le().await? as u16,
                9 => self.grade = Grade::from_repr(reader.read_f32_le().await? as u8),
                11 => self.durability = reader.read_f32_le().await?,
                12 => self.purchase_price = reader.read_f32_le().await?,
                13 => self.disposal_price = reader.read_f32_le().await?,

                14 => self.physical_min_attack = reader.read_f32_le().await?,
                15 => self.physical_max_attack = reader.read_f32_le().await?,
                16 => self.magic_defense = reader.read_f32_le().await?,

                21 => self.random_option_enabled = reader.read_f32_le().await?,
                22 => self.random_option_count_min = reader.read_f32_le().await? as u8,
                23 => self.random_option_count_max = reader.read_f32_le().await? as u8,

                26 => self.variable_attribute_min = reader.read_f32_le().await?,
                27 => self.variable_attribute_max = reader.read_f32_le().await?,
                28 => self.enhancement_limit = reader.read_f32_le().await? as u8,
                30 => self.no_drop = reader.read_f32_le().await?,
                31 => self.no_trade = reader.read_f32_le().await?,
                32 => self.no_disposal = reader.read_f32_le().await?,
                33 => self.no_destroy = reader.read_f32_le().await?,
                37 => self.binding_release_count = reader.read_f32_le().await?,
                39 => self.dyeable = reader.read_f32_le().await?,
                42 => self.ignore_drop_level_check = reader.read_f32_le().await?,
                43 => self.usage_period = reader.read_f32_le().await?,
                48 => self.sealed_fellow_equip_slot_max = reader.read_f32_le().await?,
                49 => self.use_level_option = reader.read_f32_le().await?,
                50 => self.level_weight = reader.read_f32_le().await?,
                51 => self.overrise_max = reader.read_f32_le().await? as u8,
                55 => self.contents_level = reader.read_f32_le().await?,
                56 => self.integrated_channel_disabled = reader.read_f32_le().await?,
                57 => self.reverse_enhancement_limit = reader.read_f32_le().await? as u8,
                58 => self.level_up = reader.read_f32_le().await?,
                59 => self.item_exp = reader.read_f32_le().await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Accessory {
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

impl Item for Accessory {
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
        SharedString::new(format!("{}_01", self.accessory_type))
    }

    fn get_type(&self) -> SharedString {
        self.accessory_type.clone()
    }

    fn set_item_set(&mut self, item_set: &Vec<ItemSet>) {
        self.item_set = item_set.iter().find(|f| f.items.contains(&self.id)).cloned();
    }
}
