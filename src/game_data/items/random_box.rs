use std::{
    cell::RefCell,
    collections::HashMap,
    io::SeekFrom,
    rc::{Rc, Weak},
};

use crate::game_data::{DataFormat, TagType, common::Common, item::ItemTrait, item::ReadableItem, random_box_group::RandomBoxGroup};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

use gpui::SharedString;

#[derive(Default)]
pub struct RandomBox {
    pub debug: Vec<u8>,
    pub content: Option<Weak<RefCell<RandomBoxGroup>>>,

    pub common: Common,
}

impl ReadableItem for RandomBox {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.common.id.clone()
    }

    type CollectionItem = Self;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        item
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
        &mut self.debug
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        self.parse_debug(reader, offsets, item_idx, definitions, global_offset).await?;
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
                3 => self.common.parse_usable_class(reader, Self::FORMAT).await?,
                4 => self.common.parse_grade(reader).await?,
                5 => self.common.parse_required_level(reader).await?,

                7 => self.common.parse_item_level(reader).await?,

                19 => self.common.parse_no_trade(reader).await?,
                20 => self.common.parse_no_sell(reader).await?,
                21 => self.common.parse_no_destroy(reader).await?,
                22 => self.common.parse_binding(reader, Self::FORMAT).await?,

                24 => self.common.parse_id(reader, Self::FORMAT).await?,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl RandomBox {
    pub fn set_random_box_group(&mut self, random_box_groups: &HashMap<SharedString, Rc<RefCell<RandomBoxGroup>>>) {
        self.content = random_box_groups.get(&self.common.id).map(|f| Rc::downgrade(f));
    }
}

impl ItemTrait for RandomBox {
    fn common(&self) -> &Common {
        &self.common
    }

    fn debug(&self) -> &[u8] {
        &self.debug
    }
}
