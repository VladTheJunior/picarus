use std::{
    io::{BufRead, Seek, SeekFrom},
    sync::Arc,
};

use crate::{
    game_data::{DataFormat, ReadExt, TagType, items::ReadableItem},
    language::LanguageController,
};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;

#[derive(Debug, Default, Clone)]
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

    fn debug_mut(&mut self) -> &mut Arc<Vec<u8>> {
        unimplemented!()
    }

    fn read<R: BufRead + Seek + ?Sized>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        let tag_count = definitions.len();
        for (tag_idx, (tag, _)) in definitions.iter().enumerate() {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match Self::FORMAT {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset))?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2))?;
                }
            };

            match tag.as_str() {
                "key" => {
                    self.key = {
                        let key = reader.read_string(Self::FORMAT)?.to_uppercase();
                        SharedString::new(key.strip_suffix("_NAME").unwrap_or(&key))
                    }
                }
                "eng" => self.eng = reader.read_string(Self::FORMAT)?,
                "rus" => self.rus = reader.read_string(Self::FORMAT)?,
                _ => {}
            }
        }
        Ok(self)
    }
}
