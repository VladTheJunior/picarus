use std::{cell::RefCell, collections::HashMap, io::SeekFrom, rc::Rc};

use crate::game_data::{
    AsyncBufReadExtReadString, DataFormat, Item, TagType, item::ItemNode, item::ReadableItem, random_box_probability::RandomBoxProbability,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

use gpui::SharedString;
use tracing::warn;

#[derive(Default, Clone)]
pub struct RandomBoxGroup {
    pub node: ItemNode,

    pub items: IndexMap<usize, ItemNode>,
    pub attributes: IndexMap<usize, f32>,
    pub probabilities: IndexMap<usize, f32>,
}

impl ReadableItem for RandomBoxGroup {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.node.id.clone()
    }

    type CollectionItem = Rc<RefCell<Self>>;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        Rc::new(RefCell::new(item))
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
                0 => self.node.id = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),

                10..110 => {
                    let id = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase());
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
    pub fn set_items(&mut self, items: &IndexMap<SharedString, Rc<Item>>, probabilities: &HashMap<SharedString, RandomBoxProbability>) {
        self.node.item = items.get(&self.node.id).map(|f| Rc::downgrade(f));
        if self.node.item.is_none() {
            warn!(id = ?self.node.id, "Failed to detect random box");
        }
        for (_, content) in &mut self.items {
            content.item = items.get(&content.id).map(|f| Rc::downgrade(f));
            if content.item.is_none() {
                warn!(?content.id, "Failed to detect random box content");
            }
        }

        if let Some(p) = probabilities.get(&self.node.id) {
            self.attributes = p.attributes.clone();
            self.probabilities = p.probabilities.clone();
        } else {
            warn!(id = ?self.node.id, "Failed to detect random box probabilities");
        }
    }
}
