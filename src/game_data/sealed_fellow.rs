use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::Rc,
    sync::Arc,
};

use crate::{
    game_data::{
        AbstractItem, Binding, DataFormat, GameClass, Grade, Item, ItemEffect, ItemMinMaxEffect, ItemMinMaxNoStepEffect, ItemMinMaxStepEffect, TagType, item_set::ItemSet, locale::Locale, product::Product, recipe::RecipeType,
    }, language::{LanguageController, t},
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Default, Serialize)]
pub struct SealedFellow {
    pub linked_recipes: BTreeSet<SharedString>,
    pub locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    pub id: SharedString,
    pub name: SharedString,
    pub grade: Option<Grade>,
    pub grade_plus: f32,
    pub seal_type: SharedString,
    pub item_level: u16,
    pub attribute_type: SharedString,
    pub enhancement_type: SharedString,
    pub creature_grade: SharedString,
    pub sealed_fellow_effect_1: Option<ItemMinMaxStepEffect>,
    pub sealed_fellow_effect_2: Option<ItemMinMaxStepEffect>,
    pub sealed_fellow_effect_3: Option<ItemMinMaxStepEffect>,
    pub max_enhancement_sealed_fellow_effect: Option<ItemMinMaxNoStepEffect>,
    pub rf_grade: u8,
    pub rf_effect: f32,
    pub skill_effect: SharedString,
    pub purchase_price: f32,
    pub disposal_price: f32,
    pub no_trade: bool,
    pub no_sell: bool,
    pub no_destroy: bool,
    pub drop_level_check: SharedString,
    pub binding: Option<Binding>,
    pub usage_restriction: SharedString,
    pub characteristic_power: u16,
    pub usage_period: f32,
    pub consignment_category: SharedString,
    pub integrated_channel_disabled: f32,
    pub stack_count: f32,
    pub exp_weight: f32,
}

impl AbstractItem for SealedFellow {
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
                3 => self.grade_plus = reader.read_f32_le().await?,
                4 => self.seal_type = Self::read_string(format, reader).await?,
                5 => self.item_level = reader.read_f32_le().await? as u16,
                6 => self.attribute_type = Self::read_string(format, reader).await?,
                7 => self.enhancement_type = Self::read_string(format, reader).await?,
                8 => self.creature_grade = Self::read_string(format, reader).await?,
                9 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" && !effect.starts_with("*,") && !effect.starts_with("0,") {
                        self.sealed_fellow_effect_1 = Some(ItemMinMaxStepEffect::new(&effect));
                    }
                }
                10 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" && !effect.starts_with("*,") && !effect.starts_with("0,") {
                        self.sealed_fellow_effect_2 = Some(ItemMinMaxStepEffect::new(&effect));
                    }
                }
                11 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" && !effect.starts_with("*,") && !effect.starts_with("0,") {
                        self.sealed_fellow_effect_3 = Some(ItemMinMaxStepEffect::new(&effect));
                    }
                }
                12 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" && !effect.starts_with("*,") && !effect.starts_with("0,") {
                        self.max_enhancement_sealed_fellow_effect = Some(ItemMinMaxNoStepEffect::new(&effect));
                    }
                }
                13 => self.rf_grade = reader.read_f32_le().await? as u8,
                14 => self.rf_effect = reader.read_f32_le().await?,
                15 => self.skill_effect = Self::read_string(format, reader).await?,
                16 => self.purchase_price = reader.read_f32_le().await?,
                17 => self.disposal_price = reader.read_f32_le().await?,
                18 => self.no_trade = reader.read_f32_le().await? != 0.0,
                19 => self.no_sell = reader.read_f32_le().await? != 0.0,
                20 => self.no_destroy = reader.read_f32_le().await? != 0.0,
                21 => self.drop_level_check = Self::read_string(format, reader).await?,
                22 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
                23 => self.usage_restriction = Self::read_string(format, reader).await?,
                24 => self.characteristic_power = reader.read_f32_le().await? as u16,
                25 => self.usage_period = reader.read_f32_le().await?,
                26 => self.consignment_category = Self::read_string(format, reader).await?,
                27 => self.integrated_channel_disabled = reader.read_f32_le().await?,
                28 => self.stack_count = reader.read_f32_le().await?,
                29 => self.exp_weight = reader.read_f32_le().await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Item for SealedFellow {
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
