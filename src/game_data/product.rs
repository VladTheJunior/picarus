use std::{
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::{Rc, Weak},
    sync::Arc,
};

use crate::{
    game_data::{AbstractItem, Binding, DataFormat, DataType, Grade, Item, TagType, item_set::ItemSet, locale::Locale, recipe::RecipeType},
    language::t,
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Default, Serialize, Clone)]
pub struct DataTypeNode {
    pub id: SharedString,
    #[serde(skip)]
    pub data_type: Option<Weak<DataType>>,
}

#[derive(Default, Serialize, Clone)]
pub struct Product {
    pub node: DataTypeNode,
    pub productid: SharedString,
    pub name: SharedString,
    pub technology_type: SharedString,
    pub technology_grade: u8,
    pub multiproduct: bool,
    pub material1: Option<DataTypeNode>,
    pub count1: u16,
    pub material1_1: Option<DataTypeNode>,
    pub count1_1: u16,
    pub material2: Option<DataTypeNode>,
    pub count2: u16,
    pub material2_1: Option<DataTypeNode>,
    pub count2_1: u16,
    pub material3: Option<DataTypeNode>,
    pub count3: u16,
    pub material3_1: Option<DataTypeNode>,
    pub count3_1: u16,
    pub material4: Option<DataTypeNode>,
    pub count4: u16,
    pub material4_1: Option<DataTypeNode>,
    pub count4_1: u16,
    pub material5: Option<DataTypeNode>,
    pub count5: u16,
    pub material5_1: Option<DataTypeNode>,
    pub count5_1: u16,
    pub item_proficiency1: f32,
    pub item_proficiency2: f32,
    pub item_proficiency3: f32,
    pub item_proficiency4: f32,
    pub success_probability: f32,
    pub base: f32,
    pub crit_probability_1: f32,
    pub crit_probability_2: f32,
    pub additional_crit_ratio: f32,
    pub penalty_probability1: f32,
    pub penalty_probability2: f32,
    pub penalty_probability3: f32,
    pub penalty_probability4: f32,
    pub penalty_probability5: f32,
    pub double_application: f32,
    pub great_success_application: f32,
    pub inheritance_on_craft: bool,
    pub inheritance_enhancement_condition: u8,
    pub inheritance_transcendence_condition: u8,
    pub seal_slot_inheritance: f32,
    pub f47: f32,
}

impl AbstractItem for Product {
    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<String, TagType>,
        global_offset: u64,
        format: DataFormat,
    ) -> Result<Self> {
        let tag_count = definitions.len();
        for tag_idx in 0..tag_count {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match format {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset)).await?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2)).await?;
                }
            };

            match tag_idx {
                0 => {
                    self.node = DataTypeNode {
                        id: SharedString::new(Self::read_string(format, reader).await?.to_uppercase()),
                        data_type: None,
                    }
                }
                1 => self.productid = SharedString::new(Self::read_string(format, reader).await?.to_uppercase()),
                2 => self.name = Self::read_string(format, reader).await?,
                3 => self.technology_type = Self::read_string(format, reader).await?,
                4 => self.technology_grade = reader.read_f32_le().await? as u8,
                5 => self.multiproduct = reader.read_f32_le().await? != 0.0,
                6 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material1 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                7 => self.count1 = reader.read_f32_le().await? as u16,
                8 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material1_1 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                9 => self.count1_1 = reader.read_f32_le().await? as u16,
                10 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material2 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                11 => self.count2 = reader.read_f32_le().await? as u16,
                12 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material2_1 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                13 => self.count2_1 = reader.read_f32_le().await? as u16,
                14 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material3 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                15 => self.count3 = reader.read_f32_le().await? as u16,
                16 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material3_1 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                17 => self.count3_1 = reader.read_f32_le().await? as u16,
                18 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material4 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                19 => self.count4 = reader.read_f32_le().await? as u16,
                20 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material4_1 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                21 => self.count4_1 = reader.read_f32_le().await? as u16,
                22 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material5 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                23 => self.count5 = reader.read_f32_le().await? as u16,
                24 => {
                    let m = SharedString::new(Self::read_string(format, reader).await?.to_uppercase());
                    if m != "*" {
                        self.material5_1 = Some(DataTypeNode { id: m, data_type: None });
                    }
                }
                25 => self.count5_1 = reader.read_f32_le().await? as u16,
                26 => self.item_proficiency1 = reader.read_f32_le().await?,
                27 => self.item_proficiency2 = reader.read_f32_le().await?,
                28 => self.item_proficiency3 = reader.read_f32_le().await?,
                29 => self.item_proficiency4 = reader.read_f32_le().await?,
                30 => self.success_probability = reader.read_f32_le().await?,
                31 => self.base = reader.read_f32_le().await?,
                32 => self.crit_probability_1 = reader.read_f32_le().await?,
                33 => self.crit_probability_2 = reader.read_f32_le().await?,
                34 => self.additional_crit_ratio = reader.read_f32_le().await?,
                35 => self.penalty_probability1 = reader.read_f32_le().await?,
                36 => self.penalty_probability2 = reader.read_f32_le().await?,
                37 => self.penalty_probability3 = reader.read_f32_le().await?,
                38 => self.penalty_probability4 = reader.read_f32_le().await?,
                39 => self.penalty_probability5 = reader.read_f32_le().await?,
                40 => self.double_application = reader.read_f32_le().await?,
                41 => self.great_success_application = reader.read_f32_le().await?,
                42 => self.inheritance_on_craft = reader.read_f32_le().await? != 0.0,
                43 => self.inheritance_enhancement_condition = reader.read_f32_le().await? as u8,
                44 => self.inheritance_transcendence_condition = reader.read_f32_le().await? as u8,
                45 => self.seal_slot_inheritance = reader.read_f32_le().await?,
                46 => self.f47 = reader.read_f32_le().await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Product {
    pub fn validate(&mut self, items: &IndexMap<SharedString, Rc<DataType>>) {
        self.node.data_type = items.get(&self.node.id).map(|f| Rc::downgrade(f));
        if self.node.data_type.is_none() {
            warn!(id = ?self.node.id, "Failed to detect product result");
        }
        if let Some(node) = self.material1.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }
        if let Some(node) = self.material1_1.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }
        if let Some(node) = self.material2.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }
        if let Some(node) = self.material2_1.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }
        if let Some(node) = self.material3.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }
        if let Some(node) = self.material3_1.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }
        if let Some(node) = self.material4.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }
        if let Some(node) = self.material4_1.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }
        if let Some(node) = self.material5.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }

        if let Some(node) = self.material5_1.as_mut() {
            node.data_type = items.get(&node.id).map(|f| Rc::downgrade(f));
            if node.data_type.is_none() {
                warn!(?node.id, "Failed to detect product material");
            }
        }
    }
}
