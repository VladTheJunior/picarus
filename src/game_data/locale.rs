use std::io::SeekFrom;

use crate::{
    game_data::TagType,
    game_data::{AsyncBufReadExtReadString, DataFormat, items::ReadableItem},
    language::LanguageController,
};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;
use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Default, Clone)]
pub struct Locale {
    pub key: SharedString,
    pub eng: SharedString,
    pub rus: SharedString,
}

impl Locale {
    pub fn locale(&self) -> Option<SharedString> {
        let language = LanguageController::get_current_language();

        let s = match language {
            crate::settings::Language::English => self.eng.clone(),
            crate::settings::Language::Russian => self.rus.clone(),
        };
        if s.is_empty() {
            return None;
        } else {
            return Some(s);
        }
    }
}

impl ReadableItem for Locale {
    const FORMAT: DataFormat = DataFormat::WideString;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.key.clone()
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
        for tag_idx in 0..tag_count {
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

            match tag_idx {
                2 => {
                    self.key = {
                        let key = reader.read_string(Self::FORMAT).await?.to_uppercase();
                        SharedString::new(key.strip_suffix("_NAME").unwrap_or(&key))
                    }
                }
                6 => self.eng = reader.read_string(Self::FORMAT).await?,
                7 => self.rus = reader.read_string(Self::FORMAT).await?,
                _ => {}
            }
        }
        Ok(self)
    }
}
