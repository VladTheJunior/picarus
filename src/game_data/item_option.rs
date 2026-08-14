use std::io::SeekFrom;

use crate::game_data::{AbstractItem, DataFormat, ItemMinMaxEffect};
use anyhow::Result;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Debug, Default)]
pub struct ItemOption {
    pub level: u16,
    pub effect1: Vec<ItemMinMaxEffect>,
    pub effect2: Vec<ItemMinMaxEffect>,
    pub special_effect: Vec<ItemMinMaxEffect>,
    pub wr_effect1: Vec<ItemMinMaxEffect>,
    pub wr_effect2: Vec<ItemMinMaxEffect>,
    pub gd_effect1: Vec<ItemMinMaxEffect>,
    pub gd_effect2: Vec<ItemMinMaxEffect>,
    pub tf_effect1: Vec<ItemMinMaxEffect>,
    pub tf_effect2: Vec<ItemMinMaxEffect>,
    pub pr_effect1: Vec<ItemMinMaxEffect>,
    pub pr_effect2: Vec<ItemMinMaxEffect>,
    pub wz_effect1: Vec<ItemMinMaxEffect>,
    pub wz_effect2: Vec<ItemMinMaxEffect>,
    pub ac_effect1: Vec<ItemMinMaxEffect>,
    pub ac_effect2: Vec<ItemMinMaxEffect>,
    pub do_effect1: Vec<ItemMinMaxEffect>,
    pub do_effect2: Vec<ItemMinMaxEffect>,
}

impl AbstractItem for ItemOption {
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
                    self.level = reader.read_f32_le().await? as u16;
                }
                1 => {
                    let effects = Self::read_string(format, reader).await?;

                    self.effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                2 => {
                    let effects = Self::read_string(format, reader).await?;

                    self.effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }

                3 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.special_effect = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                4 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.wr_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                5 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.wr_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                6 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.gd_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                7 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.gd_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                8 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.tf_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                9 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.tf_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                10 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.pr_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                11 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.pr_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                12 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.wz_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                13 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.wz_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                14 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.ac_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                15 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.ac_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }

                16 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.do_effect1 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                17 => {
                    let effects = Self::read_string(format, reader).await?;
                    self.do_effect2 = effects.split(",").map(|e| ItemMinMaxEffect::new(e)).collect();
                }
                _ => {}
            }
        }
        Ok(self)
    }
}
