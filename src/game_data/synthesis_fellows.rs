use std::io::SeekFrom;

use crate::game_data::{AsyncBufReadExtReadString, DataFormat, TagType, items::ReadableItem};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Default, Debug, Clone)]
pub struct SynthesisFellows {
    pub grade: f32,
    pub fellows: IndexMap<u8, SynthesisFellow>,
}

#[derive(Debug, Clone)]
pub struct SynthesisFellow {
    pub id: SharedString,
    pub rate: f32,
}

impl ReadableItem for SynthesisFellows {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        unimplemented!()
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
        let tag_count = definitions.len();
        let mut fellow_ids: [SharedString; 30] = std::array::from_fn(|_| SharedString::default());
        let mut fellow_rates = [0.0f32; 30];
        for (tag_idx, (tag, _)) in definitions.iter().enumerate() {
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
                "grade" => self.grade = reader.read_f32_le().await?,

                "fellowid01" => fellow_ids[0] = reader.read_string(Self::FORMAT).await?,
                "fellowid02" => fellow_ids[1] = reader.read_string(Self::FORMAT).await?,
                "fellowid03" => fellow_ids[2] = reader.read_string(Self::FORMAT).await?,
                "fellowid04" => fellow_ids[3] = reader.read_string(Self::FORMAT).await?,
                "fellowid05" => fellow_ids[4] = reader.read_string(Self::FORMAT).await?,
                "fellowid06" => fellow_ids[5] = reader.read_string(Self::FORMAT).await?,
                "fellowid07" => fellow_ids[6] = reader.read_string(Self::FORMAT).await?,
                "fellowid08" => fellow_ids[7] = reader.read_string(Self::FORMAT).await?,
                "fellowid09" => fellow_ids[8] = reader.read_string(Self::FORMAT).await?,
                "fellowid10" => fellow_ids[9] = reader.read_string(Self::FORMAT).await?,
                "fellowid11" => fellow_ids[10] = reader.read_string(Self::FORMAT).await?,
                "fellowid12" => fellow_ids[11] = reader.read_string(Self::FORMAT).await?,
                "fellowid13" => fellow_ids[12] = reader.read_string(Self::FORMAT).await?,
                "fellowid14" => fellow_ids[13] = reader.read_string(Self::FORMAT).await?,
                "fellowid15" => fellow_ids[14] = reader.read_string(Self::FORMAT).await?,
                "fellowid16" => fellow_ids[15] = reader.read_string(Self::FORMAT).await?,
                "fellowid17" => fellow_ids[16] = reader.read_string(Self::FORMAT).await?,
                "fellowid18" => fellow_ids[17] = reader.read_string(Self::FORMAT).await?,
                "fellowid19" => fellow_ids[18] = reader.read_string(Self::FORMAT).await?,
                "fellowid20" => fellow_ids[19] = reader.read_string(Self::FORMAT).await?,
                "fellowid21" => fellow_ids[20] = reader.read_string(Self::FORMAT).await?,
                "fellowid22" => fellow_ids[21] = reader.read_string(Self::FORMAT).await?,
                "fellowid23" => fellow_ids[22] = reader.read_string(Self::FORMAT).await?,
                "fellowid24" => fellow_ids[23] = reader.read_string(Self::FORMAT).await?,
                "fellowid25" => fellow_ids[24] = reader.read_string(Self::FORMAT).await?,
                "fellowid26" => fellow_ids[25] = reader.read_string(Self::FORMAT).await?,
                "fellowid27" => fellow_ids[26] = reader.read_string(Self::FORMAT).await?,
                "fellowid28" => fellow_ids[27] = reader.read_string(Self::FORMAT).await?,
                "fellowid29" => fellow_ids[28] = reader.read_string(Self::FORMAT).await?,
                "fellowid30" => fellow_ids[29] = reader.read_string(Self::FORMAT).await?,

                "fellowid01_rate" => fellow_rates[0] = reader.read_f32_le().await?,
                "fellowid02_rate" => fellow_rates[1] = reader.read_f32_le().await?,
                "fellowid03_rate" => fellow_rates[2] = reader.read_f32_le().await?,
                "fellowid04_rate" => fellow_rates[3] = reader.read_f32_le().await?,
                "fellowid05_rate" => fellow_rates[4] = reader.read_f32_le().await?,
                "fellowid06_rate" => fellow_rates[5] = reader.read_f32_le().await?,
                "fellowid07_rate" => fellow_rates[6] = reader.read_f32_le().await?,
                "fellowid08_rate" => fellow_rates[7] = reader.read_f32_le().await?,
                "fellowid09_rate" => fellow_rates[8] = reader.read_f32_le().await?,
                "fellowid10_rate" => fellow_rates[9] = reader.read_f32_le().await?,
                "fellowid11_rate" => fellow_rates[10] = reader.read_f32_le().await?,
                "fellowid12_rate" => fellow_rates[11] = reader.read_f32_le().await?,
                "fellowid13_rate" => fellow_rates[12] = reader.read_f32_le().await?,
                "fellowid14_rate" => fellow_rates[13] = reader.read_f32_le().await?,
                "fellowid15_rate" => fellow_rates[14] = reader.read_f32_le().await?,
                "fellowid16_rate" => fellow_rates[15] = reader.read_f32_le().await?,
                "fellowid17_rate" => fellow_rates[16] = reader.read_f32_le().await?,
                "fellowid18_rate" => fellow_rates[17] = reader.read_f32_le().await?,
                "fellowid19_rate" => fellow_rates[18] = reader.read_f32_le().await?,
                "fellowid20_rate" => fellow_rates[19] = reader.read_f32_le().await?,
                "fellowid21_rate" => fellow_rates[20] = reader.read_f32_le().await?,
                "fellowid22_rate" => fellow_rates[21] = reader.read_f32_le().await?,
                "fellowid23_rate" => fellow_rates[22] = reader.read_f32_le().await?,
                "fellowid24_rate" => fellow_rates[23] = reader.read_f32_le().await?,
                "fellowid25_rate" => fellow_rates[24] = reader.read_f32_le().await?,
                "fellowid26_rate" => fellow_rates[25] = reader.read_f32_le().await?,
                "fellowid27_rate" => fellow_rates[26] = reader.read_f32_le().await?,
                "fellowid28_rate" => fellow_rates[27] = reader.read_f32_le().await?,
                "fellowid29_rate" => fellow_rates[28] = reader.read_f32_le().await?,
                "fellowid30_rate" => fellow_rates[29] = reader.read_f32_le().await?,

                _ => {}
            }
        }
        for i in 0..30 {
            if fellow_ids[i] != "*" && !fellow_ids[i].is_empty() {
                self.fellows.insert(
                    i as u8,
                    SynthesisFellow {
                        id: SharedString::new(fellow_ids[i].to_uppercase()),
                        rate: fellow_rates[i],
                    },
                );
            }
        }
        Ok(self)
    }
}
