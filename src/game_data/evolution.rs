use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    io::SeekFrom,
    rc::Rc,
};

use crate::game_data::{
    AsyncBufReadExtReadString, DataFormat, TagType,
    grade::Grade,
    items::{Item, ItemNode, ReadableItem},
    locale::Locale,
};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Default, Clone)]
pub struct Evolution {
    pub evolvegroup: f32,
    pub cost_grade: f32,
    pub material_fellow: ItemNode,
    pub material_pet: SharedString,
    pub resultfellow: SharedString,
    pub fellowicon: SharedString,
    pub evolvetabcategoryid: SharedString,
    pub successrate: f32,
    pub extinctionrate: [f32; 5],
    pub reinforceplusrate: [f32; 8],
    pub material_count: u16,
}

impl ReadableItem for Evolution {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        unimplemented!()
    }

    type CollectionItem = Self;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        item
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
        unimplemented!()
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

        for (tag_idx, (tag, _)) in definitions.iter().enumerate() {
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

            match tag.as_str() {
                "evolvegroup" => self.evolvegroup = reader.read_f32_le().await?,
                "cost_grade" => self.cost_grade = reader.read_f32_le().await?,
                "material_fellow" => self.material_fellow.id = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),
                "material_pet" => self.material_pet = reader.read_string(Self::FORMAT).await?,
                "resultfellow" => self.resultfellow = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),
                "fellowicon" => self.fellowicon = reader.read_string(Self::FORMAT).await?,
                "evolvetabcategoryid" => self.evolvetabcategoryid = reader.read_string(Self::FORMAT).await?,
                "successrate" => self.successrate = reader.read_f32_le().await? / 100.0,

                "extinctionrate00" => self.extinctionrate[0] = reader.read_f32_le().await?,
                "extinctionrate01" => self.extinctionrate[1] = reader.read_f32_le().await?,
                "extinctionrate02" => self.extinctionrate[2] = reader.read_f32_le().await?,
                "extinctionrate03" => self.extinctionrate[3] = reader.read_f32_le().await?,
                "extinctionrate04" => self.extinctionrate[4] = reader.read_f32_le().await?,

                "reinforceplusrate00" => self.reinforceplusrate[0] = reader.read_f32_le().await?,
                "reinforceplusrate01" => self.reinforceplusrate[1] = reader.read_f32_le().await?,
                "reinforceplusrate02" => self.reinforceplusrate[2] = reader.read_f32_le().await?,
                "reinforceplusrate03" => self.reinforceplusrate[3] = reader.read_f32_le().await?,
                "reinforceplusrate04" => self.reinforceplusrate[4] = reader.read_f32_le().await?,
                "reinforceplusrate05" => self.reinforceplusrate[5] = reader.read_f32_le().await?,
                "reinforceplusrate06" => self.reinforceplusrate[6] = reader.read_f32_le().await?,
                "reinforceplusrate07" => self.reinforceplusrate[7] = reader.read_f32_le().await?,

                "material_count" => self.material_count = reader.read_f32_le().await? as u16,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Evolution {
    pub fn set_fellows(&mut self, items: &IndexMap<SharedString, Rc<RefCell<Item>>>, unknown_ids: &mut BTreeMap<SharedString, u32>) {
        self.material_fellow.item = items.get(&self.material_fellow.id).map(|f| Rc::downgrade(f));
        if self.material_fellow.item.is_none() {
            unknown_ids
                .entry(self.material_fellow.id.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
    }
}
