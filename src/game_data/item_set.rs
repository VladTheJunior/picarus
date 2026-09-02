use std::{collections::HashMap, io::SeekFrom};

use crate::{
    game_data::{AsyncBufReadExtReadString, items::ReadableItem},
    game_data::{DataFormat, TagType, effects::ItemEffect, locale::Locale},
};
use anyhow::Result;
use gpui::SharedString;
use indexmap::IndexMap;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tracing::warn;

#[derive(Default, Clone)]
pub struct ItemSetEffects {
    pub locale: Option<Locale>,
    pub seteffect_count: u8,
    pub seteffect_effects: Vec<ItemEffect>,
    pub seteffect_skill: Option<SharedString>,
}

impl ItemSetEffects {
    pub fn get_localized_name(&self) -> Option<SharedString> {
        self.locale.as_ref().and_then(|f| f.locale()).or_else(|| self.seteffect_skill.clone())
    }
}

#[derive(Default, Clone)]
pub struct ItemSet {
    pub locale: Option<Locale>,
    pub setid: SharedString,
    pub setname: SharedString,
    pub items: Vec<SharedString>,
    pub effects: Vec<ItemSetEffects>,
}

impl ItemSet {
    pub fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.locale = locales.get(&self.setid).cloned();
    }

    pub fn set_effects_skill_locale(&mut self, skill_locales: &HashMap<SharedString, Locale>) {
        for effect in self.effects.iter_mut() {
            if let Some(skill) = effect.seteffect_skill.as_ref() {
                effect.locale = skill_locales.get(skill).cloned();
                if effect.locale.is_none() {
                    warn!(?skill, "Can not find locale for skill");
                }
            }
        }
    }

    pub fn get_localized_name(&self) -> SharedString {
        self.locale.as_ref().and_then(|f| f.locale()).unwrap_or_else(|| self.setid.clone())
    }
}

impl ReadableItem for ItemSet {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.setid.clone()
    }

    type CollectionItem = Self;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        item
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
        unimplemented!()
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        // Read all fields sequentially
        let tag_count = definitions.len();
        let mut set_effects = ItemSetEffects::default();
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
            if tag_count == 58 {
                match tag_idx {
                    0 => self.setid = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),

                    1 => self.setname = reader.read_string(Self::FORMAT).await?,
                    2..10 => {
                        let id = reader.read_string(Self::FORMAT).await?.to_uppercase();
                        if id != "*" {
                            self.items.push(SharedString::new(id));
                        }
                    }

                    10 | 16 | 22 | 28 | 34 | 40 | 46 | 52 => set_effects.seteffect_count = reader.read_f32_le().await? as u8,
                    11..15 | 17..21 | 23..27 | 29..33 | 35..39 | 41..45 | 47..51 | 53..57 => {
                        let effect = reader.read_string(Self::FORMAT).await?;
                        if effect != "*" && effect != "0" {
                            set_effects.seteffect_effects.push(ItemEffect::new(effect));
                        }
                    }
                    15 | 21 | 27 | 33 | 39 | 45 | 51 | 57 => {
                        let effect_skill = reader.read_string(Self::FORMAT).await?.to_uppercase();
                        if effect_skill != "*" {
                            set_effects.seteffect_skill = Some(SharedString::new(format!("{}_DESCRIPTION_1", effect_skill)));
                        }

                        if !set_effects.seteffect_effects.is_empty() || set_effects.seteffect_skill.is_some() {
                            self.effects.push(set_effects);
                        }
                        set_effects = ItemSetEffects::default();
                    }

                    _ => {}
                }
            } else {
                match tag_idx {
                    0 => self.setid = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),

                    1 => self.setname = reader.read_string(Self::FORMAT).await?,
                    2..16 => {
                        let id = reader.read_string(Self::FORMAT).await?.to_uppercase();
                        if id != "*" {
                            self.items.push(SharedString::new(id));
                        }
                    }

                    16 | 22 | 28 | 34 | 40 | 46 | 52 | 58 => set_effects.seteffect_count = reader.read_f32_le().await? as u8,
                    17..21 | 23..27 | 29..33 | 35..39 | 41..45 | 47..51 | 53..57 | 59..63 => {
                        let effect = reader.read_string(Self::FORMAT).await?;
                        if effect != "*" && effect != "0" {
                            set_effects.seteffect_effects.push(ItemEffect::new(effect));
                        }
                    }
                    21 | 27 | 33 | 39 | 45 | 51 | 57 | 63 => {
                        let effect_skill = reader.read_string(Self::FORMAT).await?.to_uppercase();
                        if effect_skill != "*" {
                            set_effects.seteffect_skill = Some(SharedString::new(format!("{}_DESCRIPTION_1", effect_skill)));
                        }

                        if !set_effects.seteffect_effects.is_empty() || set_effects.seteffect_skill.is_some() {
                            self.effects.push(set_effects);
                        }
                        set_effects = ItemSetEffects::default();
                    }

                    _ => {}
                }
            }
        }
        Ok(self)
    }
}
