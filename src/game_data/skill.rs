use std::{
    collections::{BTreeMap, BTreeSet, HashMap},

    sync::Arc,
};

use anyhow::Result;
use gpui_kit::{ Image, SharedString};
use serde::Deserialize;


use crate::game_data::{effects::ItemEffect, locale::Locale};

#[derive(Clone, Deserialize)]
pub struct Skill {
    #[serde(skip)]
    pub max_item_level: u8,
    #[serde(skip)]
    pub description_locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,
    #[serde(skip)]
    pub locale: Option<Locale>,
    #[serde(rename = "RECID")]
    pub recid: SharedString,
    #[serde(rename = "MAX_LEVEL")]
    pub max_level: u8,
    #[serde(rename = "PASSIVE_SKILL")]
    pub passive_skill: u8,
    #[serde(rename = "SKILL_DATA")]
    pub skill_data: SkillData,
    #[serde(rename = "CLT_ICON")]
    pub clt_icon: SharedString,
    #[serde(rename = "COOL_TIME")]
    pub cool_time: i32,
}

#[derive(Clone, Deserialize)]
pub struct SkillData {
    #[serde(rename = "SKILL_LEVEL")]
    pub skill_level: Vec<SkillLevel>,
}

#[derive(Clone, Deserialize)]
pub struct SkillLevel {
    #[serde(rename = "LEVEL")]
    pub level: u8,
    #[serde(rename = "LEARN_LEVEL")]
    pub learn_level: u8,
    #[serde(rename = "KEEP_BUFF_TIME")]
    pub keep_buff_time: i32,
    #[serde(rename = "buff1")]
    pub buff1: Buff,
}

#[derive(Clone, Deserialize)]
pub struct Buff {
    #[serde(rename = "VISIBLE")]
    pub visible: u8,
    #[serde(rename = "EFFECT_PATTERN_LIST")]
    pub effect_pattern_list: Option<EffectPatternList>,
}

#[derive(Clone, Deserialize)]
pub struct EffectPatternList {
    #[serde(rename = "EFFECT_PATTERN")]
    pub effect_pattern: Option<Vec<EffectPattern>>,
}

#[derive(Clone, Deserialize)]
pub struct EffectPattern {
    #[serde(skip)]
    pub effect: ItemEffect,
    #[serde(rename = "EFFECT_PATTERN_ENUM")]
    pub effect_pattern_enum: SharedString,
    #[serde(rename = "PARAM1")]
    pub param1: SharedString,
    #[serde(rename = "PARAM2")]
    pub param2: Option<SharedString>,
}

impl Skill {
    pub fn load(bytes: &[u8]) -> Result<Self> {
        let content = match std::str::from_utf8(bytes) {
            Ok(text) => text.replace(r#"encoding="EUC-KR""#, r#"encoding="UTF-8""#),
            Err(_) => {
                let (decoded, _, _) = encoding_rs::EUC_KR.decode(bytes);
                decoded.into_owned()
            }
        };

        let mut skill = quick_xml::de::from_str::<Skill>(content.as_ref())?;
        skill.recid = SharedString::new(skill.recid.to_uppercase());
        Ok(skill)
    }

    pub fn get_localized_name(&self) -> SharedString {
        self.locale.as_ref().and_then(|f| f.locale()).unwrap_or_else(|| self.recid.clone())
    }

    pub fn get_localized_description(&self) -> Option<SharedString> {
        self.description_locale.as_ref().and_then(|f| f.locale())
    }

    pub fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.locale = locales.get(&self.recid).cloned();
    }

    pub fn set_icon(
        &mut self,
        icons: &HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) {
        let icon_key = self.clt_icon.to_lowercase();
        if icon_key.is_empty() || icon_key == "0" {
            return;
        }
        if let Some(icon) = icons.get(&format!("libs/ui/resources/textures/slot_icons/{}.dds", icon_key)) {
            self.icon = Some(icon.clone());
            return;
        } else {
            unknown_icons
                .entry(self.clt_icon.clone())
                .or_insert_with(BTreeSet::new)
                .insert(self.recid.clone());
        }
    }

    pub fn set_effects(&mut self) {
        for s in self.skill_data.skill_level.iter_mut() {
            if let Some(patterns) = s.buff1.effect_pattern_list.as_mut().and_then(|f| f.effect_pattern.as_mut()) {
                let mut i = 0;
                while i < patterns.len() {
                    if patterns[i].effect_pattern_enum != "지속효과적용" {
                        patterns.remove(i);
                    } else {
                        patterns[i].effect = ItemEffect::from_effect_and_value(&patterns[i].param1, patterns[i].param2.as_ref());
                        i += 1;
                    }
                }
            }
        }
    }
}
