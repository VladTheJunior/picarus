use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::Read,
    sync::Arc,
};

use anyhow::Result;
use gpui_kit::{Image, SharedString};
use serde::{Deserialize};
use tracing::error;

use crate::game_data::{dds_to_jpeg, effects::ItemEffect, locale::Locale};

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
    #[serde(rename = "NAME")]
    pub name: SharedString,
    #[serde(rename = "ENABLED")]
    pub enabled: u8,
    #[serde(rename = "SKILL_TYPE")]
    pub skill_type: SharedString,
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
    #[serde(rename = "buff2")]
    pub buff2: Buff,
    #[serde(rename = "passive")]
    pub passive: Passive,
}

#[derive(Clone, Deserialize)]
pub struct Buff {
    #[serde(rename = "VISIBLE")]
    pub visible: u8,
    #[serde(rename = "APPLY_RATE")]
    pub apply_rate: f32,
    #[serde(rename = "BOARDER_APPLY_OPTION")]
    pub boarder_apply_option: SharedString,
    #[serde(rename = "EFFECT_PATTERN_LIST")]
    pub effect_pattern_list: Option<EffectPatternList>,
    #[serde(rename = "EFFECT_BUFFICON_ID")]
    pub effect_bufficon_id: SharedString,
    #[serde(rename = "SHOW_BUFFICON")]
    pub show_bufficon: u8,
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

    #[serde(rename = "EFFECT_PERCENT")]
    pub effect_percent: f32,
    #[serde(rename = "EFFECT_PATTERN_ENUM")]
    pub effect_pattern_enum: SharedString,
    #[serde(rename = "PARAM1")]
    pub param1: SharedString,
    #[serde(rename = "PARAM2")]
    pub param2: Option<SharedString>,
}

#[derive(Clone, Deserialize)]
pub struct Passive {
    #[serde(rename = "VISIBLE")]
    pub visible: u8,
    #[serde(rename = "APPLY_RATE")]
    pub apply_rate: f32,
    #[serde(rename = "EFFECT_MAIN_PERSON")]
    pub effect_main_person: SharedString,
}

impl Skill {
    pub async fn load<R: std::io::Read + std::io::Seek>(zip: &mut zip::ZipArchive<R>, path: &str) -> Result<Self> {
        let mut file = zip.by_path(path)?;
        let mut bytes = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut bytes)?;

        let content = if let Ok(text) = String::from_utf8(bytes.clone()) {
            text.replace(r#"encoding="EUC-KR""#, r#"encoding="UTF-8""#)
        } else {
            let (decoded, _, _) = encoding_rs::EUC_KR.decode(&bytes);
            decoded.into_owned()
        };

        let mut skill = tokio::task::spawn_blocking(move || quick_xml::de::from_str::<Skill>(content.as_ref())).await??;
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


    pub async fn set_icon<R: std::io::Read + std::io::Seek>(
        &mut self,
        zip: &mut zip::ZipArchive<R>,
        icons: &HashMap<String, String>,
        icon_cache: &mut HashMap<String, Arc<Image>>,
        unknown_icons: &mut BTreeMap<SharedString, BTreeSet<SharedString>>,
    ) -> Result<()> {
        let icon_key = self.clt_icon.to_lowercase();
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
                        Err(e) => error!(?e, ?self.recid, ?self.clt_icon, "Failed to convert icon"),
                    }
                }
                Err(e) => {
                    error!(?e, ?self.recid, ?self.clt_icon, "Failed to load icon");
                }
            }
        } else {
            unknown_icons
                .entry(self.clt_icon.clone())
                .or_insert_with(BTreeSet::new)
                .insert(self.recid.clone());
        }

        Ok(())
    }

    pub fn set_effects(&mut self) {
        for s in self.skill_data.skill_level.iter_mut() {
            if let Some(patterns) = s.buff1.effect_pattern_list.as_mut().and_then(|f| f.effect_pattern.as_mut()) {
                let mut i = 0;
                while i < patterns.len() {
                    if patterns[i].effect_pattern_enum != "지속효과적용"{
                        patterns.remove(i);
                    } else {
                        patterns[i].effect = ItemEffect::from_effect_and_value(&patterns[i].param1, patterns[i].param2.as_ref());
                        i += 1;
                    }
                }
            }
        }
    }

    /*
    pub fn parse<R: std::io::Read + std::io::Seek>(
        zip: &mut zip::ZipArchive<R>,
        skill: &str,
        skills: &HashMap<String, String>,
        skill_cache: &mut HashMap<String, Rc<Skill>>,
        unknown_skills: &mut BTreeSet<SharedString>,
    ) -> Result<Vec<Rc<Self>>> {
        let mut result = vec![];

        for name in skill.split(",") {
            let name = name.strip_suffix("_1").unwrap_or_else(|| name).to_lowercase();
            if let Some(skill) = skill_cache.get(&name) {
                result.push(skill.clone());
            }

            if let Some(skill_path) = skills.get(&format!("gamedata/adataxml/skill/{}.xml", name)) {
                match Self::load(zip.skill_path) {
                    Ok(mut file) => {
                        let mut buf = Vec::with_capacity(file.size() as usize);
                        file.read_to_end(&mut buf)?;

                        match quick_xml::de::from_reader(buf.as_ref()) {
                            Ok(skill) => {
                                let skill: Rc<Skill> = Rc::new(skill);
                                skill_cache.insert(name, skill.clone());
                                result.push(skill);
                            }
                            Err(e) => error!(?e, ?name, "Failed to deserialize skill"),
                        }
                    }
                    Err(e) => {
                        error!(?e, ?name, "Failed to load skill");
                    }
                }
            } else {
                unknown_skills.insert(SharedString::new(name));
            }
        }

        Ok(result)
    }*/
}
