use std::{ collections::BTreeMap, io::{BufRead, Seek, SeekFrom}, sync::Arc};

use crate::game_data::{
     DataFormat, ReadExt, TagType, items::{Item, ItemNode, ReadableItem},
};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;
use parking_lot::Mutex;


#[derive(Default, Clone)]
pub struct SynthesisParts {
    pub itemid: ItemNode,
    pub itemcnt: u16,
    pub resultid: SharedString,
    pub cost: f32,
    pub successrate: f32,
    pub delblock_max: f32,
}

impl ReadableItem for SynthesisParts {
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
                "itemid" => self.itemid.id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase()),
                "itemcnt" => self.itemcnt = reader.read_f32_le()? as u16,
                "resultid" => self.resultid = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase()),
                "cost" => self.cost = reader.read_f32_le()?,
                "successrate" => self.successrate = reader.read_f32_le()?,
                "delblock_max" => self.delblock_max = reader.read_f32_le()?,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl SynthesisParts {
    pub fn set_parts(&mut self, items: &IndexMap<SharedString, Arc<Mutex<Item>>>, unknown_ids: &mut BTreeMap<SharedString, u32>) {
        self.itemid.item = items.get(&self.itemid.id).map(|f| Arc::downgrade(f));
        if self.itemid.item.is_none() {
            unknown_ids.entry(self.itemid.id.clone()).and_modify(|count| *count += 1).or_insert(1);
        }
    }
}
