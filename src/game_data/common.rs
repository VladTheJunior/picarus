use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
    io::Read,
    rc::Rc,
    sync::Arc,
};

use crate::game_data::{
    AsyncBufReadExtReadString, DataFormat,
    binding::Binding,
    dds_to_jpeg,
    effects::{EffectKind, ItemEffect},
    game_class::GameClass,
    grade::Grade,
    item_res::ItemRes,
    item_set::ItemSet,
    locale::Locale,
    product::Product,
};
use anyhow::Result;
use gpui::{Image, SharedString};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek};
use tracing::{error, warn};

#[derive(Default, Clone)]
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
    pub fn get_unique_effects(&self) -> Vec<EffectKind> {
        let mut effects = Vec::new();
        effects.extend(self.effects.iter().map(|effect| EffectKind::Common {
            id: self.id.clone(),
            effect: effect.clone(),
        }));

        if let Some(set) = &self.item_set {
            effects.extend(
                set.effects
                    .iter()
                    .flat_map(|f| f.seteffect_effects.iter())
                    .map(|effect| EffectKind::Common {
                        id: self.id.clone(),
                        effect: effect.clone(),
                    }),
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

    pub fn set_item_set(&mut self, item_set: &Vec<ItemSet>) {
        self.item_set = item_set.iter().find(|f| f.items.contains(&self.id)).cloned();
    }

    pub fn set_linked_recipes(&mut self, products: &Vec<Rc<RefCell<Product>>>) {
        self.linked_recipes = products
            .iter()
            .filter_map(|product| {
                let p = product.borrow();
                if p.node.id == self.id
                    || p.productid == self.id
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
        icons: &HashMap<String, String>,
        icon_cache: &mut HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        if let Some(item_res) = res.get(&self.id) {
            let icon_key = item_res.icon.to_lowercase();
            if let Some(icon) = icon_cache.get(&icon_key) {
                self.icon = Some(icon.clone());
                return Ok(());
            }

            if let Some(icon_path) = icons.get(&format!("libs/ui/resources/textures/slot_icons/{}.dds", icon_key)) {
                match zip.by_path(icon_path) {
                    Ok(mut file) => {
                        let mut buf = Vec::with_capacity(file.size() as usize);
                        file.read_to_end(&mut buf)?;

                        match dds_to_jpeg(buf).await {
                            Ok(icon) => {
                                icon_cache.insert(icon_key, icon.clone());
                                self.icon = Some(icon);
                            }
                            Err(e) => error!(?e, ?self.id, ?item_res.icon, "Failed to convert icon"),
                        }
                    }
                    Err(e) => {
                        error!(?e, ?self.id, ?item_res.icon, "Failed to load icon");
                    }
                }
            } else {
                unknown_icons
                    .entry(item_res.icon.clone())
                    .or_insert_with(BTreeSet::new)
                    .insert(self.id.clone());
            }
        } else {
            warn!(?self.id, "Failed to find icon");
        }

        Ok(())
    }

    async fn parse_id<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        self.id = SharedString::new(reader.read_string(format).await?.to_uppercase());
        Ok(())
    }

    async fn parse_usable_class<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        let value = reader.read_string(format).await?;

        self.usable_class = value.split("_").filter_map(|c| GameClass::try_from(c).ok()).collect();
        Ok(())
    }

    async fn parse_effect<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        let effect = reader.read_string(format).await?;
        if effect != "*" && effect != "0" {
            self.effects.push(ItemEffect::new(effect));
        }
        Ok(())
    }

    async fn parse_binding<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        self.binding = Binding::try_from(reader.read_string(format).await?.as_str()).ok();
        Ok(())
    }

    async fn parse_no_trade<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.no_trade = reader.read_f32_le().await? != 0.0;
        Ok(())
    }

    async fn parse_no_sell<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.no_sell = reader.read_f32_le().await? != 0.0;
        Ok(())
    }

    async fn parse_no_destroy<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.no_destroy = reader.read_f32_le().await? != 0.0;
        Ok(())
    }

    async fn parse_grade<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.grade = Grade::from(reader.read_f32_le().await? as u8);
        Ok(())
    }

    async fn parse_item_level<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.item_level = reader.read_f32_le().await? as u16;
        Ok(())
    }

    async fn parse_required_level<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R) -> Result<()> {
        self.required_level = reader.read_f32_le().await? as u8;
        Ok(())
    }

    pub async fn parse<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, key: &str, reader: &mut R, format: DataFormat) -> Result<()> {
        match key {
            "id" => self.parse_id(reader, format).await?,
            "등급" => self.parse_grade(reader).await?,
            "요구레벨" | "습득 필요 레벨" => self.parse_required_level(reader).await?,
            "아이템레벨" => self.parse_item_level(reader).await?,
            "파괴불능" => self.parse_no_destroy(reader).await?,
            "처분불능" => self.parse_no_sell(reader).await?,
            "거래불능" => self.parse_no_trade(reader).await?,
            "귀속" => self.parse_binding(reader, format).await?,
            "사용클래스" => self.parse_usable_class(reader, format).await?,
            "장착효과1" | "장착효과2" | "장착효과3" | "장착효과4" => self.parse_effect(reader, format).await?,
            _ => {}
        }

        Ok(())
    }
}
