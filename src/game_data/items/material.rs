use std::{
    collections::{BTreeSet, HashMap},
    io::SeekFrom,
};

use crate::game_data::{
    DataFormat, TagType, common::Common, item::ItemTrait, item::ReadableItem, item_res::ItemRes, items::recipe::RecipeType, locale::Locale,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

use gpui::SharedString;

#[derive(Default)]
pub struct Material {
    pub debug: Vec<u8>,
    pub description_locale: Option<Locale>,
    pub recipe_type: Option<BTreeSet<RecipeType>>,
    pub common: Common,
}

impl ReadableItem for Material {
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

                10 => self.common.parse_no_trade(reader).await?,
                11 => self.common.parse_no_sell(reader).await?,
                12 => self.common.parse_no_destroy(reader).await?,

                14 => self.common.parse_binding(reader, Self::FORMAT).await?,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Material {
    pub fn set_description_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.description_locale = locales.get(&SharedString::new(format!("{}_DESCRIPTION", self.common.id))).cloned();
    }

    pub fn set_recipe_type(&mut self, res: &HashMap<SharedString, ItemRes>) {
        if let Some(item_res) = res.get(&self.common.id) {
            self.recipe_type = item_res
                .using_recipe_type
                .as_ref()
                .map(|r| r.split("_").filter_map(|r| RecipeType::try_from(r).ok()).collect());
        }
    }
}

impl ItemTrait for Material {
    fn common(&self) -> &Common {
        &self.common
    }

    fn debug(&self) -> &[u8] {
        &self.debug
    }
}
