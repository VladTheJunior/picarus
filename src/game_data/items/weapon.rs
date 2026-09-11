use std::{collections::HashMap, io::SeekFrom};

use crate::{
    game_data::{
        AsyncBufReadExtReadString, DataFormat, TagType,
        common::Common,
        items::{ItemTrait, ReadableItem},
        locale::Locale,
    },
    game_data_view::PreviewBuilder,
};
use anyhow::Result;

use gpui_kit::SharedString;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tracing::warn;

#[derive(Default, Clone)]
pub struct Weapon {
    pub debug: Vec<u8>,
    pub skill_locale: Option<Locale>,
    pub common: Common,

    pub equipment_slot: SharedString,
    pub weapon_type: SharedString,
    pub attack_range_type: SharedString,
    pub attribute_type: SharedString,
    pub skill_effect: Option<SharedString>,
    pub min_attack: f32,
    pub max_attack: f32,

    pub attack_speed: f32,

    pub min_random_options: u8,
    pub max_random_options: u8,

    pub min_crafting_seal_slots: u8,
    pub max_crafting_seal_slots: u8,

    pub enhancement_limit: u8,

    pub overrise_max: u8,

    pub reverse_enhancement_limit: u8,
}

impl ReadableItem for Weapon {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.common.id.clone()
    }

    type CollectionItem = Self;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        item
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
        &mut self.debug
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        self.parse_debug(reader, offsets, item_idx, definitions, global_offset).await?;
        let tag_count = definitions.len();
        for (tag_idx, (tag, tag_type)) in definitions.iter().enumerate() {
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
            self.common.parse(tag, tag_type, reader, Self::FORMAT).await?;
            match tag_idx {
                10 => self.equipment_slot = reader.read_string(Self::FORMAT).await?,
                11 => self.weapon_type = SharedString::new(reader.read_string(Self::FORMAT).await?.to_lowercase()),
                12 => self.attack_range_type = reader.read_string(Self::FORMAT).await?,

                64 => {
                    self.skill_effect = {
                        let effect_skill = reader.read_string(Self::FORMAT).await?.to_uppercase().replace(".", "_DESCRIPTION_");
                        if effect_skill != "*" {
                            Some(SharedString::new(effect_skill))
                        } else {
                            None
                        }
                    }
                }

                17 => self.min_attack = reader.read_f32_le().await?,
                18 => self.max_attack = reader.read_f32_le().await?,
                20 => self.attack_speed = reader.read_f32_le().await?,

                36 => self.min_random_options = reader.read_f32_le().await? as u8,
                37 => self.max_random_options = reader.read_f32_le().await? as u8,

                42 => self.min_crafting_seal_slots = reader.read_f32_le().await? as u8,
                43 => self.max_crafting_seal_slots = reader.read_f32_le().await? as u8,

                51 => self.enhancement_limit = reader.read_f32_le().await? as u8,

                73 => self.overrise_max = reader.read_f32_le().await? as u8,

                79 => self.reverse_enhancement_limit = reader.read_f32_le().await? as u8,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Weapon {
    pub fn set_skill_locale(&mut self, skill_locales: &HashMap<SharedString, Locale>) {
        if let Some(skill) = self.skill_effect.as_ref() {
            self.skill_locale = skill_locales.get(skill).cloned();
            if self.skill_locale.is_none() {
                warn!(?skill, "Can not find locale for skill");
            }
        }
    }

    pub fn get_localized_skill(&self) -> Option<SharedString> {
        self.skill_locale.as_ref().and_then(|f| f.locale()).or_else(|| self.skill_effect.clone())
    }

    pub fn get_full_type(&self) -> SharedString {
        SharedString::new(format!("{}_01", self.weapon_type))
    }

    pub fn get_type(&self) -> SharedString {
        self.weapon_type.clone()
    }
}

impl ItemTrait for Weapon {
    fn common(&self) -> &Common {
        &self.common
    }
        fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }
    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn get_full_type(&self) -> Option<SharedString> {
        Some(self.get_full_type())
    }

    fn get_type(&self) -> Option<SharedString> {
        Some(self.get_type())
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common())
            .attack(self.min_attack, self.max_attack, self.attack_speed)
            .min_random_effects(self.min_random_options)
            .max_random_effects(self.min_random_options)
            .min_sealed_slots(self.min_crafting_seal_slots)
            .max_sealed_slots(self.max_crafting_seal_slots)
            .temper_limit(self.enhancement_limit)
            .reverse_limit(self.reverse_enhancement_limit)
            .transcendence_limit(self.overrise_max)
            .skill_locale(self.get_localized_skill())
    }
}
