use std::{
    collections::BTreeMap, io::SeekFrom, rc::{Rc, Weak},
};

use crate::game_data::{AsyncBufReadExtReadString, DataFormat, Item, ItemNode, ReadableItem, TagType};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::SharedString;
use tracing::warn;

#[derive(Clone)]

pub struct MaterialItem {
    pub node: ItemNode,
    pub count: u16,
    pub additional_node: Option<ItemNode>,
    pub additional_count: u16,
}

#[derive(Default, Clone)]
pub struct Product {
    pub debug: Vec<u8>,
    pub node: ItemNode,
    pub productid: SharedString,
    pub technology_grade: u8,
    pub multiproduct: bool,
    pub materials: IndexMap<usize, MaterialItem>,
    pub success_probability: f32,
    pub inheritance_on_craft: bool,
    pub inheritance_enhancement_condition: u8,
    pub inheritance_transcendence_condition: u8,
}

impl ReadableItem for Product {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(item: &Self) -> Self::Key {
        item.node.id.clone()
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        self.debug = Self::parse_debug(reader, offsets, item_idx, definitions, global_offset).await?;
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
                0 => {
                    self.node = ItemNode {
                        id: SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),
                        item: None,
                    }
                }
                1 => self.productid = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),

                4 => self.technology_grade = reader.read_f32_le().await? as u8,
                5 => self.multiproduct = reader.read_f32_le().await? != 0.0,
                6 => self.parse_material_id(0, reader, Self::FORMAT).await?,
                7 => self.parse_material_count(0, reader).await?,
                8 => self.parse_material_additional_id(0, reader, Self::FORMAT).await?,
                9 => self.parse_material_additional_count(0, reader).await?,
                10 => self.parse_material_id(1, reader, Self::FORMAT).await?,
                11 => self.parse_material_count(1, reader).await?,
                12 => self.parse_material_additional_id(1, reader, Self::FORMAT).await?,
                13 => self.parse_material_additional_count(1, reader).await?,
                14 => self.parse_material_id(2, reader, Self::FORMAT).await?,
                15 => self.parse_material_count(2, reader).await?,
                16 => self.parse_material_additional_id(2, reader, Self::FORMAT).await?,
                17 => self.parse_material_additional_count(2, reader).await?,
                18 => self.parse_material_id(3, reader, Self::FORMAT).await?,
                19 => self.parse_material_count(3, reader).await?,
                20 => self.parse_material_additional_id(3, reader, Self::FORMAT).await?,
                21 => self.parse_material_additional_count(3, reader).await?,
                22 => self.parse_material_id(4, reader, Self::FORMAT).await?,
                23 => self.parse_material_count(4, reader).await?,
                24 => self.parse_material_additional_id(4, reader, Self::FORMAT).await?,
                25 => self.parse_material_additional_count(4, reader).await?,

                30 => self.success_probability = reader.read_f32_le().await?,

                42 => self.inheritance_on_craft = reader.read_f32_le().await? != 0.0,
                43 => self.inheritance_enhancement_condition = reader.read_f32_le().await? as u8,
                44 => self.inheritance_transcendence_condition = reader.read_f32_le().await? as u8,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Product {
    async fn parse_material_id<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        &mut self,
        index: usize,
        reader: &mut R,
        format: DataFormat,
    ) -> Result<()> {
        let id = SharedString::new(reader.read_string(format).await?.to_uppercase());
        if id != "*" {
            self.materials
                .entry(index)
                .and_modify(|f| f.node.id = id.clone())
                .or_insert(MaterialItem {
                    node: ItemNode { id, item: None },
                    count: 0,
                    additional_node: None,
                    additional_count: 0,
                });
        }
        Ok(())
    }

    async fn parse_material_additional_id<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        &mut self,
        index: usize,
        reader: &mut R,
        format: DataFormat,
    ) -> Result<()> {
        let id = SharedString::new(reader.read_string(format).await?.to_uppercase());
        if id != "*" {
            self.materials
                .entry(index)
                .and_modify(|f| f.additional_node = Some(ItemNode { id, item: None }));
        }
        Ok(())
    }

    async fn parse_material_count<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, index: usize, reader: &mut R) -> Result<()> {
        let count = reader.read_f32_le().await? as u16;

        self.materials.entry(index).and_modify(|f| f.count = count);
        Ok(())
    }

    async fn parse_material_additional_count<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        &mut self,
        index: usize,
        reader: &mut R,
    ) -> Result<()> {
        let count = reader.read_f32_le().await? as u16;

        self.materials.entry(index).and_modify(|f| f.additional_count = count);
        Ok(())
    }

    pub fn set_materials(&mut self, items: &IndexMap<SharedString, Rc<Item>>) {
        self.node.item = items.get(&self.node.id).map(|f| Rc::downgrade(f));
        if self.node.item.is_none() {
            warn!(id = ?self.node.id, "Failed to detect product result");
        }

        for (_, m) in self.materials.iter_mut() {
            m.node.item = items.get(&m.node.id).map(|f| Rc::downgrade(f));
            if m.node.item.is_none() {
                warn!(?m.node.id, "Failed to detect product material");
            }

            if let Some(node) = m.additional_node.as_mut() {
                node.item = items.get(&node.id).map(|f| Rc::downgrade(f));
                if node.item.is_none() {
                    warn!(?node.id, "Failed to detect product material");
                }
            }
        }
    }
}
