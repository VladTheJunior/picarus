use std::{
    collections::BTreeSet, io::{BufRead, Seek, SeekFrom}, sync::{Arc, Weak},
};

use crate::{
    game_data::{
        DataFormat, ReadExt, TagType, common::Common, items::{ItemTrait, ReadableItem}, product::Product,
    }, game_data_view::PreviewBuilder, language::t,
};
use anyhow::Result;
use indexmap::IndexMap;
use parking_lot::Mutex;
use serde::Serialize;


use gpui_kit::SharedString;

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

#[derive(Default, Clone)]
pub struct Recipe {
    pub debug: Vec<u8>,
    pub product: Option<Weak<Mutex<Product>>>,
    pub common: Common,

    pub recipe_type: Option<RecipeType>,
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
                4 => {
                    let m = reader.read_string(Self::FORMAT)?;
                    self.recipe_type = RecipeType::try_from(m.as_str()).ok()
                }
                9 => self.crafted_item_id = SharedString::new(reader.read_string(Self::FORMAT)?.to_uppercase()),

                _ => {}
            }
        }

        Ok(self)
    }
}

impl Recipe {
    pub fn set_product(&mut self, products: &Vec<Arc<Mutex<Product>>>) {
        self.product = products
            .iter()
            .find(|p| {
                let g = p.lock();
                g.node.id == self.common.id || g.productid == self.crafted_item_id
            })
            .map(|f| Arc::downgrade(f));

        if let Some(f) = self.product.as_mut().and_then(|f| f.upgrade()) {
            f.lock().recipe = Some(self.common.id.clone());
        }
    }
}

impl ItemTrait for Recipe {
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
        PreviewBuilder::new(self.common())
            .recipe_stage(self.product.as_ref().and_then(|f| f.upgrade()).map(|p| p.lock().technology_grade))
            .product(self.product.clone())
            .recipe_types(self.recipe_type.as_ref().map(|f| BTreeSet::from([*f])))
    }
}
