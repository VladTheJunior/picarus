use std::{
    cell::RefCell,
    collections::HashMap,
    io::SeekFrom,
    rc::{Rc, Weak},
};

use crate::{
    game_data::{
        AsyncBufReadExtReadString, DataFormat, TagType,
        common::Common,
        items::{ItemTrait, ReadableItem},
        random_box_group::RandomBoxGroup,
    },
    game_data_view::PreviewBuilder,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

use gpui_kit::SharedString;

#[derive(Default, Clone)]
pub struct RandomBox {
    pub debug: Vec<u8>,
    pub content: Option<Weak<RefCell<RandomBoxGroup>>>,

    pub common: Common,
    random_item_id: SharedString,
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
        for (tag_idx, (tag, tag_type)) in definitions.iter().enumerate() {
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
            self.common.parse(tag, tag_type, reader, Self::FORMAT).await?;

            match tag.as_str() {
                "랜덤아이템id" => self.random_item_id = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),

                _ => {}
            }
        }

        Ok(self)
    }
}

impl RandomBox {
    pub fn set_random_box_group(&mut self, random_box_groups: &HashMap<SharedString, Rc<RefCell<RandomBoxGroup>>>) {
        self.content = random_box_groups.get(&self.random_item_id).map(|f| Rc::downgrade(f));
    }
}

impl ItemTrait for RandomBox {
    fn common(&self) -> &Common {
        &self.common
    }
        fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }
    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common()).random_box_group(self.content.clone())
    }
}
