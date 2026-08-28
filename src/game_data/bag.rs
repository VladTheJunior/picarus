use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::Rc,
    sync::Arc,
};

use crate::game_data::{
    Binding, Common, DataFormat, Grade, Item, ItemEffect, ItemTrait, ReadableItem, TagType, item_set::ItemSet, locale::Locale, product::Product,
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Default)]
pub struct Bag {
    pub common: Common,
    pub description_locale: Option<Locale>,
}

impl ReadableItem for Bag {
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
                0 => self.common.parse_id(reader, Self::FORMAT).await?,

                3 => self.common.parse_grade(reader).await?,
                4 => self.common.parse_required_level(reader).await?,
                5 => self.common.parse_item_level(reader).await?,

                11 => self.common.parse_no_trade(reader).await?,
                12 => self.common.parse_no_sell(reader).await?,
                13 => self.common.parse_no_destroy(reader).await?,

                15 => self.common.parse_binding(reader, Self::FORMAT).await?,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Bag {
    pub fn set_description_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.description_locale = locales.get(&SharedString::new(format!("{}_DESCRIPTION", self.common.id))).cloned();
    }
}


impl ItemTrait for Bag {
   fn common(&self) ->  &Common {
       &self.common
   }
}
