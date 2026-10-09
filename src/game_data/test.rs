use std::io::{BufRead, Seek, SeekFrom};

use crate::game_data::{ DataFormat, ReadExt, TagType, items::ReadableItem};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;


#[derive(Default, Clone)]
pub struct Test {}

impl ReadableItem for Test {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = u16;

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

    fn read<R: BufRead + Seek + ?Sized>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        let tag_count = definitions.len();
        for (tag_idx, (tag, tag_type)) in definitions.iter().enumerate() {
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
            match tag_type {
                TagType::String => println!("{tag:?}: {:?}", reader.read_string(Self::FORMAT)?),
                TagType::Float => println!("{tag:?}: {}", reader.read_f32_le()?),
            };
        }
        Ok(self)
    }
}
