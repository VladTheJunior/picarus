use std::{
    io::{BufRead, Seek, SeekFrom},
    sync::Arc,
};

use crate::game_data::{DataFormat, ReadExt, TagType, items::ReadableItem};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;

#[derive(Default, Clone)]
pub struct Tempering {
    pub level: u16,
    pub attack_ratios: [f32; 30],
    pub defenses: [f32; 30],
    pub defense_ratios: [f32; 30],
    pub spell_ratios: [f32; 30],
}

impl ReadableItem for Tempering {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = u16;

    fn key(&self) -> Self::Key {
        self.level
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
        for tag_idx in 0..tag_count {
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
            if tag_count == 121 {
                match tag_idx {
                    0 => self.level = reader.read_f32_le()? as u16,
                    1..31 => self.attack_ratios[tag_idx - 1] = reader.read_f32_le()?,
                    31..61 => self.defenses[tag_idx - 31] = reader.read_f32_le()?,
                    61..91 => self.defense_ratios[tag_idx - 61] = reader.read_f32_le()?,
                    91..121 => self.spell_ratios[tag_idx - 91] = reader.read_f32_le()?,
                    _ => {}
                }
            } else if tag_count == 106 || tag_count == 61 || tag_count == 101 || tag_count == 111 {
                match tag_idx {
                    0 => self.level = reader.read_f32_le()? as u16,
                    1..31 => self.defenses[tag_idx - 1] = reader.read_f32_le()?,
                    31..61 => self.defense_ratios[tag_idx - 31] = reader.read_f32_le()?,
                    _ => {}
                }
            }
        }
        Ok(self)
    }
}
