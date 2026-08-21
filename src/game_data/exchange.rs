use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    io::{Read, SeekFrom},
    rc::Rc,
    sync::Arc,
};

use crate::{
    game_data::{
        AbstractItem, Binding, DataFormat, GameClass, Grade, Item, TagType, item_set::ItemSet, locale::Locale, product::Product, recipe::RecipeType,
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
pub struct Exchange {
    pub linked_recipes: BTreeSet<SharedString>,
    pub locale: Option<Locale>,
    pub description_locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,

    pub id: SharedString,
    pub name: SharedString,
    pub grade: Option<Grade>,
    pub purchase_price: f32,
    pub disposal_price: f32,
    pub stack_count: f32,
    pub untradeable: bool,
    pub unsellable: bool,
    pub indestructible: bool,
    pub drop_level_check: SharedString,
    pub binding: Option<Binding>,
    pub drop_level_check_ignored: f32,
    pub usage_period: f32,
    pub consignment_category: SharedString,
    pub purchase_limit: f32,
    pub unified_channel_disabled: f32,
    pub f17: SharedString,
}

impl AbstractItem for Exchange {
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
                3 => self.purchase_price = reader.read_f32_le().await?,
                4 => self.disposal_price = reader.read_f32_le().await?,
                5 => self.stack_count = reader.read_f32_le().await?,
                6 => self.untradeable = reader.read_f32_le().await? != 0.0,
                7 => self.unsellable = reader.read_f32_le().await? != 0.0,
                8 => self.indestructible = reader.read_f32_le().await? != 0.0,
                9 => self.drop_level_check = Self::read_string(format, reader).await?,
                10 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
                11 => self.drop_level_check_ignored = reader.read_f32_le().await?,
                12 => self.usage_period = reader.read_f32_le().await?,
                13 => self.consignment_category = Self::read_string(format, reader).await?,
                14 => self.purchase_limit = reader.read_f32_le().await?,
                15 => self.unified_channel_disabled = reader.read_f32_le().await?,
                16 => self.f17 = Self::read_string(format, reader).await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Item for Exchange {
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
        products_by_recipe_id: &HashMap<SharedString, Rc<RefCell<Product>>>,
        products_by_result_id: &HashMap<SharedString, Rc<RefCell<Product>>>,
    ) {
    }
}
