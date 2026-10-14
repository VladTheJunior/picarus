use std::{
    collections::HashMap,
    io::{BufRead, Seek, SeekFrom},
    sync::Arc,
};

use crate::game_data::{DataFormat, ReadExt, TagType, items::ReadableItem};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;

#[derive(Default, Clone)]
pub struct ReverseTemperingRate {
    pub id: SharedString,
    pub condition: u8,
    pub rates: HashMap<u8, f32>,
}

impl ReadableItem for ReverseTemperingRate {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.id.clone()
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
        let mut index = 1;
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
                "리버스강화아이템" => {
                    self.id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                }
                "강화단계제한" => self.condition = reader.read_f32_le()? as u8,
                "강화단계1" | "강화단계2" | "강화단계3" | "강화단계4" | "강화단계5" => {
                    index = reader.read_f32_le()? as u8;
                }
                "강화확률1" | "강화확률2" | "강화확률3" | "강화확률4" | "강화확률5" => {
                    let rate = reader.read_f32_le()?;

                    self.rates.insert(index, rate);
                }

                _ => {}
            }
        }
        Ok(self)
    }
}
