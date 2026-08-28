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
pub struct Weapon {
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

                10 => self.equipment_slot = reader.read_string(Self::FORMAT).await?,
                11 => self.weapon_type = SharedString::new(reader.read_string(Self::FORMAT).await?.to_lowercase()),

                31 => self.common.parse_effect(reader, Self::FORMAT).await?,
                32 => self.common.parse_effect(reader, Self::FORMAT).await?,
                33 => self.common.parse_effect(reader, Self::FORMAT).await?,
                34 => self.common.parse_effect(reader, Self::FORMAT).await?,

                58 => self.common.parse_binding(reader, Self::FORMAT).await?,

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

                3 => self.common.parse_required_level(reader).await?,

                5 => self.common.parse_item_level(reader).await?,
                9 => self.common.parse_grade(reader).await?,

                17 => self.min_attack = reader.read_f32_le().await?,
                18 => self.max_attack = reader.read_f32_le().await?,
                20 => self.attack_speed = reader.read_f32_le().await?,

                36 => self.min_random_options = reader.read_f32_le().await? as u8,
                37 => self.max_random_options = reader.read_f32_le().await? as u8,

                42 => self.min_crafting_seal_slots = reader.read_f32_le().await? as u8,
                43 => self.max_crafting_seal_slots = reader.read_f32_le().await? as u8,

                51 => self.enhancement_limit = reader.read_f32_le().await? as u8,

                54 => self.common.parse_no_trade(reader).await?,
                55 => self.common.parse_no_sell(reader).await?,
                56 => self.common.parse_no_destroy(reader).await?,

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
   fn common(&self) ->  &Common {
       &self.common
   }

       fn get_full_type(&self) -> Option<SharedString> {
        Some(self.get_full_type())
    }
    
    fn get_type(&self) -> Option<SharedString> {
        Some(self.get_type())
    }
}
