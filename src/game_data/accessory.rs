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
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Default)]
pub struct Accessory {
    pub common: Common,
    pub skill_locale: Option<Locale>,

    pub accessory_type: SharedString,

    pub physical_min_attack: f32,
    pub physical_max_attack: f32,
    pub magic_defense: f32,

    pub random_effects_count_min: u8,
    pub random_effects_count_max: u8,

    pub overrise_max: u8,
    pub enhancement_limit: u8,

    pub skill_effect: Option<SharedString>,

    pub reverse_enhancement_limit: u8,
}

impl ReadableItem for Accessory {
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
        // Read all fields sequentially
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
                // String fields
                0 => self.common.parse_id(reader, Self::FORMAT).await?,

                6 => self.common.parse_usable_class(reader, Self::FORMAT).await?,

                10 => self.accessory_type = SharedString::new(reader.read_string(Self::FORMAT).await?.to_lowercase()),
                17 => self.common.parse_effect(reader, Self::FORMAT).await?,
                18 => self.common.parse_effect(reader, Self::FORMAT).await?,
                19 => self.common.parse_effect(reader, Self::FORMAT).await?,
                20 => self.common.parse_effect(reader, Self::FORMAT).await?,

                35 => self.common.parse_binding(reader, Self::FORMAT).await?,

                41 => {
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

                14 => self.physical_min_attack = reader.read_f32_le().await?,
                15 => self.physical_max_attack = reader.read_f32_le().await?,
                16 => self.magic_defense = reader.read_f32_le().await?,

                22 => self.random_effects_count_min = reader.read_f32_le().await? as u8,
                23 => self.random_effects_count_max = reader.read_f32_le().await? as u8,

                28 => self.enhancement_limit = reader.read_f32_le().await? as u8,

                31 => self.common.parse_no_trade(reader).await?,
                32 => self.common.parse_no_sell(reader).await?,
                33 => self.common.parse_no_destroy(reader).await?,

                51 => self.overrise_max = reader.read_f32_le().await? as u8,

                57 => self.reverse_enhancement_limit = reader.read_f32_le().await? as u8,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Accessory {
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
        SharedString::new(format!("{}_01", self.accessory_type))
    }

    pub fn get_type(&self) -> SharedString {
        self.accessory_type.clone()
    }
}

impl ItemTrait for Accessory {
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
