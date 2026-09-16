use std::io::SeekFrom;

use crate::{
    game_data::{
        AsyncBufReadExtReadString, DataFormat, TagType,
        common::Common,
        items::{ItemTrait, ReadableItem},
    },
    game_data_view::PreviewBuilder,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui_kit::SharedString;

#[derive(Default, Clone)]
pub struct Relic {
    pub debug: Vec<u8>,
    pub common: Common,
    pub magical_defense: f32,
    pub random_effects_count_min: u8,
    pub random_effects_count_max: u8,
    pub equip_slot: SharedString,
}

impl ReadableItem for Relic {
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
                10 => self.equip_slot = reader.read_string(Self::FORMAT).await?,
                14 => self.magical_defense = reader.read_f32_le().await?,
                20 => self.random_effects_count_min = reader.read_f32_le().await? as u8,
                21 => self.random_effects_count_max = reader.read_f32_le().await? as u8,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Relic {
    pub fn get_type(&self) -> SharedString {
        self.equip_slot.clone()
    }
}

impl ItemTrait for Relic {
    fn common(&self) -> &Common {
        &self.common
    }
    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }
    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common())
            .magic_defense(self.magical_defense)
            .min_random_effects(self.random_effects_count_min)
            .max_random_effects(self.random_effects_count_max)
    }
}
