use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::BufRead,
    sync::Arc,
};

use crate::game_data::{
    DataFormat, ReadExt, TagType,
    binding::Binding,
    effects::{EffectKind, ItemEffect},
    evolution::Evolution,
    fellow_combination::FellowCombination,
    fishing::{Fishing, FishingDrop},
    game_class::GameClass,
    grade::Grade,
    item_res::ItemRes,
    item_set::ItemSet,
    locale::Locale,
    product::Product,
    synthesis_fellows::{SynthesisFellow, SynthesisFellows},
    synthesis_parts::SynthesisParts,
};
use anyhow::Result;
use gpui_kit::{Image, SharedString};
use parking_lot::Mutex;

#[derive(Default, Clone)]
pub struct Common {
    pub linked_recipes: BTreeSet<SharedString>,
    pub fishing: Vec<FishingDrop>,
    pub fellow_combinations: Vec<FellowCombination>,
    pub evolution: Vec<Evolution>,
    pub synthesis_parts: Vec<SynthesisParts>,
    pub synthesis_fellows: Vec<SynthesisFellow>,
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
    pub duration: Option<i32>,
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

        effects.extend(
            self.fellow_combinations
                .iter()
                .flat_map(|f| f.skills.iter())
                .flat_map(|f| f.skill_data.skill_level.iter())
                .filter_map(|f| f.buff1.effect_pattern_list.as_ref())
                .filter_map(|f| f.effect_pattern.as_ref())
                .flat_map(|f| f)
                .map(|p| EffectKind::Common {
                    id: self.id.clone(),
                    effect: p.effect.clone(),
                }),
        );

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

    pub fn set_fishing_drop(&mut self, fishing: &Vec<Fishing>) {
        self.fishing = fishing
            .iter()
            .filter_map(|f| {
                f.rewards.iter().find(|(_, r)| r.id == self.id).map(|(_, rew)| FishingDrop {
                    area_locale: f.area_locale.clone(),
                    map_locale: f.map_locale.clone(),
                    area: f.area.clone(),
                    map: f.map.clone(),
                    probability: rew.rate,
                    grade: f.groupno,
                })
            })
            .collect();
    }

    pub fn set_fellow_combinations(&mut self, fellow_combinations: &Vec<FellowCombination>) {
        self.fellow_combinations = fellow_combinations
            .iter()
            .filter(|f| {
                f.entries
                    .iter()
                    .find(|(_, r)| r.reqfellow.as_ref().is_some_and(|id| *id == self.id) || r.reqpet.as_ref().is_some_and(|id| *id == self.id))
                    .is_some()
            })
            .cloned()
            .collect();
    }

    pub fn set_evolution(&mut self, evolution: &Vec<Evolution>) {
        self.evolution = evolution.iter().filter(|f| f.resultfellow == self.id).cloned().collect();
    }

    pub fn set_synthesis_parts(&mut self, synthesis_parts: &Vec<SynthesisParts>) {
        self.synthesis_parts = synthesis_parts.iter().filter(|f| f.resultid == self.id).cloned().collect();
    }

    pub fn set_synthesis_fellows(&mut self, synthesis_fellows: &Vec<SynthesisFellows>) {
        self.synthesis_fellows = synthesis_fellows
            .iter()
            .filter_map(|f| f.fellows.iter().find_map(|(_, f)| (f.id == self.id).then(|| f)))
            .cloned()
            .collect();
    }

    pub fn set_linked_recipes(&mut self, products: &Vec<Arc<Mutex<Product>>>) {
        self.linked_recipes = products
            .iter()
            .filter_map(|product| {
                let p = product.lock();
                if p.id == self.id
                    || p.productid == self.id
                    || p.materials
                        .values()
                        .any(|m| m.id == self.id || m.additional_id.as_ref().is_some_and(|id| *id == self.id))
                {
                    p.recipe.clone()
                } else {
                    None
                }
            })
            .collect();
    }

    pub fn set_icon(
        &mut self,
        res: &HashMap<SharedString, ItemRes>,
        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) {
        if let Some(icon) = res.get(&self.id).and_then(|f| f.icon.as_ref()) {
            let icon_key = icon.to_lowercase();

            if icon_key.is_empty() {
                return;
            }
            if let Some(icon) = icons.get(&icon_key) {
                self.icon = Some(icon.clone());
                return;
            } else {
                unknown_icons.entry(icon.clone()).or_insert_with(BTreeSet::new).insert(self.id.clone());
            }
        }
    }

    fn parse_id<R: BufRead + ?Sized>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        self.id = SharedString::new(reader.read_string(format)?.to_uppercase());
        Ok(())
    }

    fn parse_usable_class<R: BufRead + ?Sized>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        let value = reader.read_string(format)?;

        self.usable_class = value.split("_").filter_map(|c| GameClass::try_from(c).ok()).collect();
        Ok(())
    }

    fn parse_effect<R: BufRead + ?Sized>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        let effect = reader.read_string(format)?;
        if effect != "*" && effect != "0" {
            self.effects.push(ItemEffect::new(effect));
        }
        Ok(())
    }

    fn parse_binding<R: BufRead + ?Sized>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        self.binding = Binding::try_from(reader.read_string(format)?.as_str()).ok();
        Ok(())
    }

    fn parse_no_trade<R: BufRead + ?Sized>(&mut self, reader: &mut R) -> Result<()> {
        self.no_trade = reader.read_f32_le()? != 0.0;
        Ok(())
    }

    fn parse_no_sell<R: BufRead + ?Sized>(&mut self, reader: &mut R) -> Result<()> {
        self.no_sell = reader.read_f32_le()? != 0.0;
        Ok(())
    }

    fn parse_no_destroy<R: BufRead + ?Sized>(&mut self, reader: &mut R) -> Result<()> {
        self.no_destroy = reader.read_f32_le()? != 0.0;
        Ok(())
    }

    fn parse_grade<R: BufRead + ?Sized>(&mut self, reader: &mut R, format: DataFormat, tag_type: &TagType) -> Result<()> {
        self.grade = match tag_type {
            TagType::String => Grade::from(reader.read_string(format)?),
            TagType::Float => Grade::from(reader.read_f32_le()? as u8),
        };
        Ok(())
    }

    fn parse_item_level<R: BufRead + ?Sized>(&mut self, reader: &mut R) -> Result<()> {
        self.item_level = reader.read_f32_le()? as u16;
        Ok(())
    }

    fn parse_required_level<R: BufRead + ?Sized>(&mut self, reader: &mut R) -> Result<()> {
        self.required_level = reader.read_f32_le()? as u8;
        Ok(())
    }

    fn parse_duration<R: BufRead + ?Sized>(&mut self, reader: &mut R) -> Result<()> {
        let duration = reader.read_f32_le()? as i32;
        if duration > 0 {
            self.duration = Some(duration);
        }
        Ok(())
    }

    pub fn parse<R: BufRead + ?Sized>(&mut self, tag: &str, tag_type: &TagType, reader: &mut R, format: DataFormat) -> Result<()> {
        match tag {
            "id" => self.parse_id(reader, format)?,
            "등급" => self.parse_grade(reader, format, tag_type)?,
            "요구레벨" | "습득 필요 레벨" => self.parse_required_level(reader)?,
            "아이템레벨" => self.parse_item_level(reader)?,
            "파괴불능" => self.parse_no_destroy(reader)?,
            "처분불능" => self.parse_no_sell(reader)?,
            "거래불능" => self.parse_no_trade(reader)?,
            "귀속" => self.parse_binding(reader, format)?,
            "사용클래스" => self.parse_usable_class(reader, format)?,
            "장착효과1" | "장착효과2" | "장착효과3" | "장착효과4" => self.parse_effect(reader, format)?,
            "사용기간" | "ownershipperiod" => self.parse_duration(reader)?,
            _ => {}
        }

        Ok(())
    }
}
