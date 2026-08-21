use std::{
    cell::RefCell, collections::{BTreeSet, HashMap}, io::{Read, SeekFrom}, rc::Rc, sync::Arc,
};

use crate::{
    game_data::{
        AbstractItem, Binding, DataFormat, GameClass, Grade, Item, ItemEffect, TagType, item_set::ItemSet, locale::Locale, product::Product,
    },
    language::LanguageController,
};
use anyhow::Result;
use gpui::{Image, SharedString};
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tracing::warn;

#[derive(Default, Serialize)]
pub struct FellowEquip {
    pub linked_recipes: BTreeSet<SharedString>,
    pub item_set: Option<ItemSet>,
    pub locale: Option<Locale>,
    pub skill_locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    pub id: SharedString,

    pub name: SharedString,
    pub development_use: f32,
    pub required_level: u8,
    pub restricted_level: f32,
    pub item_level: u16,
    pub usable_class: BTreeSet<GameClass>,
    pub grade: Option<Grade>,
    pub fellow_equip_type: SharedString,
    pub durability: f32,
    pub purchase_price: f32,
    pub disposal_price: f32,
    pub max_ep_plus: Option<f32>,
    pub equip_effect_1: Option<ItemEffect>,
    pub equip_effect_2: Option<ItemEffect>,
    pub equip_effect_3: Option<ItemEffect>,
    pub equip_effect_4: Option<ItemEffect>,
    pub variable_option_1: SharedString,
    pub variable_option_min_1: f32,
    pub variable_option_max_1: f32,
    pub variable_option_2: SharedString,
    pub variable_option_min_2: f32,
    pub variable_option_max_2: f32,
    pub no_drop: f32,
    pub untradeable: bool,
    pub unsellable: bool,
    pub indestructible: bool,
    pub drop_level_check: SharedString,
    pub binding: Option<Binding>,
    pub binding_target: SharedString,
    pub binding_release_count: f32,
    pub usage_restriction: SharedString,
    pub dyeable: f32,
    pub sale_agency_category: SharedString,
    pub usage_period: f32,
    pub contents_level: f32,
    pub unified_channel_disabled: f32,
    pub equip_condition: f32,
}
impl AbstractItem for FellowEquip {
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
        // Read all fields sequentially
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
                2 => self.development_use = reader.read_f32_le().await?,
                3 => self.required_level = reader.read_f32_le().await? as u8,
                4 => self.restricted_level = reader.read_f32_le().await?,
                5 => self.item_level = reader.read_f32_le().await? as u16,
                6 => {
                    let value = Self::read_string(format, reader).await?;

                    self.usable_class = value.split("_").filter_map(|c| GameClass::try_from(c).ok()).collect();
                }
                7 => self.grade = Grade::from_repr(reader.read_f32_le().await? as u8),
                8 => self.fellow_equip_type = Self::read_string(format, reader).await?,
                9 => self.durability = reader.read_f32_le().await?,
                10 => self.purchase_price = reader.read_f32_le().await?,
                11 => self.disposal_price = reader.read_f32_le().await?,
                12 => {
                    let v = reader.read_f32_le().await?;

                    if v != 0.0 {
                        self.max_ep_plus = Some(v);
                    }
                }
                13 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_1 = Some(ItemEffect::new(effect));
                    }
                }
                14 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_2 = Some(ItemEffect::new(effect));
                    }
                }
                15 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_3 = Some(ItemEffect::new(effect));
                    }
                }
                16 => {
                    let effect = Self::read_string(format, reader).await?;
                    if effect != "*" && effect != "0" {
                        self.equip_effect_4 = Some(ItemEffect::new(effect));
                    }
                }
                17 => self.variable_option_1 = Self::read_string(format, reader).await?,
                18 => self.variable_option_min_1 = reader.read_f32_le().await?,
                19 => self.variable_option_max_1 = reader.read_f32_le().await?,
                20 => self.variable_option_2 = Self::read_string(format, reader).await?,
                21 => self.variable_option_min_2 = reader.read_f32_le().await?,
                22 => self.variable_option_max_2 = reader.read_f32_le().await?,
                23 => self.no_drop = reader.read_f32_le().await?,
                24 => self.untradeable = reader.read_f32_le().await? != 0.0,
                25 => self.unsellable = reader.read_f32_le().await? != 0.0,
                26 => self.indestructible = reader.read_f32_le().await? != 0.0,
                27 => self.drop_level_check = Self::read_string(format, reader).await?,
                28 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
                29 => self.binding_target = Self::read_string(format, reader).await?,
                30 => self.binding_release_count = reader.read_f32_le().await?,
                31 => self.usage_restriction = Self::read_string(format, reader).await?,
                32 => self.dyeable = reader.read_f32_le().await?,
                33 => self.sale_agency_category = Self::read_string(format, reader).await?,
                34 => self.usage_period = reader.read_f32_le().await?,
                35 => self.contents_level = reader.read_f32_le().await?,
                36 => self.unified_channel_disabled = reader.read_f32_le().await?,
                37 => self.equip_condition = reader.read_f32_le().await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Item for FellowEquip {
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

    fn set_item_set(&mut self, item_set: &Vec<super::item_set::ItemSet>) {
        self.item_set = item_set.iter().find(|f| f.items.contains(&self.id)).cloned();
    }

    fn set_product(
        &mut self,
        products_by_recipe_id: &HashMap<SharedString,  Rc<RefCell<Product>>>,
        products_by_result_id: &HashMap<SharedString,  Rc<RefCell<Product>>>,
    ) {
    }
}
