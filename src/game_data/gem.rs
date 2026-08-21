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
pub struct Gem {
    pub linked_recipes: BTreeSet<SharedString>,
    pub locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    pub id: SharedString,
    pub name: SharedString,
    pub grade: Option<Grade>,
    pub item_level: u16,
    pub required_level: u8,
    pub attribute_type: SharedString,
    pub enhancement_type: SharedString,
    pub creature_grade: SharedString,
    pub soul_stone_color: SharedString,
    pub equip_effect_1: Option<ItemEffect>,
    pub equip_effect_2: Option<ItemEffect>,
    pub equip_effect_3: Option<ItemEffect>,
    pub equip_effect_4: Option<ItemEffect>,
    pub skill_effect: SharedString,
    pub soul_stone_equip_chance: f32,
    pub soul_stone_remove_cost: f32,
    pub purchase_price: f32,
    pub disposal_price: f32,
    pub stack_count: f32,
    pub no_trade: bool,
    pub no_sell: bool,
    pub no_destroy: bool,
    pub drop_level_check: SharedString,
    pub binding: Option<Binding>,
    pub usage_restriction: SharedString,
    pub sales_agency_category: SharedString,
    pub gem_extraction_cost: f32,
    pub drop_level_check_ignore: f32,
    pub currency_settings_id: SharedString,
    pub contents_level: f32,
    pub unified_channel_disabled: f32,
}

impl AbstractItem for Gem {
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
                3 => self.item_level = reader.read_f32_le().await? as u16,
                4 => self.required_level = reader.read_f32_le().await? as u8,
                5 => self.attribute_type = Self::read_string(format, reader).await?,
                6 => self.enhancement_type = Self::read_string(format, reader).await?,
                7 => self.creature_grade = Self::read_string(format, reader).await?,
                8 => self.soul_stone_color = Self::read_string(format, reader).await?,
                9 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_1 = Some(ItemEffect::new(effect));
                    }
                }
                10 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_2 = Some(ItemEffect::new(effect));
                    }
                }
                11 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_3 = Some(ItemEffect::new(effect));
                    }
                }
                12 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_4 = Some(ItemEffect::new(effect));
                    }
                }
                13 => self.skill_effect = Self::read_string(format, reader).await?,
                14 => self.soul_stone_equip_chance = reader.read_f32_le().await?,
                15 => self.soul_stone_remove_cost = reader.read_f32_le().await?,
                16 => self.purchase_price = reader.read_f32_le().await?,
                17 => self.disposal_price = reader.read_f32_le().await?,
                18 => self.stack_count = reader.read_f32_le().await?,
                19 => self.no_trade = reader.read_f32_le().await? != 0.0,
                20 => self.no_sell = reader.read_f32_le().await? != 0.0,
                21 => self.no_destroy = reader.read_f32_le().await? != 0.0,
                22 => self.drop_level_check = Self::read_string(format, reader).await?,
                23 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
                24 => self.usage_restriction = Self::read_string(format, reader).await?,
                25 => self.sales_agency_category = Self::read_string(format, reader).await?,
                26 => self.gem_extraction_cost = reader.read_f32_le().await?,
                27 => self.drop_level_check_ignore = reader.read_f32_le().await?,
                28 => self.currency_settings_id = Self::read_string(format, reader).await?,
                29 => self.contents_level = reader.read_f32_le().await?,
                30 => self.unified_channel_disabled = reader.read_f32_le().await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Item for Gem {
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
