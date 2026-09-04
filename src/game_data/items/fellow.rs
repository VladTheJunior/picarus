use std::{
    collections::{BTreeSet, HashMap},
    io::SeekFrom,
    rc::Rc,
};

use crate::{
    game_data::{
        AsyncBufReadExtReadString, DataFormat, TagType, common::Common, effects::EffectKind, items::{ItemTrait, ReadableItem}, locale::Locale, skill::Skill,
    }, game_data_view::PreviewBuilder,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui_kit::SharedString;

#[derive(Default, Clone)]
pub struct Fellow {
    pub skills: Vec<Skill>,
    pub debug: Vec<u8>,
    pub common: Common,
    pub region_id: Option<SharedString>,
    pub adventure_points: u16,
    pub temper_limit: u8,
    pub run_speed: f32,
    pub flight_run_speed: f32,
    pub can_fly: bool,
    pub max_level: u8,
    pub fellow_people: u8,
    pub skill: Option<SharedString>,
}

impl ReadableItem for Fellow {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.common.id.clone()
    }

    type CollectionItem = Self;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        item
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
        &mut self.debug
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        self.parse_debug(reader, offsets, item_idx, definitions, global_offset).await?;
        let tag_count = definitions.len();
        for (tag_idx, (tag, tag_type)) in definitions.iter().enumerate() {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match Self::FORMAT {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset)).await?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2)).await?;
                }
            };

            self.common.parse(tag, tag_type, reader, Self::FORMAT).await?;

            match tag.as_str() {
                "advpoint" => self.adventure_points = reader.read_f32_le().await? as u16,
                "regionid" => {
                    let value = reader.read_string(Self::FORMAT).await?.to_uppercase();
                    if value != "*" {
                        self.region_id = Some(SharedString::new(value));
                    }
                }
                "maxReinforce" => self.temper_limit = reader.read_f32_le().await? as u8,
                "뛰기속도" => self.run_speed = reader.read_f32_le().await?,
                "비행뛰기속도" => self.flight_run_speed = reader.read_f32_le().await?,
                "비행가능여부" => self.can_fly = reader.read_f32_le().await? != 0.0,
                "최대성장레벨" => self.max_level = reader.read_f32_le().await? as u8,
                "fellowpeople" => self.fellow_people = reader.read_f32_le().await? as u8,
                "skill" => {
                    let value = reader.read_string(Self::FORMAT).await?;
                    if value != "*" {
                        self.skill = Some(value);
                    }
                }
                _ => {}
            }
        }

        Ok(self)
    }
}

impl ItemTrait for Fellow {
    fn common(&self) -> &Common {
        &self.common
    }

    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common()).skills(self.skills.clone())
    }

    fn get_unique_effects(&self) -> Vec<EffectKind> {
        let mut effects = Vec::new();

        effects.extend(
            self.skills
                .iter()
                .flat_map(|f| f.skill_data.skill_level.iter())
                .filter_map(|f| f.buff1.effect_pattern_list.as_ref())
                .filter_map(|f| f.effect_pattern.as_ref())
                .flat_map(|f| f)
                .map(|p| EffectKind::Common {
                    id: self.common.id.clone(),
                    effect: p.effect.clone(),
                }),
        );

        effects
    }
}

impl Fellow {
    pub fn set_skills(&mut self, skills: &HashMap<SharedString, Skill>, skill_locales: &HashMap<SharedString, Locale>, unknown_skills: &mut BTreeSet<SharedString>) -> Result<()> {
        if let Some(skill) = self.skill.as_ref() {
            for name in skill.split(",") {
                let clear_name = SharedString::new(name.split_once("_").map(|(a, _)| a).unwrap_or_else(|| name).trim().to_uppercase());
                if let Some(skill) = skills.get(&clear_name) {
                    let mut skill = skill.clone();
                    skill.description_locale = skill_locales.get(&SharedString::new(name.to_uppercase().replace("_", "_DESCRIPTION_"))).cloned();
                    self.skills.push(skill);
                } else {
                    unknown_skills.insert(SharedString::new(clear_name));
                }
            }
        }
        Ok(())
    }
}
