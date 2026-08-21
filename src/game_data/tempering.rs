use std::io::SeekFrom;

use crate::game_data::{AbstractItem, DataFormat, TagType};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Default, Serialize, Clone)]
pub struct Tempering {
    pub level: u16,
    pub attack_ratios: [f32; 30],
    pub defenses: [f32; 30],
    pub defense_ratios: [f32; 30],
    pub spell_ratios: [f32; 30],
}

impl AbstractItem for Tempering {
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
            if tag_count == 121 {
                match tag_idx {
                    0 => self.level = reader.read_f32_le().await? as u16,
                    1..31 => self.attack_ratios[tag_idx - 1] = reader.read_f32_le().await?,
                    31..61 => self.defenses[tag_idx - 31] = reader.read_f32_le().await?,
                    61..91 => self.defense_ratios[tag_idx - 61] = reader.read_f32_le().await?,
                    91..121 => self.spell_ratios[tag_idx - 91] = reader.read_f32_le().await?,
                    _ => {}
                }
            } else if tag_count == 106 || tag_count == 61 || tag_count == 101 || tag_count == 111  {
                match tag_idx {
                    0 => self.level = reader.read_f32_le().await? as u16,
                    1..31 => self.defenses[tag_idx - 1] = reader.read_f32_le().await?,
                    31..61 => self.defense_ratios[tag_idx - 31] = reader.read_f32_le().await?,
                    _ => {}
                }
            }
        }
        Ok(self)
    }
}
