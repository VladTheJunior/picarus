use std::{
    collections::{BTreeSet, HashMap}, io::{Read, SeekFrom}, sync::Arc,
};

use crate::{
    game_data::{AbstractItem, Binding, DataFormat, Grade, Item, item_set::ItemSet, locale::Locale},
    language::t,
};
use anyhow::Result;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;
#[derive(Debug, Serialize)]
#[derive(Ord)]
#[derive(PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum MaterialType {
    Alchemy,
    Cooking,
    WeaponCrafting,
    ArmorCrafting,
    JewelryCrafting,
    BarderCrafting,
}

impl TryFrom<&str> for MaterialType {
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

impl MaterialType {
    pub fn locale(&self) -> SharedString {
        match self {
            MaterialType::Alchemy => t("item-material-type-alchemy"),
            MaterialType::Cooking => t("item-material-type-cooking"),
            MaterialType::WeaponCrafting => t("item-material-type-weapon"),
            MaterialType::ArmorCrafting => t("item-material-type-armor"),
            MaterialType::JewelryCrafting => t("item-material-type-jewelry"),
            MaterialType::BarderCrafting => t("item-material-type-barder"),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Material {
    pub locale: Option<Locale>,
    pub description_locale: Option<Locale>,
    pub recipe_type: Option<BTreeSet<MaterialType>>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    pub id: SharedString,
    pub name: SharedString,
    pub material_type: SharedString,
    pub grade: Option<Grade>,
    pub required_level: u8,
    pub item_level: u16,
    pub purchase_price: f32,
    pub disposal_price: f32,
    pub creation_count: f32,
    pub stack_size: u16,
    pub non_tradeable: bool,
    pub non_disposable: bool,
    pub non_destroyable: bool,
    pub drop_level_check: SharedString,
    pub binding: Option<Binding>,
    pub usage_restriction: SharedString,
    pub sales_agency_category: SharedString,
    pub drop_level_check_ignore: f32,
    pub currency_settings_id: SharedString,
    pub purchase_limit: f32,
    pub contents_level: f32,
    pub integrated_channel_unavailable: f32,
    pub plus_rate: f32,
}

impl AbstractItem for Material {
    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        tag_count: usize,
        global_offset: u64,
        format: DataFormat,
    ) -> Result<Self> {
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
                2 => self.material_type = Self::read_string(format, reader).await?,
                3 => self.grade = Grade::from_repr(reader.read_f32_le().await? as u8),
                4 => self.required_level = reader.read_f32_le().await? as u8,
                5 => self.item_level = reader.read_f32_le().await? as u16,
                6 => self.purchase_price = reader.read_f32_le().await?,
                7 => self.disposal_price = reader.read_f32_le().await?,

                8 => self.creation_count = reader.read_f32_le().await?,
                9 => self.stack_size = reader.read_f32_le().await? as u16,
                10 => self.non_tradeable = reader.read_f32_le().await? != 0.0,
                11 => self.non_disposable = reader.read_f32_le().await? != 0.0,
                12 => self.non_destroyable = reader.read_f32_le().await? != 0.0,
                13 => self.drop_level_check = Self::read_string(format, reader).await?,

                14 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
                15 => self.usage_restriction = Self::read_string(format, reader).await?,
                16 => self.sales_agency_category = Self::read_string(format, reader).await?,
                17 => self.drop_level_check_ignore = reader.read_f32_le().await?,
                18 => self.currency_settings_id = Self::read_string(format, reader).await?,

                19 => self.purchase_limit = reader.read_f32_le().await?,
                20 => self.contents_level = reader.read_f32_le().await?,
                21 => self.integrated_channel_unavailable = reader.read_f32_le().await?,
                22 => self.plus_rate = reader.read_f32_le().await?,
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Item for Material {
    fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>, _skill_locales: &HashMap<SharedString, Locale>) {
        self.locale = locales.get(&self.id).cloned();
        self.description_locale = locales.get(&SharedString::new(format!("{}_DESCRIPTION", self.id))).cloned();
    }

    async fn set_icon<R: std::io::Read + std::io::Seek>(
        &mut self,
        res: &HashMap<SharedString, super::item_res::ItemRes>,
        zip: &mut zip::ZipArchive<R>,
    ) -> Result<()> {
        if let Some(item_res) = res.get(&self.id) {
            self.recipe_type = item_res.using_recipe_type.as_ref().map(|r| r.split("_").filter_map(|r| MaterialType::try_from(r).ok()).collect());

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
}
