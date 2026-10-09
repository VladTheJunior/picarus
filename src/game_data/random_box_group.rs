use std::{

    collections::{BTreeMap, HashMap}, io::{BufRead, Seek, SeekFrom}, sync::Arc,
};

use crate::game_data::{
    DataFormat, Item, ReadExt, TagType, items::{ItemNode, ReadableItem}, random_box_probability::RandomBoxProbability,
};
use anyhow::Result;
use indexmap::IndexMap;

use parking_lot::Mutex;

use gpui_kit::SharedString;
use tracing::warn;

#[derive(Default, Clone)]
pub struct RandomBoxGroup {
    pub id: SharedString,

    pub items: IndexMap<usize, ItemNode>,
    pub attributes: IndexMap<usize, f32>,
    pub probabilities: IndexMap<usize, f32>,
}

impl ReadableItem for RandomBoxGroup {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.id.clone()
    }

    type CollectionItem = Arc<Mutex<Self>>;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        Arc::new(Mutex::new(item))
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

            match tag_idx {
                0 => self.id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase()),

                10..110 => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase());
                    if id != "*" {
                        self.items.insert(tag_idx - 10, ItemNode { id, item: None });
                    }
                }
                _ => {}
            }
        }

        Ok(self)
    }
}

impl RandomBoxGroup {
    pub fn set_items(
        &mut self,
        items: &IndexMap<SharedString, Arc<Mutex<Item>>>,
        probabilities: &HashMap<SharedString, RandomBoxProbability>,
        unknown_ids: &mut BTreeMap<SharedString, u32>,
    ) {
        for (_, content) in &mut self.items {
            content.item = items.get(&content.id).map(|f| Arc::downgrade(f));
            if content.item.is_none() {
                unknown_ids.entry(content.id.clone()).and_modify(|count| *count += 1).or_insert(1);
            }
        }

        if let Some(p) = probabilities.get(&self.id) {
            self.attributes = p.attributes.clone();
            self.probabilities = p.probabilities.clone();
        } else {
            warn!(id = ?self.id, "Failed to detect random box probabilities");
        }
    }
}
