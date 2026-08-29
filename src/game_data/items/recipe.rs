use std::{
    cell::RefCell,
    io::SeekFrom,
    rc::{Rc, Weak},
};

use crate::{
    game_data::{AsyncBufReadExtReadString, DataFormat, TagType, common::Common, item::ItemTrait, item::ReadableItem, product::Product},
    language::t,
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::SharedString;

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
    pub debug: Vec<u8>,
    pub product: Option<Weak<RefCell<Product>>>,
    pub common: Common,

    pub recipe_type: Option<RecipeType>,
    pub required_stage: u8,
    pub crafted_item_id: SharedString,
}

impl ReadableItem for Recipe {
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
    pub fn set_product(&mut self, products: &Vec<Rc<RefCell<Product>>>) {
        self.product = products
            .iter()
            .find(|p| p.borrow().node.id == self.common.id || p.borrow().productid == self.crafted_item_id)
            .map(|f| Rc::downgrade(f));

        if let Some(f) = self.product.as_mut().and_then(|f| f.upgrade()) {
            f.borrow_mut().recipe = Some(self.common.id.clone());
        }
    }
}

impl ItemTrait for Recipe {
    fn common(&self) -> &Common {
        &self.common
    }

    fn debug(&self) -> &[u8] {
        &self.debug
    }
}
