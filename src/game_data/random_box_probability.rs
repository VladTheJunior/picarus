use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::Rc,
    sync::Arc,
};

use crate::game_data::{
    AsyncBufReadExtReadString, Binding, DataFormat, GameClass, Grade, Item, ItemEffect, ReadableItem, TagType, item_set::ItemSet, locale::Locale, product::Product,
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Default, Serialize, Clone)]
pub struct RandomBoxProbability {
    pub randomboxgroupid: SharedString,

    pub attributes: IndexMap<usize, f32>,
    pub probabilities: IndexMap<usize, f32>,
}

impl ReadableItem for RandomBoxProbability {


    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(item: &Self) -> Self::Key {
        item.randomboxgroupid.clone()
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
                0 => self.randomboxgroupid = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),

                1..101 => {
                    let v = reader.read_f32_le().await?;
                    if v != 0.0 {
                        self.attributes.insert(tag_idx - 1, v);
                    }
                }
                101..201 => {
                    let v = reader.read_f32_le().await?;
                    if v != 0.0 {
                        self.probabilities.insert(tag_idx - 101, v);
                    }
                }
                _ => {}
            }
        }

        Ok(self)
    }
}
