use std::{collections::HashMap, io::SeekFrom};

use crate::{
    game_data::{
        DataFormat, TagType,
        common::Common,
        items::{ItemTrait, ReadableItem},
        locale::Locale,
    },
    game_data_view::PreviewBuilder,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui_kit::SharedString;

#[derive(Default, Clone)]
pub struct Bracelet {
    pub debug: Vec<u8>,
    pub description_locale: Option<Locale>,
    pub common: Common,
    pub max_gem_slots: u8,
}

impl ReadableItem for Bracelet {
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
                41 => self.max_gem_slots = reader.read_f32_le().await? as u8,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Bracelet {
    pub fn set_description_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.description_locale = locales.get(&SharedString::new(format!("{}_DESCRIPTION", self.common.id))).cloned();
    }
}

impl ItemTrait for Bracelet {
    fn common(&self) -> &Common {
        &self.common
    }

    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common())
            .description_locale(self.description_locale.as_ref().and_then(|f| f.locale()))
            .max_gem_slots(self.max_gem_slots)
    }
}
