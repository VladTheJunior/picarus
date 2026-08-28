use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::Rc,
    sync::Arc,
};

use crate::game_data::{
    Binding, Common, DataFormat, GameClass, Grade, Item, ItemEffect, ItemTrait, ReadableItem, TagType, item_set::ItemSet, locale::Locale, product::Product,
};
use anyhow::Result;
use gpui::{Image, SharedString};
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tracing::warn;

#[derive(Default)]
pub struct FellowEquip {
    pub common: Common,

    pub max_ep_plus: Option<f32>,
}
impl ReadableItem for FellowEquip {
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
        // Read all fields sequentially
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

                3 => self.common.parse_required_level(reader).await?,

                5 => self.common.parse_item_level(reader).await?,
                6 => self.common.parse_usable_class(reader, Self::FORMAT).await?,
                7 => self.common.parse_grade(reader).await?,

                12 => {
                    let v = reader.read_f32_le().await?;

                    if v != 0.0 {
                        self.max_ep_plus = Some(v);
                    }
                }
                13 => self.common.parse_effect(reader, Self::FORMAT).await?,
                14 => self.common.parse_effect(reader, Self::FORMAT).await?,
                15 => self.common.parse_effect(reader, Self::FORMAT).await?,
                16 => self.common.parse_effect(reader, Self::FORMAT).await?,

                24 => self.common.parse_no_trade(reader).await?,
                25 => self.common.parse_no_sell(reader).await?,
                26 => self.common.parse_no_destroy(reader).await?,

                28 => self.common.parse_binding(reader, Self::FORMAT).await?,

                _ => {}
            }
        }

        Ok(self)
    }
}


impl ItemTrait for FellowEquip {
   fn common(&self) ->  &Common {
       &self.common
   }
}
