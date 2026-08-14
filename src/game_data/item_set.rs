use std::{collections::HashMap, io::SeekFrom};

use crate::{
    game_data::{AbstractItem, DataFormat, ItemEffect, locale::Locale},
    language::LanguageController,
};
use anyhow::Result;
use gpui::SharedString;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tracing::warn;

#[derive(Debug, Default, Serialize, Clone)]
pub struct ItemSetEffects {
    pub locale: Option<Locale>,
    pub seteffect_count: u8,
    pub seteffect_effects: Vec<ItemEffect>,
    pub seteffect_skill: Option<SharedString>,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct ItemSet {
    pub locale: Option<Locale>,
    pub setid: SharedString,
    pub setname: SharedString,
    pub items: Vec<SharedString>,
    pub effects: Vec<ItemSetEffects>,
}

impl ItemSet {
    pub fn get_locale(&self) -> SharedString {
        let language = LanguageController::get_current_language();

        self.locale
            .as_ref()
            .map(|f| match language {
                crate::settings::Language::English => f.eng.clone(),
                crate::settings::Language::Russian => f.rus.clone(),
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or_else(|| self.setid.clone())
    }

    pub fn set_skill_effects_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        for effect in self.effects.iter_mut() {
            if let Some(skill) = effect.seteffect_skill.as_ref() {
                effect.locale = locales.get(skill).cloned();
                if effect.locale.is_none() {
                    warn!(?skill, "Can not find locale for skill");
                }
            }
        }
    }
}

impl ItemSetEffects {
    pub fn get_locale(&self) -> Option<SharedString> {
        let language = LanguageController::get_current_language();

        self.locale
            .as_ref()
            .map(|f| match language {
                crate::settings::Language::English => f.eng.clone(),
                crate::settings::Language::Russian => f.rus.clone(),
            })
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
            .or_else(|| self.seteffect_skill.clone())
    }
}

impl AbstractItem for ItemSet {
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
        let mut set_effects = ItemSetEffects::default();
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
                0 => self.setid = SharedString::new(Self::read_string(format, reader).await?.to_uppercase()),

                1 => self.setname = Self::read_string(format, reader).await?,
                2..16 => {
                    let id = Self::read_string(format, reader).await?.to_uppercase();
                    if id != "*" {
                        self.items.push(SharedString::new(id));
                    }
                }

                16 | 22 | 28 | 34 | 40 | 46 | 52 | 58 => set_effects.seteffect_count = reader.read_f32_le().await? as u8,
                17..21 | 23..27 | 29..33 | 35..39 | 41..45 | 47..51 | 53..57 | 59..63 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        set_effects.seteffect_effects.push(ItemEffect::new(effect));
                    }
                }
                21 | 27 | 33 | 39 | 45 | 51 | 57 | 63 => {
                    let effect_skill = Self::read_string(format, reader).await?.to_uppercase();
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
        Ok(self)
    }
}
