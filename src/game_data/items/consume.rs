use std::{
    collections::{BTreeSet, HashMap},
    io::SeekFrom,
};

use crate::{
    game_data::{
        AsyncBufReadExtReadString, DataFormat, TagType,
        common::Common,
        effects::EffectKind,
        items::{ItemTrait, ReadableItem},
        locale::Locale,
        skill::Skill,
    },
    game_data_view::PreviewBuilder,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncSeek, AsyncSeekExt};

use gpui_kit::SharedString;

#[derive(Default, Clone)]
pub struct Consume {
    pub skills: Vec<Skill>,
    pub debug: Vec<u8>,
    pub description_locale: Option<Locale>,
    pub common: Common,
    pub skill_effect: Option<SharedString>,
}

impl ReadableItem for Consume {
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
                "스킬효과" => {
                    let value = reader.read_string(Self::FORMAT).await?;
                    if value != "*" {
                        self.skill_effect = Some(value);
                    }
                }
                _ => {}
            }
        }

        Ok(self)
    }
}

impl Consume {
    pub fn set_description_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.description_locale = locales.get(&SharedString::new(format!("{}_DESCRIPTION", self.common.id))).cloned();
    }

    pub fn set_skills(
        &mut self,
        skills: &HashMap<SharedString, Skill>,
        skill_locales: &HashMap<SharedString, Locale>,
        unknown_skills: &mut BTreeSet<SharedString>,
    ) -> Result<()> {
        if let Some(skill) = self.skill_effect.as_ref() {
            for name in skill.split(",").filter(|s| !s.is_empty()) {
                let clear_name = SharedString::new(name.split_once(".").map(|(a, _)| a).unwrap_or_else(|| name).trim().to_uppercase());
                if let Some(skill) = skills.get(&clear_name) {
                    let mut skill = skill.clone();
                    if let Some(level) = name.split_once(".").and_then(|(_, a)| a.trim().parse::<u8>().ok()) {
                        skill.max_item_level = level;
                    }

                    skill.description_locale = skill_locales
                        .get(&SharedString::new(name.to_uppercase().replace(".", "_DESCRIPTION_")))
                        .cloned();
                    self.skills.push(skill);
                } else {
                    unknown_skills.insert(SharedString::new(clear_name));
                }
            }
        }
        Ok(())
    }
}

impl ItemTrait for Consume {
    fn common(&self) -> &Common {
        &self.common
    }
        fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }
    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn get_unique_effects(&self) -> Vec<EffectKind> {
        self.skills
            .iter()
            .flat_map(|f| f.skill_data.skill_level.iter())
            .filter_map(|f| f.buff1.effect_pattern_list.as_ref())
            .filter_map(|f| f.effect_pattern.as_ref())
            .flat_map(|f| f)
            .map(|p| EffectKind::Common {
                id: self.common.id.clone(),
                effect: p.effect.clone(),
            })
            .collect()
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common())
            .description_locale(self.description_locale.as_ref().and_then(|f| f.locale()))
            .skills(self.skills.clone())
    }
}
