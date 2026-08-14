use std::io::SeekFrom;

use crate::game_data::{AbstractItem, DataFormat};
use anyhow::Result;
use gpui::SharedString;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Debug, Default, Clone, Serialize)]
pub struct Locale {
    //_type: SharedString,
    //typename: SharedString,
    pub key: SharedString,
    //korean: SharedString,
    //jpn: SharedString,
    //chinese: SharedString,
    pub eng: SharedString,
    pub rus: SharedString,
}
impl AbstractItem for Locale {
    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        tag_count: usize,
        global_offset: u64,
        format: DataFormat,
    ) -> Result<Self> {
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
                //0 => self._type = Self::read_string(format, reader)?,
                //1 => self.typename = Self::read_string(format, reader)?,
                2 => {
                    self.key = {
                        let key = Self::read_string(format, reader).await?.to_uppercase();
                        SharedString::new(key.strip_suffix("_NAME").unwrap_or(&key))
                    }
                }
                //3 => self.korean = Self::read_string(format, reader)?,
                //4 => self.jpn = Self::read_string(format, reader)?,
                //5 => self.chinese = Self::read_string(format, reader)?,
                6 => self.eng = Self::read_string(format, reader).await?,
                7 => self.rus = Self::read_string(format, reader).await?,
                _ => {}
            }
        }
        Ok(self)
    }
}
