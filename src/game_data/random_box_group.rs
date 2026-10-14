use std::{
    collections::HashMap,
    io::{BufRead, Seek, SeekFrom},
    sync::Arc,
};

use crate::game_data::{DataFormat, ReadExt, TagType, items::ReadableItem, random_box_probability::RandomBoxProbability};
use anyhow::Result;
use indexmap::IndexMap;

use gpui_kit::SharedString;
use tracing::warn;

#[derive(Default, Clone)]
pub struct RandomBoxGroup {
    pub id: SharedString,

    pub items: IndexMap<usize, SharedString>,
    pub probabilities: IndexMap<usize, f32>,
}

impl ReadableItem for RandomBoxGroup {
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
                        self.items.insert(tag_idx - 10, id);
                    }
                }
                _ => {}
            }
        }

        Ok(self)
    }
}

impl RandomBoxGroup {
    pub fn set_probabilities(&mut self, probabilities: &HashMap<SharedString, RandomBoxProbability>) {
        if let Some(p) = probabilities.get(&self.id) {
            self.probabilities = p.probabilities.clone();
        } else {
            warn!(id = ?self.id, "Failed to detect random box probabilities");
        }
    }
}
