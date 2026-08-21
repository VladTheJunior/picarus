use std::io::SeekFrom;

use crate::game_data::{AbstractItem, DataFormat, ItemEffect, TagType};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Default, Serialize, Clone)]
pub struct ItemQuality {
    pub level: u16,
    pub intermediate_fixed_effect: Option<ItemEffect>,
    pub intermediate_variable_effect_bonus: f32,
    pub advanced_fixed_effect: Option<ItemEffect>,
    pub advanced_variable_effect_bonus: f32,
}

impl AbstractItem for ItemQuality {
    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<String, TagType>,
        global_offset: u64,
        format: DataFormat,
    ) -> Result<Self> {
        let tag_count = definitions.len();
        // Read all fields sequentially
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
                0 => self.level = reader.read_f32_le().await? as u16,

                1 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.intermediate_fixed_effect = Some(ItemEffect::new(effect));
                    }
                }
                2 => self.intermediate_variable_effect_bonus = reader.read_f32_le().await?,
                3 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.advanced_fixed_effect = Some(ItemEffect::new(effect));
                    }
                }
                4 => self.advanced_variable_effect_bonus = reader.read_f32_le().await?,

                _ => {}
            }
        }
        Ok(self)
    }
}
