use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap, HashSet},
    io::Read,
    rc::Rc,
    sync::Arc,
};

use crate::game_data::{
    AsyncBufReadExtReadString, DataFormat, binding::Binding, dds_to_jpeg, effects::ItemEffect, game_class::GameClass, grade::Grade,
    item_res::ItemRes, item_set::ItemSet, locale::Locale, product::Product,
};
use anyhow::Result;
use gpui::{Image, SharedString};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek};
use tracing::warn;

#[derive(Default)]
pub struct Common {
    pub linked_recipes: BTreeSet<SharedString>,
    pub item_set: Option<ItemSet>,
    pub locale: Option<Locale>,
    pub skill_locale: Option<Locale>,
    pub icon: Option<Arc<Image>>,

    pub id: SharedString,
    pub grade: Grade,
    pub no_trade: bool,
    pub no_sell: bool,
    pub no_destroy: bool,
    pub binding: Option<Binding>,
    pub required_level: u8,
    pub item_level: u16,
    pub usable_class: BTreeSet<GameClass>,

    pub effects: Vec<ItemEffect>,
}

impl Common {
    pub fn get_unique_effects(&self) -> HashSet<SharedString> {
        let mut effects = HashSet::new();
        effects.extend(self.effects.iter().filter_map(|f| f.parsed.as_ref().map(|(key, _)| key.clone())));

        if let Some(set) = &self.item_set {
            effects.extend(
                set.effects
                    .iter()
                    .flat_map(|f| f.seteffect_effects.iter())
                    .filter_map(|f| f.parsed.as_ref().map(|(key, _)| key.clone())),
            );
        }

        effects
    }

    pub fn get_localized_name(&self) -> SharedString {
        self.locale.as_ref().and_then(|f| f.locale()).unwrap_or_else(|| self.id.clone())
    }

    pub fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.locale = locales.get(&self.id).cloned();
    }

    pub fn set_item_set(&mut self, item_set: &HashMap<SharedString, ItemSet>) {
        self.item_set = item_set.get(&self.id).cloned();
    }

    pub fn set_linked_recipes(&mut self, products: &Vec<Rc<RefCell<Product>>>) {
        self.linked_recipes = products
            .iter()
            .filter_map(|product| {
                let p = product.borrow();
                if p.node.id == self.id || p.productid == self.id
                    || p.materials
                        .values()
                        .any(|m| m.node.id == self.id || m.additional_node.as_ref().is_some_and(|f| f.id == self.id))
                {
                    p.recipe.clone()
                } else {
                    None
                }
            })
            .collect();
    }

    pub async fn set_icon<R: std::io::Read + std::io::Seek>(
        &mut self,
        res: &HashMap<SharedString, ItemRes>,
        zip: &mut zip::ZipArchive<R>,
        icon_cache: &mut HashMap<String, Arc<Image>>,
    ) -> Result<()> {
        if let Some(item_res) = res.get(&self.id) {
            let icon_key = item_res.icon.to_lowercase();
            if let Some(icon) = icon_cache.get(&icon_key) {
                self.icon = Some(icon.clone());
                return Ok(());
            }

            if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon.to_lowercase())) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match dds_to_jpeg(buf).await {
                    Ok(icon) => {
                        icon_cache.insert(icon_key, icon.clone());
                        self.icon = Some(icon);
                    }
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            } else if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon)) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match dds_to_jpeg(buf).await {
                    Ok(icon) => {
                        icon_cache.insert(icon_key, icon.clone());
                        self.icon = Some(icon);
                    }
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            }
        }

        Ok(())
    }

    pub async fn parse_id<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        self.id = SharedString::new(reader.read_string(format).await?.to_uppercase());
        Ok(())
    }

    pub async fn parse_usable_class<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        &mut self,
        reader: &mut R,
        format: DataFormat,
    ) -> Result<()> {
        let value = reader.read_string(format).await?;

        self.usable_class = value.split("_").filter_map(|c| GameClass::try_from(c).ok()).collect();
        Ok(())
    }

    pub async fn parse_effect<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        let effect = reader.read_string(format).await?;
        if effect != "*" && effect != "0" {
            self.effects.push(ItemEffect::new(effect));
        }
        Ok(())
    }

    pub async fn parse_binding<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        self.binding = Binding::try_from(reader.read_string(format).await?.as_str()).ok();
        Ok(())
    }

    pub async fn parse_no_trade<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.no_trade = reader.read_f32_le().await? != 0.0;
        Ok(())
    }

    pub async fn parse_no_sell<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.no_sell = reader.read_f32_le().await? != 0.0;
        Ok(())
    }

    pub async fn parse_no_destroy<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.no_destroy = reader.read_f32_le().await? != 0.0;
        Ok(())
    }

    pub async fn parse_grade<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.grade = Grade::from(reader.read_f32_le().await? as u8);
        Ok(())
    }

    pub async fn parse_item_level<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.item_level = reader.read_f32_le().await? as u16;
        Ok(())
    }

    pub async fn parse_required_level<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.required_level = reader.read_f32_le().await? as u8;
        Ok(())
    }
}
