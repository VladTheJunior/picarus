use std::io::SeekFrom;

use crate::game_data::{AsyncBufReadExtReadString, DataFormat, TagType, items::ReadableItem};
use anyhow::Result;
use gpui::SharedString;
use indexmap::IndexMap;
use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Debug, Default)]
pub struct ItemRes {
    pub id: SharedString,
    pub icon: SharedString,
}

impl ReadableItem for ItemRes {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.id.clone()
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

            match tag.as_str() {
                "id" => {
                    self.id = {
                        let id = reader.read_string(Self::FORMAT).await?.to_uppercase();
                        SharedString::new(id)
                    }
                }
                "icon" => {
                    self.icon = reader.read_string(Self::FORMAT).await?;
                }
                _ => {}
            }
        }
        Ok(self)
    }
}
