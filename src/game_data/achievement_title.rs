use std::io::{BufRead, Seek, SeekFrom};

use crate::game_data::{DataFormat, ReadExt, TagType, effects::ItemEffect, items::ReadableItem};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;


#[derive(Default, Clone)]
pub struct AchievementTitle {
    pub id: SharedString,
    pub effects: Vec<ItemEffect>,
}

impl ReadableItem for AchievementTitle {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.id.clone()
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
                "id" => {
                    self.id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                }
                "장착효과1" | "장착효과2" | "장착효과3" | "장착효과4" => {
                    let effect = reader.read_string(Self::FORMAT)?;
                    if effect != "*" && effect != "0" {
                        self.effects.push(ItemEffect::new(effect));
                    }
                }

                _ => {}
            }
        }
        Ok(self)
    }
}
