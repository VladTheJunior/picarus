use std::io::SeekFrom;

use crate::{
    game_data::{
        DataFormat, TagType,
        common::Common,
        items::{ItemTrait, ReadableItem},
    },
    game_data_view::PreviewBuilder,
};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Default, Clone)]
pub struct FellowEquip {
    pub debug: Vec<u8>,
    pub common: Common,

    pub max_ep_plus: Option<f32>,
}
impl ReadableItem for FellowEquip {
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
        // Read all fields sequentially
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
                12 => {
                    let v = reader.read_f32_le().await?;

                    if v != 0.0 {
                        self.max_ep_plus = Some(v);
                    }
                }

                _ => {}
            }
        }

        Ok(self)
    }
}

impl ItemTrait for FellowEquip {
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
        PreviewBuilder::new(self.common()).max_ep(self.max_ep_plus)
    }
}
