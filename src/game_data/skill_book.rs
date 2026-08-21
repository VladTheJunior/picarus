use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::Rc,
    sync::Arc,
};

use crate::{
    game_data::{
        AbstractItem, Binding, DataFormat, GameClass, Grade, Item, ItemEffect, TagType, item_set::ItemSet, locale::Locale, product::Product,
        recipe::RecipeType,
    },
    language::{LanguageController, t},
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Default, Serialize)]
pub struct SkillBook {
    pub linked_recipes: BTreeSet<SharedString>,
    pub locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    pub id: SharedString,
    pub name: SharedString,
    pub grade: Option<Grade>,
    pub required_level: u8,
    pub item_level: u16,
    pub cooldown: f32,
    pub buy_price: f32,
    pub sell_price: f32,
    pub stack_size: f32,
    pub learned_skill: SharedString,
    pub learned_skill_level: f32,
    pub no_trade: bool,
    pub no_sell: bool,
    pub no_destroy: bool,
    pub drop_level_check: SharedString,
    pub binding: Option<Binding>,
    pub usage_restriction: SharedString,
    pub class: SharedString,
    pub usable_class: BTreeSet<GameClass>,
    pub sale_agency_category: SharedString,
    pub contents_level: f32,
    pub unified_channel_disabled: f32,
}

impl AbstractItem for SkillBook {
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
                3 => self.required_level = reader.read_f32_le().await? as u8,
                4 => self.item_level = reader.read_f32_le().await? as u16,
                  5 => self.cooldown = reader.read_f32_le().await?,
   6 => self.buy_price = reader.read_f32_le().await?,
    7 => self.sell_price = reader.read_f32_le().await?,
    8 => self.stack_size = reader.read_f32_le().await?,
    9 => self.learned_skill= Self::read_string(format, reader).await?,
    10 => self.learned_skill_level = reader.read_f32_le().await?,
    11 => self.no_trade = reader.read_f32_le().await? != 0.0,
    12 => self.no_sell = reader.read_f32_le().await? != 0.0,
    13 => self.no_destroy = reader.read_f32_le().await? != 0.0,
    14 => self.drop_level_check= Self::read_string(format, reader).await?,
    15 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
    16=> self.usage_restriction= Self::read_string(format, reader).await?,
   17 => self.class= Self::read_string(format, reader).await?,
    18 => {
                    let value = Self::read_string(format, reader).await?;

                    self.usable_class = value.split("_").filter_map(|c| GameClass::try_from(c).ok()).collect();
                }
    19 => self.sale_agency_category= Self::read_string(format, reader).await?,
    20 => self.contents_level = reader.read_f32_le().await?,
    21=> self.unified_channel_disabled = reader.read_f32_le().await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Item for SkillBook {
    fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>, skill_locales: &HashMap<SharedString, Locale>) {
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

    fn set_product(
        &mut self,
        products_by_recipe_id: &HashMap<SharedString, Rc<RefCell<Product>>>,
        products_by_result_id: &HashMap<SharedString, Rc<RefCell<Product>>>,
    ) {
    }
}
