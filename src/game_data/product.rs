use std::{collections::BTreeMap, io::{BufRead, Seek, SeekFrom}, sync::Arc};

use crate::game_data::{ DataFormat, Item, ReadExt, TagType, items::{ItemNode, ReadableItem}};
use anyhow::Result;
use indexmap::IndexMap;

use parking_lot::Mutex;


use gpui_kit::SharedString;

#[derive(Clone)]

pub struct MaterialItem {
    pub node: ItemNode,
    pub count: u16,
    pub additional_node: Option<ItemNode>,
    pub additional_count: u16,
}

#[derive(Default, Clone)]
pub struct Product {
    pub recipe: Option<SharedString>,
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

    fn key(&self) -> Self::Key {
        self.node.id.clone()
    }

    type CollectionItem = Arc<Mutex<Self>>;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        Arc::new(Mutex::new(item))
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
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
        for tag_idx in 0..tag_count {
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

            match tag_idx {
                0 => {
                    self.node = ItemNode {
                        id: SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase()),
                        item: None,
                    }
                }
                1 => self.productid = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase()),

                4 => self.technology_grade = reader.read_f32_le()? as u8,
                5 => self.multiproduct = reader.read_f32_le()? != 0.0,
                6 => self.parse_material_id(0, reader, Self::FORMAT)?,
                7 => self.parse_material_count(0, reader)?,
                8 => self.parse_material_additional_id(0, reader, Self::FORMAT)?,
                9 => self.parse_material_additional_count(0, reader)?,
                10 => self.parse_material_id(1, reader, Self::FORMAT)?,
                11 => self.parse_material_count(1, reader)?,
                12 => self.parse_material_additional_id(1, reader, Self::FORMAT)?,
                13 => self.parse_material_additional_count(1, reader)?,
                14 => self.parse_material_id(2, reader, Self::FORMAT)?,
                15 => self.parse_material_count(2, reader)?,
                16 => self.parse_material_additional_id(2, reader, Self::FORMAT)?,
                17 => self.parse_material_additional_count(2, reader)?,
                18 => self.parse_material_id(3, reader, Self::FORMAT)?,
                19 => self.parse_material_count(3, reader)?,
                20 => self.parse_material_additional_id(3, reader, Self::FORMAT)?,
                21 => self.parse_material_additional_count(3, reader)?,
                22 => self.parse_material_id(4, reader, Self::FORMAT)?,
                23 => self.parse_material_count(4, reader)?,
                24 => self.parse_material_additional_id(4, reader, Self::FORMAT)?,
                25 => self.parse_material_additional_count(4, reader)?,

                30 => self.success_probability = reader.read_f32_le()?,

                42 => self.inheritance_on_craft = reader.read_f32_le()? != 0.0,
                43 => self.inheritance_enhancement_condition = reader.read_f32_le()? as u8,
                44 => self.inheritance_transcendence_condition = reader.read_f32_le()? as u8,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Product {
    fn parse_material_id<R: BufRead + ?Sized>(
        &mut self,
        index: usize,
        reader: &mut R,
        format: DataFormat,
    ) -> Result<()> {
        let id = SharedString::new(reader.read_string(format)?.to_uppercase());
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

    fn parse_material_additional_id<R: BufRead + ?Sized>(
        &mut self,
        index: usize,
        reader: &mut R,
        format: DataFormat,
    ) -> Result<()> {
        let id = SharedString::new(reader.read_string(format)?.to_uppercase());
        if id != "*" {
            self.materials
                .entry(index)
                .and_modify(|f| f.additional_node = Some(ItemNode { id, item: None }));
        }
        Ok(())
    }

    fn parse_material_count<R: BufRead + ?Sized>(&mut self, index: usize, reader: &mut R) -> Result<()> {
        let count = reader.read_f32_le()? as u16;

        self.materials.entry(index).and_modify(|f| f.count = count);
        Ok(())
    }

    fn parse_material_additional_count<R: BufRead + ?Sized>(
        &mut self,
        index: usize,
        reader: &mut R,
    ) -> Result<()> {
        let count = reader.read_f32_le()? as u16;

        self.materials.entry(index).and_modify(|f| f.additional_count = count);
        Ok(())
    }

    pub fn set_materials(&mut self, items: &IndexMap<SharedString, Arc<Mutex<Item>>>, unknown_ids: &mut BTreeMap<SharedString, u32>) {
        self.node.item = items.get(&self.node.id).map(|f| Arc::downgrade(f));
        if self.node.item.is_none() {
            unknown_ids.entry(self.node.id.clone()).and_modify(|count| *count += 1).or_insert(1);
        }

        for (_, m) in self.materials.iter_mut() {
            m.node.item = items.get(&m.node.id).map(|f| Arc::downgrade(f));
            if m.node.item.is_none() {
                unknown_ids.entry(m.node.id.clone()).and_modify(|count| *count += 1).or_insert(1);
            }

            if let Some(node) = m.additional_node.as_mut() {
                node.item = items.get(&node.id).map(|f| Arc::downgrade(f));
                if node.item.is_none() {
                    unknown_ids.entry(node.id.clone()).and_modify(|count| *count += 1).or_insert(1);
                }
            }
        }
    }
}
