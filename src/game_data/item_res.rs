use std::io::SeekFrom;

use crate::game_data::{AbstractItem, DataFormat, TagType};
use anyhow::Result;
use gpui::SharedString;
use indexmap::IndexMap;
use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Debug, Default)]
pub struct ItemRes {
    pub id: SharedString,
    pub icon: SharedString,
    pub using_recipe_type: Option<SharedString>,
}

impl AbstractItem for ItemRes {
    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<String, TagType>,
        global_offset: u64,
        format: DataFormat,
    ) -> Result<Self> {
        // Read all fields sequentially
        let tag_count = definitions.len();
        let icon_index = definitions.get_index_of("icon");
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
                0 => {
                    self.id = {
                        let id = Self::read_string(format, reader).await?.to_uppercase();
                        SharedString::new(id)
                    }
                }

                /*2 => {
                    if tag_count == 13 || tag_count == 21 || tag_count == 44 {
                        self.icon = Self::read_string(format, reader).await?;
                    }
                }
                4 => {
                    if tag_count == 17 {
                        self.icon = Self::read_string(format, reader).await?;
                    }
                }
                5 => {
                    if tag_count == 35 {
                        self.icon = Self::read_string(format, reader).await?;
                    }
                }
                8 => {
                    if tag_count == 18 {
                        self.icon = Self::read_string(format, reader).await?;
                    }
                }*/
                11 => {
                    if tag_count == 17 {
                        self.using_recipe_type = Some(Self::read_string(format, reader).await?);
                    }
                }
                i => {
                    if icon_index.is_some_and(|f| f == i) {
                        self.icon = Self::read_string(format, reader).await?;
                    }
                }
            }
        }
        Ok(self)
    }
}
