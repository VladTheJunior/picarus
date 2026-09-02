use std::{cell::RefCell, collections::BTreeMap, io::SeekFrom, rc::Rc};

use crate::{
    game_data::{
        AsyncBufReadExtReadString, DataFormat, Item, TagType,
        common::Common,
        items::{ItemNode, ItemTrait, ReadableItem},
    },
    game_data_view::PreviewBuilder,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::SharedString;

#[derive(Default, Clone)]
pub struct PackageItem {
    pub node: ItemNode,
    pub count: u16,
}

#[derive(Default, Clone)]
pub struct Package {
    pub debug: Vec<u8>,
    pub common: Common,

    pub package_items: IndexMap<usize, PackageItem>,
}

impl ReadableItem for Package {
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
        for (tag_idx, tag) in definitions.keys().enumerate() {
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
            self.common.parse(tag, reader, Self::FORMAT).await?;
            match tag_idx {
                23 => self.parse_package_id(0, reader, Self::FORMAT).await?,
                24 => self.parse_package_count(0, reader).await?,

                26 => self.parse_package_id(1, reader, Self::FORMAT).await?,
                27 => self.parse_package_count(1, reader).await?,

                29 => self.parse_package_id(2, reader, Self::FORMAT).await?,
                30 => self.parse_package_count(2, reader).await?,

                32 => self.parse_package_id(3, reader, Self::FORMAT).await?,
                33 => self.parse_package_count(3, reader).await?,

                35 => self.parse_package_id(4, reader, Self::FORMAT).await?,
                36 => self.parse_package_count(4, reader).await?,

                38 => self.parse_package_id(5, reader, Self::FORMAT).await?,
                39 => self.parse_package_count(5, reader).await?,

                41 => self.parse_package_id(6, reader, Self::FORMAT).await?,
                42 => self.parse_package_count(6, reader).await?,

                44 => self.parse_package_id(7, reader, Self::FORMAT).await?,
                45 => self.parse_package_count(7, reader).await?,

                47 => self.parse_package_id(8, reader, Self::FORMAT).await?,
                48 => self.parse_package_count(8, reader).await?,

                50 => self.parse_package_id(9, reader, Self::FORMAT).await?,
                51 => self.parse_package_count(9, reader).await?,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Package {
    async fn parse_package_id<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        &mut self,
        index: usize,
        reader: &mut R,
        format: DataFormat,
    ) -> Result<()> {
        let id = SharedString::new(reader.read_string(format).await?.to_uppercase());
        if id != "*" {
            self.package_items
                .entry(index)
                .and_modify(|f| f.node.id = id.clone())
                .or_insert(PackageItem {
                    node: ItemNode { id, item: None },
                    count: 0,
                });
        }
        Ok(())
    }

    async fn parse_package_count<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, index: usize, reader: &mut R) -> Result<()> {
        let count = reader.read_f32_le().await? as u16;

        self.package_items.entry(index).and_modify(|f| f.count = count);
        Ok(())
    }

    pub fn set_package_contents(&mut self, items: &IndexMap<SharedString, Rc<RefCell<Item>>>, unknown_ids: &mut BTreeMap<SharedString, u32>) {
        for (_, content) in &mut self.package_items {
            content.node.item = items.get(&content.node.id).map(|f| Rc::downgrade(f));
            if content.node.item.is_none() {
                unknown_ids.entry(content.node.id.clone()).and_modify(|count| *count += 1).or_insert(1);
            }
        }
    }
}

impl ItemTrait for Package {
    fn common(&self) -> &Common {
        &self.common
    }

    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common()).package_contents(self.package_items.clone())
    }
}
