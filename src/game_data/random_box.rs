use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::{Rc, Weak},
    sync::Arc,
};

use crate::game_data::{
     Binding, Common, DataFormat, GameClass, Grade, Item, ItemEffect, ItemTrait, ReadableItem, TagType, item_set::ItemSet, locale::Locale, product::Product, random_box_group::RandomBoxGroup,
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Default)]
pub struct RandomBox {
    pub content: Option<Weak<RefCell<RandomBoxGroup>>>,

    pub common: Common,
}

impl ReadableItem for RandomBox {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(item: &Self) -> Self::Key {
        item.common.id.clone()
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        self.common.debug = Self::parse_debug(reader, offsets, item_idx, definitions, global_offset).await?;
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
   fn common(&self) ->  &Common {
       &self.common
   }
}