use std::{
    io::{BufRead, Seek, SeekFrom},
    sync::Arc,
};

use crate::{
    game_data::{
        DataFormat, ReadExt, TagType,
        common::Common,
        items::{ItemTrait, ReadableItem},
    },
    game_data_view::PreviewBuilder,
};
use anyhow::Result;
use indexmap::IndexMap;

use gpui_kit::SharedString;

#[derive(Default, Clone)]
pub struct PackageItem {
    pub id: SharedString,
    pub count: u16,
}

#[derive(Default, Clone)]
pub struct Package {
    pub debug: Arc<Vec<u8>>,
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

    fn debug_mut(&mut self) -> &mut Arc<Vec<u8>> {
        &mut self.debug
    }

    fn read<R: BufRead + Seek + ?Sized>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        self.parse_debug(reader, offsets, item_idx, definitions, global_offset)?;
        let tag_count = definitions.len();
        for (tag_idx, (tag, tag_type)) in definitions.iter().enumerate() {
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
            self.common.parse(tag, tag_type, reader, Self::FORMAT)?;
            match tag_idx {
                23 => self.parse_package_id(0, reader, Self::FORMAT)?,
                24 => self.parse_package_count(0, reader)?,

                26 => self.parse_package_id(1, reader, Self::FORMAT)?,
                27 => self.parse_package_count(1, reader)?,

                29 => self.parse_package_id(2, reader, Self::FORMAT)?,
                30 => self.parse_package_count(2, reader)?,

                32 => self.parse_package_id(3, reader, Self::FORMAT)?,
                33 => self.parse_package_count(3, reader)?,

                35 => self.parse_package_id(4, reader, Self::FORMAT)?,
                36 => self.parse_package_count(4, reader)?,

                38 => self.parse_package_id(5, reader, Self::FORMAT)?,
                39 => self.parse_package_count(5, reader)?,

                41 => self.parse_package_id(6, reader, Self::FORMAT)?,
                42 => self.parse_package_count(6, reader)?,

                44 => self.parse_package_id(7, reader, Self::FORMAT)?,
                45 => self.parse_package_count(7, reader)?,

                47 => self.parse_package_id(8, reader, Self::FORMAT)?,
                48 => self.parse_package_count(8, reader)?,

                50 => self.parse_package_id(9, reader, Self::FORMAT)?,
                51 => self.parse_package_count(9, reader)?,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Package {
    fn parse_package_id<R: BufRead + ?Sized>(&mut self, index: usize, reader: &mut R, format: DataFormat) -> Result<()> {
        let id = SharedString::new(reader.read_string(format)?.to_uppercase());
        if id != "*" {
            self.package_items
                .entry(index)
                .and_modify(|f| f.id = id.clone())
                .or_insert(PackageItem { id, count: 0 });
        }
        Ok(())
    }

    fn parse_package_count<R: BufRead + ?Sized>(&mut self, index: usize, reader: &mut R) -> Result<()> {
        let count = reader.read_f32_le()? as u16;

        self.package_items.entry(index).and_modify(|f| f.count = count);
        Ok(())
    }
}

impl ItemTrait for Package {
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
        PreviewBuilder::new(self.common()).package_contents(self.package_items.clone())
    }
}
