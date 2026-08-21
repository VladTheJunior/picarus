use std::{
    cell::RefCell, collections::{BTreeSet, HashMap}, io::{Read, SeekFrom}, rc::{Rc, Weak}, sync::Arc,
};

use crate::{
    game_data::{AbstractItem, Binding, DataFormat, Grade, Item, TagType, item_set::ItemSet, locale::Locale, product::Product}, language::t,
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

#[derive(Default, Serialize)]
pub struct Recipe {
    pub locale: Option<Locale>,
    pub product: Option<Weak<RefCell<Product>>>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    pub id: SharedString,

    pub name: SharedString,
    pub grade: Option<Grade>,
    pub recipe_category: SharedString,
    pub recipe_type: Option<RecipeType>,
    pub item_level: f32,
    pub required_stage: u8,
    pub required_proficiency: f32,
    pub required_level: u8,
    pub crafted_item_id: SharedString,
    pub crafted_item_name: SharedString,
    pub cooldown: f32,
    pub buy_price: f32,
    pub sell_price: f32,
    pub stack_size: f32,
    pub learned_skill: SharedString,
    pub skill_level_required: f32,
    pub untradeable: bool,
    pub unsellable: bool,
    pub indestructible: bool,
    pub drop_level_check: SharedString,
    pub binding: Option<Binding>,
    pub usage_restriction: SharedString,
    pub market_category: SharedString,
    pub ignore_drop_level_check: f32,
    pub contents_level: f32,
    pub disable_unified_channel: f32,
    pub inheritance_enhance_condition: f32,
    pub inheritance_transcend_condition: f32,
    pub currency_settings_id: SharedString,
}

impl AbstractItem for Recipe {
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
                0 => self.id = SharedString::new(Self::read_string(format, reader).await?.to_uppercase()),
                1 => self.name = Self::read_string(format, reader).await?,
                2 => self.grade = Grade::from_repr(reader.read_f32_le().await? as u8),
                3 => self.recipe_category = Self::read_string(format, reader).await?,
                4 => {
                    let m = Self::read_string(format, reader).await?;
                    self.recipe_type = RecipeType::try_from(m.as_str()).ok()
                }
                5 => self.item_level = reader.read_f32_le().await?,
                6 => self.required_stage = reader.read_f32_le().await? as u8,
                7 => self.required_proficiency = reader.read_f32_le().await?,
                8 => self.required_level = reader.read_f32_le().await? as u8,
                9 => self.crafted_item_id = SharedString::new(Self::read_string(format, reader).await?.to_uppercase()),
                10 => self.crafted_item_name = Self::read_string(format, reader).await?,
                11 => self.cooldown = reader.read_f32_le().await?,
                12 => self.buy_price = reader.read_f32_le().await?,
                13 => self.sell_price = reader.read_f32_le().await?,
                14 => self.stack_size = reader.read_f32_le().await?,
                15 => self.learned_skill = Self::read_string(format, reader).await?,
                16 => self.skill_level_required = reader.read_f32_le().await?,
                17 => self.untradeable = reader.read_f32_le().await? != 0.0,
                18 => self.unsellable = reader.read_f32_le().await? != 0.0,
                19 => self.indestructible = reader.read_f32_le().await? != 0.0,
                20 => self.drop_level_check = Self::read_string(format, reader).await?,
                21 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
                22 => self.usage_restriction = Self::read_string(format, reader).await?,
                23 => self.market_category = Self::read_string(format, reader).await?,
                24 => self.ignore_drop_level_check = reader.read_f32_le().await?,
                25 => self.contents_level = reader.read_f32_le().await?,
                26 => self.disable_unified_channel = reader.read_f32_le().await?,
                27 => self.inheritance_enhance_condition = reader.read_f32_le().await?,
                28 => self.inheritance_transcend_condition = reader.read_f32_le().await?,
                29 => self.currency_settings_id = Self::read_string(format, reader).await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Item for Recipe {
    fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>, _skill_locales: &HashMap<SharedString, Locale>) {
        self.locale = locales.get(&self.id).cloned();
    }

    async fn set_icon<R: std::io::Read + std::io::Seek>(
        &mut self,
        res: &HashMap<SharedString, super::item_res::ItemRes>,
        zip: &mut zip::ZipArchive<R>,
    ) -> Result<()> {
        if let Some(item_res) = res.get(&self.id) {
            if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon.to_lowercase())) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match Self::dds_to_jpeg(buf).await {
                    Ok(icon) => self.icon = Some(icon),
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            } else if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon)) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match Self::dds_to_jpeg(buf).await {
                    Ok(icon) => self.icon = Some(icon),
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            }
        }
        Ok(())
    }

    fn get_full_type(&self) -> SharedString {
        unimplemented!()
    }

    fn get_type(&self) -> SharedString {
        unimplemented!()
    }

    fn set_item_set(&mut self, _item_set: &Vec<ItemSet>) {}
    
    fn set_product(&mut self, products_by_recipe_id: &HashMap<SharedString, Rc<RefCell<Product>>>, products_by_result_id: &HashMap<SharedString, Rc<RefCell<Product>>>) {
        self.product = products_by_recipe_id.get(&self.id).or_else(|| products_by_result_id.get(&self.crafted_item_id)).map(|f| Rc::downgrade(f));
    }
}
