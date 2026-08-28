use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::Rc,
    sync::Arc,
};

use crate::{
    game_data::{
        AsyncBufReadExtReadString, Binding, Common, DataFormat, GameClass, Grade, Item, ItemEffect, ItemTrait, ReadableItem, TagType, item_set::ItemSet, locale::Locale, product::Product,
    }, language::LanguageController,
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Default)]
pub struct Style {
    pub skill_locale: Option<Locale>,
    pub common: Common,

    pub skill_effect: Option<SharedString>,
}

impl ReadableItem for Style {
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

                3 => self.common.parse_required_level(reader).await?,

                5 => self.common.parse_item_level(reader).await?,
                6 => self.common.parse_usable_class(reader, Self::FORMAT).await?,

                8 => self.common.parse_grade(reader).await?,

                13 => self.common.parse_effect(reader, Self::FORMAT).await?,
                14 => self.common.parse_effect(reader, Self::FORMAT).await?,
                15 => self.common.parse_effect(reader, Self::FORMAT).await?,
                16 => self.common.parse_effect(reader, Self::FORMAT).await?,

                26 => self.common.parse_no_trade(reader).await?,
                27 => self.common.parse_no_sell(reader).await?,
                28 => self.common.parse_no_destroy(reader).await?,

                30 => self.common.parse_binding(reader, Self::FORMAT).await?,

                37 => {
                    self.skill_effect = {
                        let effect_skill = reader.read_string(Self::FORMAT).await?.to_uppercase().replace(".", "_DESCRIPTION_");
                        if effect_skill != "*" {
                            Some(SharedString::new(effect_skill))
                        } else {
                            None
                        }
                    }
                }

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Style {
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
  
}


impl ItemTrait for Style {
   fn common(&self) ->  &Common {
       &self.common
   }
}
