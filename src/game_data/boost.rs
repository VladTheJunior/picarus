use std::{
    cell::RefCell, collections::{BTreeSet, HashMap}, io::{Read, SeekFrom}, rc::Rc, sync::Arc,
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
pub struct Boost {
    pub linked_recipes: BTreeSet<SharedString>,
    pub locale: Option<Locale>,
    pub description_locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,

    pub number: f32,
    pub id: SharedString,
    pub name: SharedString,
    pub required_level: u8,
    pub limit_level: f32,
    pub item_level: u16,
    pub consume_type: SharedString,
    pub grade: Option<Grade>,
    pub r#type: SharedString, // "type" is a reserved keyword, so use raw identifier
    pub purchase_price: f32,
    pub disposal_price: f32,
    pub equip_effect_1: Option<ItemEffect>,
    pub equip_effect_2: Option<ItemEffect>,
    pub equip_effect_3: Option<ItemEffect>,
    pub equip_effect_4: Option<ItemEffect>,
    pub no_drop: f32,
    pub no_trade: bool,
    pub no_sell: bool,
    pub no_destroy: bool,
    pub drop_level_check: SharedString,
    pub binding: Option<Binding>,
    pub binding_target: SharedString,
    pub binding_release_count: f32,
    pub usage_restriction: SharedString,
    pub consignment_category: SharedString,
    pub skill_effect: SharedString,
    pub stack_count: f32,
    pub create_count: f32,
    pub usage_period: f32,
    pub cash: f32,
    pub contents_level: f32,
    pub no_unified_channel: f32,
    pub purchase_limit: f32,
    pub group: f32,
    pub rank: f32,
    pub true_keeping: f32,
    pub area_type: SharedString,
    pub usage_area: SharedString,
}

impl AbstractItem for Boost {
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
                0 => self.number = reader.read_f32_le().await?,
                1 => self.id = SharedString::new(Self::read_string(format, reader).await?.to_uppercase()),
                2 => self.name = Self::read_string(format, reader).await?,
                3 => self.required_level = reader.read_f32_le().await? as u8,
                4 => self.limit_level = reader.read_f32_le().await?,
                5 => self.item_level = reader.read_f32_le().await? as u16,
                6 => self.consume_type = Self::read_string(format, reader).await?,
                7 => self.grade = Grade::from_repr(reader.read_f32_le().await? as u8),
                8 => self.r#type = Self::read_string(format, reader).await?, // "type" is a reserved keyword, so use raw identifier
                9 => self.purchase_price = reader.read_f32_le().await?,
                10 => self.disposal_price = reader.read_f32_le().await?,
                11 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_1 = Some(ItemEffect::new(effect));
                    }
                }
                12 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_2 = Some(ItemEffect::new(effect));
                    }
                }
                13 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_3 = Some(ItemEffect::new(effect));
                    }
                }
                14 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_4 = Some(ItemEffect::new(effect));
                    }
                }
                15 => self.no_drop = reader.read_f32_le().await?,
                16 => self.no_trade = reader.read_f32_le().await? != 0.0,
                17 => self.no_sell = reader.read_f32_le().await? != 0.0,
                18 => self.no_destroy = reader.read_f32_le().await? != 0.0,
                19 => self.drop_level_check = Self::read_string(format, reader).await?,
                20 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
                21 => self.binding_target = Self::read_string(format, reader).await?,
                22 => self.binding_release_count = reader.read_f32_le().await?,
                23 => self.usage_restriction = Self::read_string(format, reader).await?,
                24 => self.consignment_category = Self::read_string(format, reader).await?,
                25 => self.skill_effect = Self::read_string(format, reader).await?,
                26 => self.stack_count = reader.read_f32_le().await?,
                27 => self.create_count = reader.read_f32_le().await?,
                28 => self.usage_period = reader.read_f32_le().await?,
                29 => self.cash = reader.read_f32_le().await?,
                30 => self.contents_level = reader.read_f32_le().await?,
                31 => self.no_unified_channel = reader.read_f32_le().await?,
                32 => self.purchase_limit = reader.read_f32_le().await?,
                33 => self.group = reader.read_f32_le().await?,
                34 => self.rank = reader.read_f32_le().await?,
                35 => self.true_keeping = reader.read_f32_le().await?,
                36 => self.area_type = Self::read_string(format, reader).await?,
                37 => self.usage_area = Self::read_string(format, reader).await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Item for Boost {
    fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>, skill_locales: &HashMap<SharedString, Locale>) {
        self.locale = locales.get(&self.id).cloned();
        self.description_locale = locales.get(&SharedString::new(format!("{}_DESCRIPTION", self.id))).cloned();
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
        products_by_recipe_id: &HashMap<SharedString,  Rc<RefCell<Product>>>,
        products_by_result_id: &HashMap<SharedString,  Rc<RefCell<Product>>>,
    ) {
    }
}
