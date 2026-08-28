use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, SeekFrom},
    rc::{Rc, Weak},
    sync::Arc,
};

use crate::{
    game_data::{AsyncBufReadExtReadString, Binding, Common, DataFormat, Grade, Item, ItemTrait, ReadableItem, TagType, item_set::ItemSet, locale::Locale, product::Product}, language::t,
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Serialize, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum RecipeType {
    Alchemy,
    Cooking,
    WeaponCrafting,
    ArmorCrafting,
    JewelryCrafting,
    BarderCrafting,
}

impl TryFrom<&str> for RecipeType {
    type Error = String;

    fn try_from(other: &str) -> Result<Self, Self::Error> {
        match other {
            "al" => Ok(Self::Alchemy),
            "co" => Ok(Self::Cooking),
            "we" => Ok(Self::WeaponCrafting),
            "ar" => Ok(Self::ArmorCrafting),
            "as" => Ok(Self::JewelryCrafting),
            "fe" => Ok(Self::BarderCrafting),
            unk => Err(format!("Cannot convert {} material type", unk)),
        }
    }
}

impl RecipeType {
    pub fn locale(&self) -> SharedString {
        match self {
            RecipeType::Alchemy => t("item-material-type-alchemy"),
            RecipeType::Cooking => t("item-material-type-cooking"),
            RecipeType::WeaponCrafting => t("item-material-type-weapon"),
            RecipeType::ArmorCrafting => t("item-material-type-armor"),
            RecipeType::JewelryCrafting => t("item-material-type-jewelry"),
            RecipeType::BarderCrafting => t("item-material-type-barder"),
        }
    }
}

#[derive(Default)]
pub struct Recipe {
    pub product: Option<Weak<RefCell<Product>>>,
    pub common: Common,

    pub recipe_type: Option<RecipeType>,
    pub required_stage: u8,
    pub crafted_item_id: SharedString,
}

impl ReadableItem for Recipe {
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

                2 => self.common.parse_grade(reader).await?,

                4 => {
                    let m = reader.read_string(Self::FORMAT).await?;
                    self.recipe_type = RecipeType::try_from(m.as_str()).ok()
                }
                5 => self.common.parse_item_level(reader).await?,
                6 => self.required_stage = reader.read_f32_le().await? as u8,

                8 => self.common.parse_required_level(reader).await?,
                9 => self.crafted_item_id = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),

                17 => self.common.parse_no_trade(reader).await?,
                18 => self.common.parse_no_sell(reader).await?,
                19 => self.common.parse_no_destroy(reader).await?,

                21 => self.common.parse_binding(reader, Self::FORMAT).await?,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Recipe {
    pub fn set_product(
        &mut self,
        products_by_recipe_id: &HashMap<SharedString, Rc<RefCell<Product>>>,
        products_by_result_id: &HashMap<SharedString, Rc<RefCell<Product>>>,
    ) {
        self.product = products_by_recipe_id
            .get(&self.common.id)
            .or_else(|| products_by_result_id.get(&self.crafted_item_id))
            .map(|f| Rc::downgrade(f));
    }
}


impl ItemTrait for Recipe {
   fn common(&self) ->  &Common {
       &self.common
   }
}