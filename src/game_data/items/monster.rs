use std::{
    collections::{BTreeSet, HashMap}, io::{BufRead, Seek, SeekFrom},
};

use crate::{
    game_data::{
        DataFormat, ReadExt, TagType, common::Common, items::{ItemTrait, ReadableItem}, locale::Locale, skill::Skill,
    }, game_data_view::PreviewBuilder,
};
use anyhow::Result;
use indexmap::IndexMap;


use gpui_kit::SharedString;

#[derive(Default, Clone)]
pub struct Monster {
    pub debug: Vec<u8>,
    pub common: Common,
    pub skills: Vec<Skill>,
    pub skill: Option<SharedString>,
}

impl ReadableItem for Monster {
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

    fn read<R: BufRead + Seek + ?Sized>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        self.parse_debug(reader, offsets, item_idx, definitions, global_offset)?;
        let tag_count = definitions.len();
        for (tag_idx, (tag, tag_type)) in definitions.iter().enumerate() {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match Self::FORMAT {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset))?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2))?;
                }
            };

            self.common.parse(tag, tag_type, reader, Self::FORMAT)?;

            match tag.as_str() {
                "skill" => {
                    let value = reader.read_string(Self::FORMAT)?;
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

impl ItemTrait for Monster {
    fn common(&self) -> &Common {
        &self.common
    }
    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }
    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common()).skills(self.skills.clone())
    }
}

impl Monster {
    pub fn set_skills(
        &mut self,
        skills: &HashMap<SharedString, Skill>,
        skill_locales: &HashMap<SharedString, Locale>,
        unknown_skills: &mut BTreeSet<SharedString>,
    ) -> Result<()> {
        if let Some(skill) = self.skill.as_ref() {
            for name in skill.split(",").filter(|s| !s.is_empty()) {
                let clear_name = SharedString::new(name.split_once("_").map(|(a, _)| a).unwrap_or_else(|| name).trim().to_uppercase());
                if let Some(skill) = skills.get(&clear_name) {
                    let mut skill = skill.clone();

                    skill.description_locale = skill_locales
                        .get(&SharedString::new(name.to_uppercase().replace("_", "_DESCRIPTION_")))
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
