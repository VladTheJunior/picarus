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
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

use gpui::SharedString;
use tracing::warn;

#[derive(Default, Clone)]
pub struct Style {
    pub debug: Vec<u8>,
    pub skill_locale: Option<Locale>,
    pub common: Common,

    pub skill_effect: Option<SharedString>,
}

impl ReadableItem for Style {
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
        for (tag_idx, tag) in definitions.keys().enumerate() {
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
            self.common.parse(tag, reader, Self::FORMAT).await?;
            match tag_idx {
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
    fn common(&self) -> &Common {
        &self.common
    }

    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common()).skill_locale(self.get_localized_skill())
    }
}
