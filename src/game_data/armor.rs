use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::Rc,
    sync::Arc,
};

use crate::{
    game_data::{
        AsyncBufReadExtReadString, Binding, Common, DataFormat, GameClass, Grade, Item, ItemEffect, ItemTrait, ReadableItem, TagType,
        item_set::ItemSet, locale::Locale, product::Product,
    },
    language::LanguageController,
};
use anyhow::Result;
use gpui::{Image, SharedString};
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tracing::warn;
#[derive(Default)]
pub struct Armor {
    pub common: Common,
    pub skill_locale: Option<Locale>,
    pub equip_slot: SharedString,
    pub armor_type: SharedString,
    pub physical_defense: f32,
    pub magical_defense: f32,
    pub random_effects_count_min: u8,
    pub random_effects_count_max: u8,
    pub sealed_fellow_slots_min: u8,
    pub sealed_fellow_slots_max: u8,
    pub skill_effect: Option<SharedString>,
    pub enhancement_limit: u8,
    pub reverse_enhancement_limit: u8,
    pub overrise_max: u8,
}

impl Armor {
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
        SharedString::new(format!("{}_{}", self.armor_type, self.equip_slot))
    }

    pub fn get_type(&self) -> SharedString {
        self.armor_type.clone()
    }
}

impl ReadableItem for Armor {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(item: &Self) -> Self::Key {
        item.common.id.clone()
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        self.common.debug = Self::parse_debug(reader, offsets, item_idx, definitions, global_offset).await?;
        let tag_count = definitions.len();

        for tag_idx in 0..tag_count {
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

            match tag_idx {
                0 => self.common.parse_id(reader, Self::FORMAT).await?,
                6 => self.common.parse_usable_class(reader, Self::FORMAT).await?,
                10 => self.equip_slot = reader.read_string(Self::FORMAT).await?,
                11 => self.armor_type = SharedString::new(reader.read_string(Self::FORMAT).await?.to_lowercase()),

                19 => self.common.parse_effect(reader, Self::FORMAT).await?,
                20 => self.common.parse_effect(reader, Self::FORMAT).await?,
                21 => self.common.parse_effect(reader, Self::FORMAT).await?,
                22 => self.common.parse_effect(reader, Self::FORMAT).await?,

                43 => self.common.parse_binding(reader, Self::FORMAT).await?,
                49 => {
                    self.skill_effect = {
                        let effect_skill = reader.read_string(Self::FORMAT).await?.to_uppercase().replace(".", "_DESCRIPTION_");
                        if effect_skill != "*" {
                            Some(SharedString::new(effect_skill))
                        } else {
                            None
                        }
                    }
                }

                3 => self.common.parse_required_level(reader).await?,
                5 => self.common.parse_item_level(reader).await?,
                9 => self.common.parse_grade(reader).await?,
                14 => self.physical_defense = reader.read_f32_le().await?,
                15 => self.magical_defense = reader.read_f32_le().await?,

                24 => self.random_effects_count_min = reader.read_f32_le().await? as u8,
                25 => self.random_effects_count_max = reader.read_f32_le().await? as u8,
                27 => self.sealed_fellow_slots_min = reader.read_f32_le().await? as u8,
                28 => self.sealed_fellow_slots_max = reader.read_f32_le().await? as u8,
                36 => self.enhancement_limit = reader.read_f32_le().await? as u8,

                39 => self.common.parse_no_trade(reader).await?,
                40 => self.common.parse_no_sell(reader).await?,
                41 => self.common.parse_no_destroy(reader).await?,
                58 => self.overrise_max = reader.read_f32_le().await? as u8,
                64 => self.reverse_enhancement_limit = reader.read_f32_le().await? as u8,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl ItemTrait for Armor {
    fn common(&self) -> &Common {
        &self.common
    }

        fn get_full_type(&self) -> Option<SharedString> {
        Some(self.get_full_type())
    }
    
    fn get_type(&self) -> Option<SharedString> {
        Some(self.get_type())
    }
}
