use std::{collections::HashMap, io::SeekFrom};

use crate::game_data::{AsyncBufReadExtReadString, DataFormat, TagType, common::Common, item::ItemTrait, item::ReadableItem, locale::Locale};
use anyhow::Result;

use gpui::SharedString;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tracing::warn;

#[derive(Default)]
pub struct SecondaryWeapon {
    pub debug: Vec<u8>,
    pub skill_locale: Option<Locale>,
    pub common: Common,
    pub equipment_slot: SharedString,
    pub weapon_type: SharedString,
    pub skill_effect: Option<SharedString>,
    pub physical_defense: f32,
    pub magical_defense: f32,

    pub random_option_count_min: u8,
    pub random_option_count_max: u8,

    pub enchant_limit: u8,

    pub overrise_max: u8,

    pub reverse_enchant_limit: u8,
}

impl ReadableItem for SecondaryWeapon {
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

                21 => self.common.parse_effect(reader, Self::FORMAT).await?,
                22 => self.common.parse_effect(reader, Self::FORMAT).await?,
                23 => self.common.parse_effect(reader, Self::FORMAT).await?,
                24 => self.common.parse_effect(reader, Self::FORMAT).await?,

                39 => self.common.parse_binding(reader, Self::FORMAT).await?,

                45 => {
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

                15 => self.physical_defense = reader.read_f32_le().await?,
                16 => self.magical_defense = reader.read_f32_le().await?,

                26 => self.random_option_count_min = reader.read_f32_le().await? as u8,
                27 => self.random_option_count_max = reader.read_f32_le().await? as u8,

                32 => self.enchant_limit = reader.read_f32_le().await? as u8,

                35 => self.common.parse_no_trade(reader).await?,
                36 => self.common.parse_no_sell(reader).await?,
                37 => self.common.parse_no_destroy(reader).await?,

                54 => self.overrise_max = reader.read_f32_le().await? as u8,

                60 => self.reverse_enchant_limit = reader.read_f32_le().await? as u8,

                _ => {} // Should not happen
            }
        }

        Ok(self)
    }
}

impl SecondaryWeapon {
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

impl ItemTrait for SecondaryWeapon {
    fn common(&self) -> &Common {
        &self.common
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
}
